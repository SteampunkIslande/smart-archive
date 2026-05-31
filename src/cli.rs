use crate::ArchiveError;
use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[arg(long)]
    pub log_file: Option<PathBuf>,

    #[arg(short, long)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Prépare un dossier en générant le fichier .checksums
    Prepare { path: PathBuf },

    /// Vérifie l'intégrité d'un dossier
    Verify {
        path: PathBuf,

        #[arg(short, long)]
        interactive: bool,
    },

    /// Copie un dossier en vérifiant les checksums
    Copy {
        source: PathBuf,
        destination: PathBuf,
    },
}

impl Cli {
    pub fn try_parse() -> anyhow::Result<Self> {
        let mut cli = <Self as clap::Parser>::parse();

        let history_file = match &mut cli.command {
            self::Commands::Copy {
                source,
                destination,
            } => {
                // Le dossier de destination existe et n'est pas vide: on ne veut pas écrire les éléments de source dans destination. Donc on change destination en y ajoutant un niveau d'arborescence.
                if destination.exists()
                    && !std::fs::read_dir(&destination)
                        .with_context(|| {
                            format!("Impossible de lire le dossier {}", destination.display())
                        })?
                        .collect::<Vec<_>>()
                        .is_empty()
                {
                    *destination = destination.join(source.file_name().ok_or_else(|| {
                        ArchiveError::InvalidSourceDir(source.display().to_string())
                    })?);
                } else {
                    std::fs::create_dir_all(&destination).with_context(|| {
                        format!(
                            "Impossible de créer le dossier de destination `{}`",
                            destination.display()
                        )
                    })?;
                }
                destination
            }
            self::Commands::Prepare { path } => path,
            self::Commands::Verify { path, .. } => path,
        }
        .join("archivage-historique.log");

        cli.log_file = Some(history_file);

        Ok(cli)
    }
}
