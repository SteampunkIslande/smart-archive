use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

// ============================================================
// Helpers
// ============================================================

fn setup_temp_dir() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("file1.txt"), "contenu de file1").unwrap();
    fs::write(dir.path().join("file2.txt"), "contenu de file2").unwrap();
    fs::write(dir.path().join("file3.txt"), "contenu de file3").unwrap();
    dir
}

fn run_prepare(dir: &TempDir) {
    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("prepare").arg(dir.path());
    cmd.assert().success();
}

fn read_log(dir: &TempDir) -> String {
    let log_path = dir.path().join(".archivage-historique.log");
    fs::read_to_string(&log_path).unwrap_or_default()
}

fn log_contains(dir: &TempDir, message: &str) -> bool {
    read_log(dir).contains(message)
}

fn log_lines_without_timestamps(dir: &TempDir) -> Vec<String> {
    read_log(dir)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            if let Some(pos) = line.find("] ") {
                line[pos + 2..].to_string()
            } else if let Some(pos) = line.find(" INFO ") {
                line[pos..].to_string()
            } else if let Some(pos) = line.find(" WARN ") {
                line[pos..].to_string()
            } else if let Some(pos) = line.find(" ERROR ") {
                line[pos..].to_string()
            } else {
                line.to_string()
            }
        })
        .collect()
}

fn stdin_for_confirm_and_reason(confirm: bool, reason: &str) -> String {
    if confirm {
        format!("y\n{}\n", reason)
    } else {
        "n\n".to_string()
    }
}

fn stdin_for_multiple_confirm_and_reason(
    count: usize,
    confirms: &[bool],
    reasons: &[&str],
) -> String {
    let mut input = String::new();
    for i in 0..count {
        if confirms[i] {
            input.push_str("y\n");
            input.push_str(reasons[i]);
            input.push('\n');
        } else {
            input.push_str("n\n");
        }
    }
    input
}

// ============================================================
// Test: prepare dans un dossier temporaire avec des fichiers bidon
// ============================================================

#[test]
fn test_prepare_creates_checksum_file() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    assert!(
        dir.path().join(".checksums").exists(),
        "Le fichier .checksums aurait dû être créé"
    );

    let checksum_content = fs::read_to_string(dir.path().join(".checksums")).unwrap();
    assert!(
        checksum_content.contains("./file1.txt"),
        "Le checksum devrait contenir file1.txt"
    );
    assert!(
        checksum_content.contains("./file2.txt"),
        "Le checksum devrait contenir file2.txt"
    );
    assert!(
        checksum_content.contains("./file3.txt"),
        "Le checksum devrait contenir file3.txt"
    );
}

#[test]
fn test_prepare_logs_success_message() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    assert!(
        log_contains(&dir, "Fichier `.checksums` généré."),
        "Le log devrait contenir le message de succès. Log: {}",
        read_log(&dir)
    );
}

#[test]
fn test_prepare_existing_checksum_runs_verify() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    // Run prepare again - should detect existing .checksums and verify
    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("prepare").arg(dir.path());
    cmd.assert().success();

    assert!(
        log_contains(&dir, "Le fichier `.checksums` existe déjà, vérification"),
        "Le log devrait mentionner que le fichier .checksums existe déjà. Log: {}",
        read_log(&dir)
    );
    assert!(
        log_contains(&dir, "Vérification terminée, tout est conforme"),
        "Le log devrait indiquer que la vérification est conforme. Log: {}",
        read_log(&dir)
    );
}

// ============================================================
// Test: suppression d'un fichier (mode interactif)
// ============================================================

#[test]
fn test_verify_delete_one_file_interactive_accept() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    fs::remove_file(dir.path().join("file1.txt")).unwrap();

    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("verify").arg(dir.path()).arg("--interactive");
    cmd.write_stdin(stdin_for_confirm_and_reason(
        true,
        "fichier supprimé intentionnellement",
    ));
    cmd.assert().success();

    let log_lines = log_lines_without_timestamps(&dir);
    let log_joined = log_lines.join("\n");

    assert!(
        log_joined.contains("Fichier manquant"),
        "Le log devrait mentionner un fichier manquant. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("Prévu: suppression du fichier"),
        "Le log devrait mentionner la suppression prévue. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("fichier supprimé intentionnellement"),
        "Le log devrait contenir le motif fourni. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("Le fichier des checksums a bien été modifié"),
        "Le log devrait confirmer la modification du fichier checksums. Log: {}",
        log_joined
    );
}

