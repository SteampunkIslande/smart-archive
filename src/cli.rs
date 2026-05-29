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
