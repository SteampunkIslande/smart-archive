use crate::ArchiveError;
use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    /// log_file n'est pas un argument de la ligne de commande, il est déterminé à partir des chemins spécifiés et du type de commande
    #[clap(skip)]
    pub log_file: PathBuf,

    #[arg(short, long)]
    pub quiet: bool,

    /// Liste de chemins à exclure (sous la forme d'expressions UNIX glob, séparés par des virgules)
    ///
    /// Attention, tous les chemins spécifiés dans la ligne de commande sont résolus de façon absolue, donc pour le pattern utilisé, attention
    #[arg(short, long)]
    pub exclude: String,

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

fn is_dir_empty<P>(path: P) -> anyhow::Result<bool>
where
    P: AsRef<Path>,
{
    Ok(std::fs::read_dir(path.as_ref())
        .with_context(|| format!("Impossible de lire le dossier {}", path.as_ref().display()))?
        .collect::<Vec<_>>()
        .is_empty())
}

impl Cli {
    pub fn try_parse() -> anyhow::Result<Self> {
        let mut cli = <Self as clap::Parser>::parse();

        match &mut cli.command {
            self::Commands::Copy {
                source,
                destination,
            } => {
                *source = source.canonicalize()?;
                if !source.is_dir() {
                    return Err(ArchiveError::InvalidSourceDir(source.display().to_string()).into());
                }
                // Le dossier de destination existe et n'est pas vide: on ne veut pas écrire les éléments de source dans destination. Donc on change destination en y ajoutant un niveau d'arborescence.
                if destination.exists() {
                    *destination = destination.canonicalize()?;
                    if !is_dir_empty(&destination)? {
                        *destination = destination.join(source.file_name().ok_or_else(|| {
                            ArchiveError::InvalidSourceDir(source.display().to_string())
                        })?);
                    }
                } else {
                    std::fs::create_dir_all(&destination).with_context(|| {
                        format!(
                            "Impossible de créer le dossier de destination `{}`",
                            destination.display()
                        )
                    })?;
                }
            }
            self::Commands::Prepare { path } => {
                *path = path.canonicalize()?;
                if !path.is_dir() {
                    return Err(ArchiveError::InvalidSourceDir(path.display().to_string()).into());
                }
            }
            self::Commands::Verify { path, .. } => {
                *path = path.canonicalize()?;
                if !path.is_dir() {
                    return Err(ArchiveError::InvalidSourceDir(path.display().to_string()).into());
                }
            }
        }

        cli.log_file = {
            match &cli.command {
                self::Commands::Copy {
                    source: _,
                    destination,
                } => destination,
                self::Commands::Prepare { path, .. } => path,
                self::Commands::Verify { path, .. } => path,
            }
            .join(".archivage-historique.log")
        };

        Ok(cli)
    }
}