#[test]
fn test_verify_delete_one_file_interactive_refuse() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    fs::remove_file(dir.path().join("file1.txt")).unwrap();

    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("verify").arg(dir.path()).arg("--interactive");
    cmd.write_stdin(stdin_for_confirm_and_reason(false, ""));
    cmd.assert().success();

    let log_lines = log_lines_without_timestamps(&dir);
    let log_joined = log_lines.join("\n");

    assert!(
        log_joined.contains("Fichier manquant"),
        "Le log devrait mentionner un fichier manquant. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("ne sera pas supprimé de la liste des checksums"),
        "Le log devrait indiquer que la checksum ne sera pas supprimée. Log: {}",
        log_joined
    );
}

// ============================================================
// Test: suppression de deux fichiers (mode interactif)
// ============================================================

#[test]
fn test_verify_delete_two_files_interactive_accept_both() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    fs::remove_file(dir.path().join("file1.txt")).unwrap();
    fs::remove_file(dir.path().join("file2.txt")).unwrap();

    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("verify").arg(dir.path()).arg("--interactive");
    cmd.write_stdin(stdin_for_multiple_confirm_and_reason(
        2,
        &[true, true],
        &["suppression fichier 1", "suppression fichier 2"],
    ));
    cmd.assert().success();

    let log_lines = log_lines_without_timestamps(&dir);
    let log_joined = log_lines.join("\n");

    assert!(
        log_joined.contains("Fichier manquant"),
        "Le log devrait mentionner des fichiers manquants. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("Prévu: suppression du fichier"),
        "Le log devrait mentionner les suppressions prévues. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("suppression fichier 1"),
        "Le log devrait contenir le premier motif. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("suppression fichier 2"),
        "Le log devrait contenir le second motif. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("Le fichier des checksums a bien été modifié"),
        "Le log devrait confirmer la modification du fichier checksums. Log: {}",
        log_joined
    );
}

#[test]
fn test_verify_delete_two_files_interactive_accept_first_refuse_second() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    fs::remove_file(dir.path().join("file1.txt")).unwrap();
    fs::remove_file(dir.path().join("file2.txt")).unwrap();

    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("verify").arg(dir.path()).arg("--interactive");
    cmd.write_stdin(stdin_for_multiple_confirm_and_reason(
        2,
        &[true, false],
        &["suppression fichier 1", ""],
    ));
    cmd.assert().success();

    let log_lines = log_lines_without_timestamps(&dir);
    let log_joined = log_lines.join("\n");

    assert!(
        log_joined.contains("suppression fichier 1"),
        "Le log devrait contenir le motif du premier fichier. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("ne sera pas supprimé de la liste des checksums"),
        "Le log devrait indiquer que la seconde checksum ne sera pas supprimée. Log: {}",
        log_joined
    );
}

#[test]
fn test_verify_delete_two_files_interactive_refuse_both() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    fs::remove_file(dir.path().join("file1.txt")).unwrap();
    fs::remove_file(dir.path().join("file2.txt")).unwrap();

    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("verify").arg(dir.path()).arg("--interactive");
    cmd.write_stdin(stdin_for_multiple_confirm_and_reason(
        2,
        &[false, false],
        &["", ""],
    ));
    cmd.assert().success();

    let log_lines = log_lines_without_timestamps(&dir);
    let log_joined = log_lines.join("\n");

    assert!(
        log_joined.contains("Fichier manquant"),
        "Le log devrait mentionner des fichiers manquants. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("ne sera pas supprimé de la liste des checksums"),
        "Le log devrait indiquer que les checksums ne seront pas supprimées. Log: {}",
        log_joined
    );
}

// ============================================================
// Test: ajout d'un fichier inattendu (mode interactif)
// ============================================================

