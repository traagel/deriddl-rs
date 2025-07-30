use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::{tempdir, TempDir};
use std::fs;
use std::path::Path;
use serial_test::serial;

/// Helper to create a temporary directory with sample migration files
fn setup_test_migrations() -> TempDir {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    
    // Create migrations directory
    let migrations_dir = temp_dir.path().join("migrations");
    fs::create_dir(&migrations_dir).expect("Failed to create migrations directory");
    
    // Create sample migration files
    fs::write(
        migrations_dir.join("0001_init_schema.sql"),
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);"
    ).expect("Failed to write migration file");
    
    fs::write(
        migrations_dir.join("0002_add_email.sql"),
        "ALTER TABLE users ADD COLUMN email TEXT;"
    ).expect("Failed to write migration file");
    
    fs::write(
        migrations_dir.join("0003_create_posts.sql"),
        "CREATE TABLE posts (id INTEGER PRIMARY KEY, user_id INTEGER, title TEXT, content TEXT);"
    ).expect("Failed to write migration file");
    
    temp_dir
}

/// Helper to get the binary under test
fn deri_ddl_cmd() -> Command {
    Command::cargo_bin("deriDDL").expect("Failed to find binary")
}

#[test]
fn test_help_command() {
    deri_ddl_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Rust-based ODBC schema migration runner"))
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("apply"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("plan"))
        .stdout(predicate::str::contains("health"))
        .stdout(predicate::str::contains("config"));
}

#[test]
fn test_version_command() {
    deri_ddl_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("deriDDL"));
}

#[test]
fn test_invalid_subcommand() {
    deri_ddl_cmd()
        .arg("invalid-command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_missing_subcommand() {
    deri_ddl_cmd()
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
#[serial]
fn test_config_generation() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let config_path = temp_dir.path().join("test-config.toml");
    
    deri_ddl_cmd()
        .arg("config")
        .arg("--output")
        .arg(&config_path)
        .current_dir(&temp_dir)
        .assert()
        .success();
    
    // Verify config file was created
    assert!(config_path.exists());
    
    // Verify config content
    let config_content = fs::read_to_string(&config_path).expect("Failed to read config file");
    assert!(config_content.contains("[database]"));
    assert!(config_content.contains("[migrations]"));
    assert!(config_content.contains("[logging]"));
    assert!(config_content.contains("[behavior]"));
    assert!(config_content.contains("[validation]"));
}

#[test]
#[serial]
fn test_config_generation_with_env() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    
    deri_ddl_cmd()
        .arg("config")
        .arg("--env")
        .arg("test")
        .current_dir(&temp_dir)
        .assert()
        .success();
    
    // Verify both base config and env config were created
    assert!(temp_dir.path().join("config.toml").exists());
    assert!(temp_dir.path().join("config/test.toml").exists());
}

#[test]
fn test_health_command_default() {
    let temp_dir = setup_test_migrations();
    
    deri_ddl_cmd()
        .arg("health")
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .current_dir(&temp_dir)
        .assert()
        .success();
}

#[test]
fn test_health_command_custom_dialect() {
    let temp_dir = setup_test_migrations();
    
    deri_ddl_cmd()
        .arg("health")
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .arg("--dialect")
        .arg("mysql")
        .current_dir(&temp_dir)
        .assert()
        .success();
}

#[test]
fn test_health_command_nonexistent_path() {
    deri_ddl_cmd()
        .arg("health")
        .arg("--path")
        .arg("/nonexistent/path")
        .assert()
        .failure();
}

#[test]
fn test_status_command_no_connection() {
    let temp_dir = setup_test_migrations();
    
    deri_ddl_cmd()
        .arg("status")
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .current_dir(&temp_dir)
        .assert()
        .failure()
        .stderr(predicate::str::contains("No connection string provided"));
}

#[test]
fn test_status_command_invalid_connection() {
    let temp_dir = setup_test_migrations();
    
    deri_ddl_cmd()
        .arg("status")
        .arg("--conn")
        .arg("invalid-connection-string")
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .current_dir(&temp_dir)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Connection failed"));
}

