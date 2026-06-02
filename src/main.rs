use anyhow::{Context, bail};
use glob::Pattern;
use inquire::{error::InquireResult, prompt_confirmation, prompt_text};
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;

use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{self, BufRead, BufReader, Read, Write},
    path::Path,
};
use walkdir::WalkDir;

use log::{error, info, warn};

mod error;
use error::ArchiveError;

mod cli;
use cli::Cli;

mod custom_log;
use custom_log::log_init;

const CHECKSUM_FILE: &str = ".checksums";

fn main() -> anyhow::Result<()> {
    // En cas d'erreur d'initialisation de la CLI, les erreurs ne seront pas loggées dans le fichier approprié. Ce qui n'est pas gênant, car aucune opération concernant les fichiers n'est effectuée.
    let cli = Cli::try_parse()?;
    log_init(&cli)?;

    // Initialisation de rayon
    rayon::ThreadPoolBuilder::new()
        .num_threads(cli.threads.unwrap_or(4usize))
        .build_global()?;

    match entry_point(cli) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Peut-être un peu tiré par les cheveux, mais permet de tracer même les erreurs d'exécution dans le fichier de log
            error!("{}", e);
            Err(e)
        }
    }
}

fn entry_point(cli: Cli) -> anyhow::Result<()> {
    let exclude = cli.exclude.unwrap_or_default();
    let exclude: Vec<&str> = exclude.split(",").filter(|s| !s.is_empty()).collect();

    match cli.command {
        cli::Commands::Prepare { path } => prepare(&path, &exclude),
        cli::Commands::Verify { path, interactive } => verify(&path, interactive, &exclude),
        cli::Commands::Copy {
            source,
            destination,
        } => copy_dir(&source, &destination, &exclude),
    }
}

fn prepare(path: &Path, exclude: &[&str]) -> anyhow::Result<()> {
    let checksum_path = path.join(CHECKSUM_FILE);

    if checksum_path.exists() {
        info!("Le fichier `.checksums` existe déjà, vérification...");
        verify(path, false, exclude)?;
        info!("Vérification OK.");
        return Ok(());
    }

    let checksums = compute_checksums(path, exclude).context("Erreur de calcul de la checksum")?;

    write_checksums(&checksum_path, &checksums)
        .context("Erreur lors de l'écriture du fichier `.checksums`")?;

    info!("Fichier `.checksums` généré.");
    Ok(())
}

fn verify(path: &Path, interactive: bool, exclude: &[&str]) -> anyhow::Result<()> {
    let checksum_path = path.join(CHECKSUM_FILE);

    if !checksum_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Le fichier `{}` n'existe pas", checksum_path.display()),
        )
        .into());
    }

    let expected: HashMap<String, String> = read_checksums(&checksum_path)
        .with_context(|| format!("Erreur lors de la lecture de `{}`", checksum_path.display()))?;

    let actual: HashMap<String, String> = compute_checksums(path, exclude).with_context(|| {
        format!(
            "Erreur lors du calcul des checksums du dossier `{}`",
            path.display()
        )
    })?;

    let mut modified = false;
    let mut aok = true;
    let mut updated = expected.clone();

    let expected_keys: HashSet<_> = expected.keys().cloned().collect();
    let actual_keys: HashSet<_> = actual.keys().cloned().collect();

    // fichiers manquants
    for missing in expected_keys.difference(&actual_keys) {
        aok = false;
        if interactive {
            warn!("Fichier manquant: `{}`", missing);
            if matches!(
                prompt_confirmation("Supprimer cette checksum ?"),
                InquireResult::Ok(true)
            ) {
                if let InquireResult::Ok(reason) =
                    prompt_text("Veuillez entrer un motif pour la disparition de ce fichier")
                {
                    updated.remove(missing);
                    modified = true;
                    info!(
                        "Prévu: suppression du fichier `{}` de l'archivage. Motif: {}",
                        missing, reason
                    );
                } else {
                    info!(
                        "Le fichier (manquant) `{}` ne sera pas supprimé de la liste des checksums.",
                        missing
                    );
                }
            } else {
                info!(
                    "Le fichier (manquant) `{}` ne sera pas supprimé de la liste des checksums.",
                    missing
                );
            }
        } else {
            return Err(ArchiveError::MissingFile(missing.into()).into());
        }
    }

    // nouveaux fichiers
    for extra in actual_keys.difference(&expected_keys) {
        aok = false;
        if interactive {
            warn!("Fichier non encore listé: {}", extra);
            if matches!(
                prompt_confirmation("Ajouter ce fichier aux checksums ?"),
                InquireResult::Ok(true)
            ) {
                if let InquireResult::Ok(reason) = prompt_text(
                    "Veuillez entrer un motif pour l'existence du fichier supplémentaire",
                ) {
                    updated.insert(extra.clone(), actual[extra].clone());
                    modified = true;
                    info!(
                        "Prévu: ajout du fichier `{}` à l'archivage. Motif: {}",
                        extra, reason
                    );
                } else {
                    info!(
                        "Le fichier (supplémentaire) `{}` ne sera pas ajouté à la liste des checksums.",
                        extra
                    );
                }
            } else {
                info!(
                    "Le fichier (supplémentaire) `{}` ne sera pas ajouté à la liste des checksums.",
                    extra
                );
            }
        } else {
            return Err(ArchiveError::UnexpectedFile(extra.into()).into());
        }
    }

    // checksum différentes
    for key in expected_keys.intersection(&actual_keys) {
        if expected[key] != actual[key] {
            aok = false;
            if interactive {
                warn!("Checksum différente pour le fichier `{}`", key);
                if matches!(
                    prompt_confirmation("Mettre à jour la checksum ?"),
                    InquireResult::Ok(true)
                ) {
                    if let InquireResult::Ok(reason) = prompt_text(
                        "Veuillez entrer un motif pour la différence entre les checksums",
                    ) {
                        updated.insert(key.clone(), actual[key].clone());
                        modified = true;
                        info!(
                            "Prévu: mise à jour de la checksum du fichier `{}`. Motif: {}",
                            key, reason
                        );
                    } else {
                        info!(
                            "La checksum de `{}` ne sera pas modifiée. L'erreur reviendra à la prochaine exécution si le fichier n'a pas été corrigé.",
                            key
                        );
                    }
                } else {
                    info!(
                        "La checksum de `{}` ne sera pas modifiée. L'erreur reviendra à la prochaine exécution si le fichier n'a pas été corrigé.",
                        key
                    );
                }
            } else {
                return Err(ArchiveError::ChecksumMismatch {
                    file_name: key.into(),
                    expected_sum: expected[key].to_string(),
                    actual_sum: actual[key].to_string(),
                }
                .into());
            }
        }
    }

    if modified {
        info!(
            "Le fichier des checksums va être modifié pour refléter les changements détectés. Application des changements..."
        );
        write_checksums(&checksum_path, &updated).with_context(|| {
            format!(
                "Impossible d'écrire les checksums dans {}",
                checksum_path.display()
            )
        })?;
        info!("Le fichier des checksums a bien été modifié.");
    }
    if aok {
        info!("Vérification terminée, tout est conforme.");
    } else {
        info!("Des erreurs ont été détectées, voir les messages précédents.")
    }
    Ok(())
}