#[test]
fn test_verify_add_one_unexpected_file_interactive_accept() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    fs::write(dir.path().join("nouveau_fichier.txt"), "contenu inattendu").unwrap();

    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("verify").arg(dir.path()).arg("--interactive");
    cmd.write_stdin(stdin_for_confirm_and_reason(true, "ajout légitime"));
    cmd.assert().success();

    let log_lines = log_lines_without_timestamps(&dir);
    let log_joined = log_lines.join("\n");

    assert!(
        log_joined.contains("Fichier non listé"),
        "Le log devrait mentionner un fichier non listé. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("Prévu: ajout du fichier"),
        "Le log devrait mentionner l'ajout prévu. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("ajout légitime"),
        "Le log devrait contenir le motif fourni. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("Le fichier des checksums a bien été modifié"),
        "Le log devrait confirmer la modification du fichier checksums. Log: {}",
        log_joined
    );
}

#[test]
fn test_verify_add_one_unexpected_file_interactive_refuse() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    fs::write(dir.path().join("nouveau_fichier.txt"), "contenu inattendu").unwrap();

    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("verify").arg(dir.path()).arg("--interactive");
    cmd.write_stdin(stdin_for_confirm_and_reason(false, ""));
    cmd.assert().success();

    let log_lines = log_lines_without_timestamps(&dir);
    let log_joined = log_lines.join("\n");

    assert!(
        log_joined.contains("Fichier non listé"),
        "Le log devrait mentionner un fichier non listé. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("ne sera pas ajouté à la liste des checksums"),
        "Le log devrait indiquer que le fichier ne sera pas ajouté. Log: {}",
        log_joined
    );
}

// ============================================================
// Test: ajout de deux fichiers inattendus (mode interactif)
// ============================================================

#[test]
fn test_verify_add_two_unexpected_files_interactive_accept_both() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    fs::write(dir.path().join("extra1.txt"), "contenu extra 1").unwrap();
    fs::write(dir.path().join("extra2.txt"), "contenu extra 2").unwrap();

    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("verify").arg(dir.path()).arg("--interactive");
    cmd.write_stdin(stdin_for_multiple_confirm_and_reason(
        2,
        &[true, true],
        &["ajout extra 1", "ajout extra 2"],
    ));
    cmd.assert().success();

    let log_lines = log_lines_without_timestamps(&dir);
    let log_joined = log_lines.join("\n");

    assert!(
        log_joined.contains("Fichier non listé"),
        "Le log devrait mentionner des fichiers non listés. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("Prévu: ajout du fichier"),
        "Le log devrait mentionner les ajouts prévus. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("ajout extra 1"),
        "Le log devrait contenir le premier motif. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("ajout extra 2"),
        "Le log devrait contenir le second motif. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("Le fichier des checksums a bien été modifié"),
        "Le log devrait confirmer la modification du fichier checksums. Log: {}",
        log_joined
    );
}

#[test]
fn test_verify_add_two_unexpected_files_interactive_accept_first_refuse_second() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    fs::write(dir.path().join("extra1.txt"), "contenu extra 1").unwrap();
    fs::write(dir.path().join("extra2.txt"), "contenu extra 2").unwrap();

    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("verify").arg(dir.path()).arg("--interactive");
    cmd.write_stdin(stdin_for_multiple_confirm_and_reason(
        2,
        &[true, false],
        &["ajout extra 1", ""],
    ));
    cmd.assert().success();

    let log_lines = log_lines_without_timestamps(&dir);
    let log_joined = log_lines.join("\n");

    assert!(
        log_joined.contains("ajout extra 1"),
        "Le log devrait contenir le motif du premier fichier. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("ne sera pas ajouté à la liste des checksums"),
        "Le log devrait indiquer que le second fichier ne sera pas ajouté. Log: {}",
        log_joined
    );
}

#[test]
fn test_verify_add_two_unexpected_files_interactive_refuse_both() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    fs::write(dir.path().join("extra1.txt"), "contenu extra 1").unwrap();
    fs::write(dir.path().join("extra2.txt"), "contenu extra 2").unwrap();

    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("verify").arg(dir.path()).arg("--interactive");
    cmd.write_stdin(stdin_for_multiple_confirm_and_reason(
        2,
        &[false, false],
        &["", ""],
    ));
    cmd.assert().success();

    let log_lines = log_lines_without_timestamps(&dir);
    let log_joined = log_lines.join("\n");

    assert!(
        log_joined.contains("Fichier non listé"),
        "Le log devrait mentionner des fichiers non listés. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("ne sera pas ajouté à la liste des checksums"),
        "Le log devrait indiquer que les fichiers ne seront pas ajoutés. Log: {}",
        log_joined
    );
}