#[test]
fn test_apply_command_no_connection() {
    let temp_dir = setup_test_migrations();
    
    deri_ddl_cmd()
        .arg("apply")
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .current_dir(&temp_dir)
        .assert()
        .failure()
        .stderr(predicate::str::contains("No connection string provided"));
}

#[test]
fn test_apply_command_dry_run_no_connection() {
    let temp_dir = setup_test_migrations();
    
    deri_ddl_cmd()
        .arg("apply")
        .arg("--dry-run")
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .current_dir(&temp_dir)
        .assert()
        .failure()
        .stderr(predicate::str::contains("No connection string provided"));
}

#[test]
fn test_plan_command_no_connection() {
    let temp_dir = setup_test_migrations();
    
    deri_ddl_cmd()
        .arg("plan")
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .current_dir(&temp_dir)
        .assert()
        .failure()
        .stderr(predicate::str::contains("No connection string provided"));
}

#[test]
fn test_init_command_no_connection() {
    deri_ddl_cmd()
        .arg("init")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No connection string provided"));
}

#[test]
fn test_global_config_flag() {
    let temp_dir = setup_test_migrations();
    let config_path = temp_dir.path().join("custom-config.toml");
    
    // First create a config file
    deri_ddl_cmd()
        .arg("config")
        .arg("--output")
        .arg(&config_path)
        .current_dir(&temp_dir)
        .assert()
        .success();
    
    // Then try to use it with health command
    deri_ddl_cmd()
        .arg("--config")
        .arg(&config_path)
        .arg("health")
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .current_dir(&temp_dir)
        .assert()
        .success();
}

#[test]
fn test_global_env_flag() {
    let temp_dir = setup_test_migrations();
    
    // Create environment config first
    deri_ddl_cmd()
        .arg("config")
        .arg("--env")
        .arg("test")
        .current_dir(&temp_dir)
        .assert()
        .success();
    
    // Then use it with health command
    deri_ddl_cmd()
        .arg("--env")
        .arg("test")
        .arg("health")
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .current_dir(&temp_dir)
        .assert()
        .success();
}

#[test]
fn test_invalid_migration_path() {
    deri_ddl_cmd()
        .arg("health")
        .arg("--path")
        .arg("/completely/invalid/path/that/does/not/exist")
        .assert()
        .failure();
}

#[test]
fn test_empty_migrations_directory() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let empty_migrations_dir = temp_dir.path().join("empty_migrations");
    fs::create_dir(&empty_migrations_dir).expect("Failed to create empty migrations directory");
    
    deri_ddl_cmd()
        .arg("health")
        .arg("--path")
        .arg(&empty_migrations_dir)
        .current_dir(&temp_dir)
        .assert()
        .success(); // Health should still pass with empty directory
}

#[test]
fn test_subcommand_help() {
    let subcommands = ["apply", "status", "init", "plan", "health", "config"];
    
    for subcommand in &subcommands {
        deri_ddl_cmd()
            .arg(subcommand)
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Usage:"));
    }
}

#[test]
#[serial]
fn test_config_overwrite_behavior() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let config_path = temp_dir.path().join("overwrite-test.toml");
    
    // Create initial config
    deri_ddl_cmd()
        .arg("config")
        .arg("--output")
        .arg(&config_path)
        .current_dir(&temp_dir)
        .assert()
        .success();
    
    let initial_content = fs::read_to_string(&config_path).expect("Failed to read initial config");
    
    // Create config again (should overwrite)
    deri_ddl_cmd()
        .arg("config")
        .arg("--output")
        .arg(&config_path)
        .current_dir(&temp_dir)
        .assert()
        .success();
    
    let new_content = fs::read_to_string(&config_path).expect("Failed to read new config");
    
    // Content should be the same (it's deterministic)
    assert_eq!(initial_content, new_content);
}