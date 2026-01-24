//! CLI integration tests for demiarch
//!
//! Tests the demiarch CLI commands end-to-end using assert_cmd.

use assert_cmd::Command;
use predicates::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::TempDir;

/// Counter for generating unique project names across tests
static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a unique project name for testing
fn unique_project_name(base: &str) -> String {
    let count = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{}-{}-{}", base, count, timestamp)
}

/// Helper to create a command with license bypass enabled
#[allow(deprecated)]
fn demiarch_cmd() -> Command {
    let mut cmd = Command::cargo_bin("demiarch").unwrap();
    cmd.env("DEMIARCH_REQUIRE_LICENSE", "0");
    cmd.env("DEMIARCH_UNSAFE_ALLOW_UNLICENSED", "1");
    cmd
}

#[test]
fn test_new_command_creates_directory() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = unique_project_name("test-project");

    demiarch_cmd()
        .current_dir(&temp_dir)
        .args(["new", &project_name, "--framework", "rust"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Project created successfully"));

    // Verify directory structure
    let project_path = temp_dir.path().join(&project_name);
    assert!(project_path.exists(), "Project directory should exist");
    assert!(
        project_path.join(".git").exists(),
        "Git directory should exist"
    );
    assert!(
        project_path.join(".gitignore").exists(),
        "Gitignore should exist"
    );
    assert!(
        project_path.join("src").exists(),
        "src directory should exist"
    );
}

#[test]
fn test_new_command_creates_correct_gitignore_for_rust() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = unique_project_name("rust-project");

    demiarch_cmd()
        .current_dir(&temp_dir)
        .args(["new", &project_name, "--framework", "rust"])
        .assert()
        .success();

    let gitignore_content =
        std::fs::read_to_string(temp_dir.path().join(&project_name).join(".gitignore")).unwrap();
    assert!(
        gitignore_content.contains("/target"),
        "Rust gitignore should contain /target"
    );
    assert!(
        gitignore_content.contains(".env"),
        "Gitignore should contain .env"
    );
}

#[test]
fn test_new_command_creates_correct_gitignore_for_node() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = unique_project_name("node-project");

    demiarch_cmd()
        .current_dir(&temp_dir)
        .args(["new", &project_name, "--framework", "nextjs"])
        .assert()
        .success();

    let gitignore_content =
        std::fs::read_to_string(temp_dir.path().join(&project_name).join(".gitignore")).unwrap();
    assert!(
        gitignore_content.contains("node_modules/"),
        "Node gitignore should contain node_modules/"
    );
    assert!(
        gitignore_content.contains(".next/"),
        "Next.js gitignore should contain .next/"
    );
}

#[test]
fn test_new_command_creates_correct_gitignore_for_python() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = unique_project_name("python-project");

    demiarch_cmd()
        .current_dir(&temp_dir)
        .args(["new", &project_name, "--framework", "python"])
        .assert()
        .success();

    let gitignore_content =
        std::fs::read_to_string(temp_dir.path().join(&project_name).join(".gitignore")).unwrap();
    assert!(
        gitignore_content.contains("__pycache__/"),
        "Python gitignore should contain __pycache__/"
    );
    assert!(
        gitignore_content.contains("venv/"),
        "Python gitignore should contain venv/"
    );
}