// ============================================================
// Test: altération d'un fichier (mode interactif)
// ============================================================

#[test]
fn test_verify_alter_one_file_interactive_accept() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    fs::write(dir.path().join("file1.txt"), "contenu modifié de file1").unwrap();

    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("verify").arg(dir.path()).arg("--interactive");
    cmd.write_stdin(stdin_for_confirm_and_reason(
        true,
        "fichier modifié intentionnellement",
    ));
    cmd.assert().success();

    let log_lines = log_lines_without_timestamps(&dir);
    let log_joined = log_lines.join("\n");

    assert!(
        log_joined.contains("Checksum différente"),
        "Le log devrait mentionner une différence de checksum. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("Prévu: mise à jour de la checksum"),
        "Le log devrait mentionner la mise à jour prévue de la checksum. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("fichier modifié intentionnellement"),
        "Le log devrait contenir le motif fourni. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("Le fichier des checksums a bien été modifié"),
        "Le log devrait confirmer la modification du fichier checksums. Log: {}",
        log_joined
    );
}

#[test]
fn test_verify_alter_one_file_interactive_refuse() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    fs::write(dir.path().join("file1.txt"), "contenu modifié de file1").unwrap();

    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("verify").arg(dir.path()).arg("--interactive");
    cmd.write_stdin(stdin_for_confirm_and_reason(false, ""));
    cmd.assert().success();

    let log_lines = log_lines_without_timestamps(&dir);
    let log_joined = log_lines.join("\n");

    assert!(
        log_joined.contains("Checksum différente"),
        "Le log devrait mentionner une différence de checksum. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("ne sera pas modifiée"),
        "Le log devrait indiquer que la checksum ne sera pas modifiée. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("L'erreur reviendra à la prochaine exécution"),
        "Le log devrait avertir que l'erreur reviendra. Log: {}",
        log_joined
    );
}

// ============================================================
// Test: altération de deux fichiers (mode interactif)
// ============================================================

#[test]
fn test_verify_alter_two_files_interactive_accept_both() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    fs::write(dir.path().join("file1.txt"), "contenu modifié file1").unwrap();
    fs::write(dir.path().join("file2.txt"), "contenu modifié file2").unwrap();

    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("verify").arg(dir.path()).arg("--interactive");
    cmd.write_stdin(stdin_for_multiple_confirm_and_reason(
        2,
        &[true, true],
        &["modification fichier 1", "modification fichier 2"],
    ));
    cmd.assert().success();

    let log_lines = log_lines_without_timestamps(&dir);
    let log_joined = log_lines.join("\n");

    assert!(
        log_joined.contains("Checksum différente"),
        "Le log devrait mentionner des différences de checksum. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("Prévu: mise à jour de la checksum"),
        "Le log devrait mentionner les mises à jour prévues. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("modification fichier 1"),
        "Le log devrait contenir le premier motif. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("modification fichier 2"),
        "Le log devrait contenir le second motif. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("Le fichier des checksums a bien été modifié"),
        "Le log devrait confirmer la modification du fichier checksums. Log: {}",
        log_joined
    );
}

#[test]
fn test_verify_alter_two_files_interactive_accept_first_refuse_second() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    fs::write(dir.path().join("file1.txt"), "contenu modifié file1").unwrap();
    fs::write(dir.path().join("file2.txt"), "contenu modifié file2").unwrap();

    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("verify").arg(dir.path()).arg("--interactive");
    cmd.write_stdin(stdin_for_multiple_confirm_and_reason(
        2,
        &[true, false],
        &["modification fichier 1", ""],
    ));
    cmd.assert().success();

    let log_lines = log_lines_without_timestamps(&dir);
    let log_joined = log_lines.join("\n");

    assert!(
        log_joined.contains("modification fichier 1"),
        "Le log devrait contenir le motif du premier fichier. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("ne sera pas modifiée"),
        "Le log devrait indiquer que la seconde checksum ne sera pas modifiée. Log: {}",
        log_joined
    );
}

