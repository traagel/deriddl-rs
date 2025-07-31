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

/// Prepares a temp dir with migration files that have rollback SQL
pub fn setup_test_migrations_with_rollback() -> TempDir {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let migrations_dir = temp_dir.path().join("migrations");
    fs::create_dir(&migrations_dir).expect("Failed to create migrations directory");

    // Migration with rollback SQL using +migrate format
    fs::write(
        migrations_dir.join("0001_create_users_table.sql"),
        r#"-- +migrate Up
CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_email ON users(email);

-- +migrate Down
DROP INDEX IF EXISTS idx_users_email;
DROP INDEX IF EXISTS idx_users_username;
DROP TABLE IF EXISTS users;
"#,
    )
    .unwrap();

    // Migration with rollback SQL using UP/DOWN format
    fs::write(
        migrations_dir.join("0002_add_user_profiles.sql"),
        r#"-- UP
CREATE TABLE user_profiles (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    bio TEXT,
    avatar_url TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- DOWN
DROP TABLE IF EXISTS user_profiles;
"#,
    )
    .unwrap();

    // Migration with rollback SQL using goose format
    fs::write(
        migrations_dir.join("0003_create_posts_table.sql"),
        r#"-- +goose Up
CREATE TABLE posts (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    content TEXT,
    published BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_posts_user_id ON posts(user_id);
CREATE INDEX idx_posts_published ON posts(published);

-- +goose Down
DROP INDEX IF EXISTS idx_posts_published;
DROP INDEX IF EXISTS idx_posts_user_id;  
DROP TABLE IF EXISTS posts;
"#,
    )
    .unwrap();

    // Migration without rollback SQL
    fs::write(
        migrations_dir.join("0004_add_user_settings.sql"),
        r#"CREATE TABLE user_settings (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    theme TEXT DEFAULT 'light',
    notifications BOOLEAN DEFAULT TRUE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
"#,
    )
    .unwrap();

    // Repeatable migration (should not be rollable) - Fixed SQLite syntax
    fs::write(
        migrations_dir.join("R__create_user_stats_view.sql"),
        r#"-- +migrate Up
DROP VIEW IF EXISTS user_stats;
CREATE VIEW user_stats AS
SELECT 
    u.id,
    u.username,
    COUNT(p.id) as post_count
FROM users u
LEFT JOIN posts p ON u.id = p.user_id
GROUP BY u.id, u.username;

-- +migrate Down
DROP VIEW IF EXISTS user_stats;
"#,
    )
    .unwrap();

    temp_dir
}

/// Returns an SQLite connection string for testing (using a unique file per test)
pub fn test_sqlite_connection() -> String {
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};
    
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
    
    let temp_dir = env::temp_dir();
    let db_path = temp_dir.join(format!("test_db_{}_{}.sqlite", std::process::id(), unique_id));
    format!("Driver=SQLite3;Database={};", db_path.display())
}

/// Initialize a database with schema_migrations table for testing
pub fn init_test_database(connection_string: &str) -> Result<(), Box<dyn std::error::Error>> {
    use deriddl_rs::tracker::schema_init::init_migration_table_with_config;
    init_migration_table_with_config(connection_string, Some("sqlite"))?;
    Ok(())
}

/// Setup database and run initial migrations to prepare for rollback tests
pub fn setup_database_with_applied_migrations(temp_dir: &TempDir) -> Result<String, Box<dyn std::error::Error>> {
    let connection_string = test_sqlite_connection();
    
    // Initialize the schema_migrations table
    init_test_database(&connection_string)?;
    
    // Apply some migrations so we have something to rollback
    let migrations_path = temp_dir.path().join("migrations").to_string_lossy().to_string();
    use deriddl_rs::orchestrator::run_apply;
    run_apply(&connection_string, &migrations_path, false)?;
    
    Ok(connection_string)
}

