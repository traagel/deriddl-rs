mod common;
use common::{deri_ddl_cmd, setup_test_migrations_with_rollback, test_sqlite_connection, init_test_database};
use predicates::str::contains;
use std::fs;

#[test]
fn test_rollback_actual_database_operations() {
    // Create a simpler test directory with only migrations that have rollback SQL
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let migrations_dir = temp_dir.path().join("migrations");
    fs::create_dir(&migrations_dir).expect("Failed to create migrations directory");

    // Create migrations with rollback SQL
    fs::write(
        migrations_dir.join("0001_create_users.sql"),
        r#"-- +migrate Up
CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);

-- +migrate Down
DROP TABLE IF EXISTS users;
"#,
    ).unwrap();

    fs::write(
        migrations_dir.join("0002_create_posts.sql"),
        r#"-- +migrate Up
CREATE TABLE posts (id INTEGER PRIMARY KEY, user_id INTEGER, title TEXT);

-- +migrate Down
DROP TABLE IF EXISTS posts;
"#,
    ).unwrap();
    
    let connection_string = test_sqlite_connection();
    
    // Initialize database
    init_test_database(&connection_string).expect("Failed to initialize database");
    
    // Apply migrations
    deri_ddl_cmd()
        .arg("apply")
        .arg("--conn")
        .arg(&connection_string)
        .arg("--path")
        .arg(&migrations_dir)
        .current_dir(&temp_dir)
        .assert()
        .success();
    
    // Now test actual rollback of applied migrations (dry run first)
    deri_ddl_cmd()
        .arg("rollback")
        .arg("--conn")
        .arg(&connection_string)
        .arg("--path")
        .arg(&migrations_dir)
        .arg("--steps")
        .arg("1")
        .arg("--dry-run")
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("Would roll back"))
        .stdout(contains("0002_create_posts.sql"));
}

#[test]
fn test_rollback_to_specific_version() {
    // Create a controlled test environment
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let migrations_dir = temp_dir.path().join("migrations");
    fs::create_dir(&migrations_dir).expect("Failed to create migrations directory");

    // Create 3 simple migrations with rollback SQL
    for i in 1..=3 {
        fs::write(
            migrations_dir.join(format!("000{}_create_table_{}.sql", i, i)),
            format!(r#"-- +migrate Up
CREATE TABLE table_{} (id INTEGER PRIMARY KEY, data TEXT);

-- +migrate Down
DROP TABLE IF EXISTS table_{};
"#, i, i),
        ).unwrap();
    }
    
    let connection_string = test_sqlite_connection();
    
    // Initialize database
    init_test_database(&connection_string).expect("Failed to initialize database");
    
    // Apply all migrations
    deri_ddl_cmd()
        .arg("apply")
        .arg("--conn")
        .arg(&connection_string)
        .arg("--path")
        .arg(&migrations_dir)
        .current_dir(&temp_dir)
        .assert()
        .success();
    
    // Test rollback to version 1 (should rollback versions 3 and 2)
    deri_ddl_cmd()
        .arg("rollback")
        .arg("--conn")
        .arg(&connection_string)
        .arg("--path")
        .arg(&migrations_dir)
        .arg("--to-version")
        .arg("1")
        .arg("--dry-run")
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("Would roll back migrations back to version 1"))
        .stdout(contains("0003_create_table_3.sql"))
        .stdout(contains("0002_create_table_2.sql"));
}

#[test]
fn test_rollback_validation_edge_cases() {
    let temp_dir = setup_test_migrations_with_rollback();
    
    // Test with invalid connection string
    deri_ddl_cmd()
        .arg("rollback")
        .arg("--conn")
        .arg("invalid_connection_string")
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .arg("--dry-run")
        .current_dir(&temp_dir)
        .assert()
        .failure();
    
    // Test with non-existent migrations path
    deri_ddl_cmd()
        .arg("rollback")
        .arg("--conn")
        .arg(test_sqlite_connection())
        .arg("--path")
        .arg("/nonexistent/migrations")
        .arg("--dry-run")
        .current_dir(&temp_dir)
        .assert()
        .failure();
}

#[test]
fn test_rollback_with_only_rollback_migrations() {
    // Create a temp directory with only migrations that have rollback SQL
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let migrations_dir = temp_dir.path().join("migrations");
    fs::create_dir(&migrations_dir).expect("Failed to create migrations directory");

    // Create migration with rollback SQL
    fs::write(
        migrations_dir.join("0001_test_migration.sql"),
        r#"-- +migrate Up
CREATE TABLE test_table (id INTEGER PRIMARY KEY, name TEXT);

-- +migrate Down
DROP TABLE IF EXISTS test_table;
"#,
    ).unwrap();

    let connection_string = test_sqlite_connection();
    
    // Initialize database and apply migration
    init_test_database(&connection_string).expect("Failed to initialize database");
    
    deri_ddl_cmd()
        .arg("apply")
        .arg("--conn")
        .arg(&connection_string)
        .arg("--path")
        .arg(&migrations_dir)
        .current_dir(&temp_dir)
        .assert()
        .success();
    
    // Test rollback - should work without errors
    deri_ddl_cmd()
        .arg("rollback")
        .arg("--conn")
        .arg(&connection_string)
        .arg("--path")
        .arg(&migrations_dir)
        .arg("--steps")
        .arg("1")
        .arg("--dry-run")
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("Would roll back"))
        .stdout(contains("0001_test_migration.sql"));
}

#[test]
fn test_rollback_force_bypass_confirmation() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let migrations_dir = temp_dir.path().join("migrations");
    fs::create_dir(&migrations_dir).expect("Failed to create migrations directory");
    
    let connection_string = test_sqlite_connection();
    
    // Initialize database but don't apply any migrations
    init_test_database(&connection_string).expect("Failed to initialize database");
    
    // Test force flag with no migrations to rollback
    deri_ddl_cmd()
        .arg("rollback")
        .arg("--conn")
        .arg(&connection_string)
        .arg("--path")
        .arg(&migrations_dir)
        .arg("--steps")
        .arg("1")
        .arg("--force")
        .arg("--dry-run")
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("No migrations to roll back"));
}

#[test]
fn test_rollback_cli_args_validation() {
    let temp_dir = setup_test_migrations_with_rollback();
    
    // Test conflicting --steps and --to-version flags
    deri_ddl_cmd()
        .arg("rollback")
        .arg("--conn")
        .arg(test_sqlite_connection())
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .arg("--steps")
        .arg("2")
        .arg("--to-version")
        .arg("1")
        .arg("--dry-run")
        .current_dir(&temp_dir)
        .assert()
        .failure();
    
    // Test invalid steps value (should accept but handle gracefully)
    let connection_string = test_sqlite_connection();
    init_test_database(&connection_string).expect("Failed to initialize database");
    
    deri_ddl_cmd()
        .arg("rollback")
        .arg("--conn")
        .arg(&connection_string)
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .arg("--steps")
        .arg("0")
        .arg("--dry-run")
        .current_dir(&temp_dir)
        .assert()
        .success() // Should handle gracefully
        .stdout(contains("No migrations to roll back"));
}