fn copy_dir(source: &Path, destination: &Path, exclude: &[&str]) -> anyhow::Result<()> {
    verify(source, false, exclude)?;
    if destination.exists() && !destination.is_dir() {
        bail!(
            "Le chemin `{}` existe et n'est pas un dossier",
            destination.display()
        );
    }

    let target =
        if destination.exists() {
            destination.join(source.file_name().ok_or_else(|| {
                anyhow::anyhow!("Impossible de déterminer le nom du dossier source")
            })?)
        } else {
            destination.to_path_buf()
        };

    let checksums: HashMap<String, String> =
        read_checksums(&source.join(CHECKSUM_FILE)).context("Erreur lecture checksums")?;

    for (rel_path, expected_md5) in checksums {
        let rel = rel_path.trim_start_matches("./");

        let src = source.join(rel);
        let dst = target.join(rel);
        if let Some(up_to_dst_parent) = dst.parent() {
            if !up_to_dst_parent.exists() {
                fs::create_dir_all(up_to_dst_parent)?;
            }
        }

        fs::copy(&src, &dst).with_context(|| {
            format!(
                "Erreur lors de la copie de `{}` à `{}`",
                src.display(),
                dst.display()
            )
        })?;

        let actual_md5 = md5_file(&dst).with_context(|| {
            format!(
                "Erreur de vérification de la checksum sur le fichier {}",
                dst.display()
            )
        })?;

        if actual_md5 != expected_md5 {
            return Err(ArchiveError::ChecksumMismatch {
                file_name: dst.display().to_string(),
                expected_sum: expected_md5,
                actual_sum: actual_md5,
            })
            .with_context(|| "Erreur après copie");
        }
    }

    fs::copy(source.join(CHECKSUM_FILE), target.join(CHECKSUM_FILE))
        .with_context(|| format!("Erreur lors de la copie de .checksums"))?;
    info!("Copie terminée. Vérification...");
    verify(&target, false, exclude)?;
    Ok(())
}

fn compute_checksums(path: &Path, exclude: &[&str]) -> anyhow::Result<HashMap<String, String>> {
    Ok(WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| should_include(e.path(), exclude))
        .filter_map(|entry| entry.ok())
        .filter(|e| !e.file_type().is_symlink() && e.file_type().is_file())
        .filter(|e| e.path().file_name().unwrap_or_default() != CHECKSUM_FILE)
        .par_bridge()
        .filter_map(|entry| {
            let p = entry.path();

            let rel = p.strip_prefix(path).ok()?;

            let rel_str = format!("./{}", rel.to_string_lossy());

            let digest = md5_file(p).ok()?;
            Some((rel_str, digest))
        })
        .collect())
}

fn should_include(path: &Path, exclude: &[&str]) -> bool {
    !exclude
        .iter()
        .filter_map(|e| Pattern::new(e).ok())
        .any(|p| p.matches_path(path))
        && !path
            .components()
            .any(|c| c.as_os_str().to_string_lossy().starts_with("."))
}

fn md5_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut context = md5::Context::new();

    let mut buffer = [0u8; 8192];

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        context.consume(&buffer[..n]);
    }

    Ok(format!("{:x}", context.finalize()))
}

fn write_checksums(path: &Path, checksums: &HashMap<String, String>) -> io::Result<()> {
    let mut entries: Vec<_> = checksums.iter().collect();
    entries.sort_by_key(|(k, _)| *k);

    let mut file = File::create(path)?;

    for (path, checksum) in entries {
        writeln!(file, "{}\t{}", path, checksum)?;
    }

    Ok(())
}

fn read_checksums(path: &Path) -> io::Result<HashMap<String, String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut map = HashMap::new();

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<_> = line.split('\t').collect();

        if parts.len() != 2 {
            continue;
        }

        map.insert(parts[0].to_string(), parts[1].to_string());
    }

    Ok(map)
}