#[test]
fn test_new_command_fails_if_directory_exists() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = unique_project_name("existing");

    // Create the directory first
    std::fs::create_dir(temp_dir.path().join(&project_name)).unwrap();

    demiarch_cmd()
        .current_dir(&temp_dir)
        .args(["new", &project_name, "--framework", "rust"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn test_new_command_shows_project_id() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = unique_project_name("id-test-project");

    demiarch_cmd()
        .current_dir(&temp_dir)
        .args(["new", &project_name, "--framework", "rust"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ID:"));
}

#[test]
fn test_new_command_shows_next_steps() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = unique_project_name("steps-project");

    demiarch_cmd()
        .current_dir(&temp_dir)
        .args(["new", &project_name, "--framework", "rust"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Next steps:"))
        .stdout(predicate::str::contains("demiarch chat"));
}

#[test]
fn test_new_command_quiet_mode() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = unique_project_name("quiet-project");

    demiarch_cmd()
        .current_dir(&temp_dir)
        .args(["--quiet", "new", &project_name, "--framework", "rust"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not()); // Should still output warning about unsafe mode
}

#[test]
fn test_projects_list_command() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = unique_project_name("list-test-project");

    // First create a project
    demiarch_cmd()
        .current_dir(&temp_dir)
        .args(["new", &project_name, "--framework", "rust"])
        .assert()
        .success();

    // Then list projects (this won't show the project due to different DB instances,
    // but should not fail)
    demiarch_cmd()
        .current_dir(&temp_dir)
        .args(["projects", "list"])
        .assert()
        .success();
}

#[test]
fn test_doctor_command() {
    demiarch_cmd().args(["doctor"]).assert().success();
}

#[test]
fn test_help_command() {
    demiarch_cmd()
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Local-first AI app builder"));
}

#[test]
fn test_version_output() {
    demiarch_cmd()
        .args(["--version"])
        .assert()
        .success()
        .stdout(predicate::str::contains("demiarch"));
}

#[test]
fn test_new_command_with_repo_url() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = unique_project_name("repo-test-project");

    demiarch_cmd()
        .current_dir(&temp_dir)
        .args([
            "new",
            &project_name,
            "--framework",
            "rust",
            "--repo",
            "https://github.com/test/test",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Repo:"));
}

#[test]
fn test_new_command_with_custom_path() {
    let temp_dir = TempDir::new().unwrap();
    let custom_location = TempDir::new().unwrap();
    let project_name = unique_project_name("path-test-project");

    demiarch_cmd()
        .current_dir(&temp_dir)
        .args([
            "new",
            &project_name,
            "--framework",
            "rust",
            "--path",
            custom_location.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "[ok] Project created successfully!",
        ));

    // Verify project was created in custom location
    let project_path = custom_location.path().join(&project_name);
    assert!(
        project_path.exists(),
        "Project directory should exist at custom path"
    );
    assert!(
        project_path.join(".git").exists(),
        "Git should be initialized"
    );
    assert!(
        project_path.join("src").exists(),
        "src directory should exist"
    );
}

#[test]
fn test_init_command_in_existing_directory() {
    let temp_dir = TempDir::new().unwrap();

    // Create a file to simulate an existing project
    std::fs::write(temp_dir.path().join("existing_file.txt"), "test").unwrap();

    demiarch_cmd()
        .current_dir(&temp_dir)
        .args(["init", "--framework", "rust"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "[ok] Project initialized successfully!",
        ))
        .stdout(predicate::str::contains("Framework: rust"));

    // .gitignore should be created
    assert!(temp_dir.path().join(".gitignore").exists());
}

#[test]
fn test_init_command_preserves_existing_gitignore() {
    let temp_dir = TempDir::new().unwrap();

    // Create a custom .gitignore
    let custom_gitignore = "# My custom gitignore\n*.custom\n";
    std::fs::write(temp_dir.path().join(".gitignore"), custom_gitignore).unwrap();

    demiarch_cmd()
        .current_dir(&temp_dir)
        .args(["init", "--framework", "rust"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Existing .gitignore preserved"));

    // Verify .gitignore was not overwritten
    let content = std::fs::read_to_string(temp_dir.path().join(".gitignore")).unwrap();
    assert_eq!(content, custom_gitignore);
}

#[test]
fn test_init_command_warns_on_empty_directory() {
    let temp_dir = TempDir::new().unwrap();

    demiarch_cmd()
        .current_dir(&temp_dir)
        .args(["init", "--framework", "python"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Directory is empty"));
}
