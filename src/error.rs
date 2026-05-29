use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArchiveError {
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error("Fichier manquant: {0}")]
    MissingFile(String),
    #[error(
        "Checksums contradictoires pour le fichier `{file_name}`. Attendu: `{expected_sum}`. Réel: `{actual_sum}`"
    )]
    ChecksumMismatch {
        file_name: String,
        expected_sum: String,
        actual_sum: String,
    },
    #[error("Fichier inattendu: {0}")]
    UnexpectedFile(String),
    #[error("Le dossier `{0}` n'est pas un nom de dossier source valide.")]
    InvalidSourceDir(String),
}