#[test]
fn test_verify_alter_two_files_interactive_refuse_both() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    fs::write(dir.path().join("file1.txt"), "contenu modifié file1").unwrap();
    fs::write(dir.path().join("file2.txt"), "contenu modifié file2").unwrap();

    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("verify").arg(dir.path()).arg("--interactive");
    cmd.write_stdin(stdin_for_multiple_confirm_and_reason(
        2,
        &[false, false],
        &["", ""],
    ));
    cmd.assert().success();

    let log_lines = log_lines_without_timestamps(&dir);
    let log_joined = log_lines.join("\n");

    assert!(
        log_joined.contains("Checksum différente"),
        "Le log devrait mentionner des différences de checksum. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("ne sera pas modifiée"),
        "Le log devrait indiquer que les checksums ne seront pas modifiées. Log: {}",
        log_joined
    );
}

// ============================================================
// Test: non-interactive mode errors (for completeness)
// ============================================================

#[test]
fn test_verify_non_interactive_missing_file_error() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    fs::remove_file(dir.path().join("file1.txt")).unwrap();

    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("verify").arg(dir.path());
    cmd.assert().failure();

    assert!(
        log_contains(&dir, "Fichier manquant"),
        "Le log devrait mentionner un fichier manquant en mode non-interactif. Log: {}",
        read_log(&dir)
    );
}

#[test]
fn test_verify_non_interactive_unexpected_file_error() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    fs::write(dir.path().join("extra.txt"), "contenu inattendu").unwrap();

    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("verify").arg(dir.path());
    cmd.assert().failure();

    assert!(
        log_contains(&dir, "Fichier inattendu"),
        "Le log devrait mentionner un fichier inattendu en mode non-interactif. Log: {}",
        read_log(&dir)
    );
}

#[test]
fn test_verify_non_interactive_checksum_mismatch_error() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    fs::write(dir.path().join("file1.txt"), "contenu modifié").unwrap();

    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("verify").arg(dir.path());
    cmd.assert().failure();

    assert!(
        log_contains(&dir, "Checksums contradictoires"),
        "Le log devrait mentionner une contradiction de checksums en mode non-interactif. Log: {}",
        read_log(&dir)
    );
}

// ============================================================
// Test: verify without .checksums file
// ============================================================

#[test]
fn test_verify_without_checksum_file_error() {
    let dir = setup_temp_dir();

    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("verify").arg(dir.path());
    cmd.assert().failure();

    let log = read_log(&dir);
    assert!(
        log.contains(".checksums") && log.contains("n'existe pas"),
        "Le log devrait indiquer que le fichier .checksums n'existe pas. Log: {}",
        log
    );
}

// ============================================================
// Test: combined scenario (delete + add + alter in one verify)
// ============================================================

#[test]
fn test_verify_combined_scenario_interactive() {
    let dir = setup_temp_dir();
    run_prepare(&dir);

    fs::remove_file(dir.path().join("file1.txt")).unwrap();
    fs::write(dir.path().join("nouveau.txt"), "fichier ajouté").unwrap();
    fs::write(dir.path().join("file2.txt"), "contenu altéré").unwrap();

    let mut cmd = Command::cargo_bin("smart-archive").unwrap();
    cmd.arg("verify").arg(dir.path()).arg("--interactive");
    cmd.write_stdin(
        "y\nsuppression file1\n".to_string()
            + "y\najout nouveau fichier\n"
            + "y\naltération file2\n",
    );
    cmd.assert().success();

    let log_lines = log_lines_without_timestamps(&dir);
    let log_joined = log_lines.join("\n");

    assert!(
        log_joined.contains("Fichier manquant"),
        "Le log devrait mentionner un fichier manquant. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("Fichier non listé"),
        "Le log devrait mentionner un fichier non listé. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("Checksum différente"),
        "Le log devrait mentionner une différence de checksum. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("suppression file1"),
        "Le log devrait contenir le motif de suppression. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("ajout nouveau fichier"),
        "Le log devrait contenir le motif d'ajout. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("altération file2"),
        "Le log devrait contenir le motif d'altération. Log: {}",
        log_joined
    );
    assert!(
        log_joined.contains("Le fichier des checksums a bien été modifié"),
        "Le log devrait confirmer la modification du fichier checksums. Log: {}",
        log_joined
    );
}
