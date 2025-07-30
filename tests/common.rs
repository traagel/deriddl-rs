use assert_cmd::Command;
use std::fs;
use tempfile::{tempdir, TempDir};

/// Returns a configured Command for `deriddl_rs`
pub fn deri_ddl_cmd() -> Command {
    Command::cargo_bin("deriddl_rs").expect("Binary not found")
}

/// Prepares a temp dir with valid migration files
pub fn setup_test_migrations() -> TempDir {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let migrations_dir = temp_dir.path().join("migrations");
    fs::create_dir(&migrations_dir).expect("Failed to create migrations directory");

    fs::write(
        migrations_dir.join("0001_init_schema.sql"),
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
    )
    .unwrap();

    fs::write(
        migrations_dir.join("0002_add_email.sql"),
        "ALTER TABLE users ADD COLUMN email TEXT;",
    )
    .unwrap();

    fs::write(
        migrations_dir.join("0003_create_posts.sql"),
        "CREATE TABLE posts (id INTEGER PRIMARY KEY, user_id INTEGER, title TEXT, content TEXT);",
    )
    .unwrap();

    temp_dir
}

