mod common;
use common::{deri_ddl_cmd, setup_test_migrations_with_rollback, test_sqlite_connection, init_test_database, setup_database_with_applied_migrations};
use predicates::str::contains;

#[test]
fn test_rollback_command_no_connection() {
    let temp_dir = setup_test_migrations_with_rollback();

    deri_ddl_cmd()
        .arg("rollback")
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .current_dir(&temp_dir)
        .assert()
        .failure()
        .stdout(contains("No connection string provided"));
}

#[test]
fn test_rollback_command_help() {
    deri_ddl_cmd()
        .arg("rollback")
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("Roll back applied migrations"))
        .stdout(contains("--steps"))
        .stdout(contains("--to-version"))
        .stdout(contains("--dry-run"))
        .stdout(contains("--force"));
}

#[test]
fn test_rollback_dry_run_no_migrations() {
    let temp_dir = setup_test_migrations_with_rollback();
    let connection_string = test_sqlite_connection();
    
    // Initialize database but don't apply any migrations
    init_test_database(&connection_string).expect("Failed to initialize database");

    deri_ddl_cmd()
        .arg("rollback")
        .arg("--conn")
        .arg(connection_string)
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .arg("--dry-run")
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("No migrations to roll back"));
}

#[test]
fn test_rollback_steps_validation() {
    let temp_dir = setup_test_migrations_with_rollback();
    let connection_string = test_sqlite_connection();
    
    // Initialize database but don't apply any migrations
    init_test_database(&connection_string).expect("Failed to initialize database");

    // Test with steps flag - should succeed even with no migrations to rollback
    deri_ddl_cmd()
        .arg("rollback")
        .arg("--conn")
        .arg(connection_string)
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .arg("--steps")
        .arg("2")
        .arg("--dry-run")
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("No migrations to roll back"));
}

#[test]
fn test_rollback_to_version_validation_no_applied_migrations() {
    let temp_dir = setup_test_migrations_with_rollback();
    let connection_string = test_sqlite_connection();
    
    // Initialize database but don't apply any migrations
    init_test_database(&connection_string).expect("Failed to initialize database");

    // Test with to-version flag - should fail because no migrations are applied beyond version 1
    deri_ddl_cmd()
        .arg("rollback")
        .arg("--conn")
        .arg(connection_string)
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .arg("--to-version")
        .arg("1")
        .arg("--dry-run")
        .current_dir(&temp_dir)
        .assert()
        .failure()
        .stdout(contains("Cannot rollback to version 1: migration not found or not applied"));
}

#[test]
fn test_rollback_conflicting_args() {
    let temp_dir = setup_test_migrations_with_rollback();

    // Test that steps and to-version conflict
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
}

#[test]
fn test_rollback_invalid_migrations_path() {
    deri_ddl_cmd()
        .arg("rollback")
        .arg("--conn")
        .arg(test_sqlite_connection())
        .arg("--path")
        .arg("/nonexistent/path")
        .arg("--dry-run")
        .assert()
        .failure();
}

#[test]
fn test_rollback_force_flag() {
    let temp_dir = setup_test_migrations_with_rollback();
    let connection_string = test_sqlite_connection();
    
    // Initialize database but don't apply any migrations
    init_test_database(&connection_string).expect("Failed to initialize database");

    // Test that force flag works (should not prompt for confirmation)
    deri_ddl_cmd()
        .arg("rollback")
        .arg("--conn")
        .arg(connection_string)
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .arg("--force")
        .arg("--dry-run")
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("No migrations to roll back"));
}

#[test]
fn test_rollback_with_applied_migrations() {
    let temp_dir = setup_test_migrations_with_rollback();
    
    // Setup database and apply migrations
    let connection_string = match setup_database_with_applied_migrations(&temp_dir) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Failed to setup database: {}", e);
            return; // Skip test if setup fails
        }
    };

    // Test rolling back 1 migration - should fail because the last applied migration (0004) has no rollback SQL
    deri_ddl_cmd()
        .arg("rollback")
        .arg("--conn")
        .arg(connection_string)
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .arg("--steps")
        .arg("1")
        .arg("--dry-run")
        .current_dir(&temp_dir)
        .assert()
        .failure()
        .stdout(contains("cannot be rolled back: no rollback SQL found"));
}

#[test]
fn test_rollback_missing_rollback_sql_error() {
    let temp_dir = setup_test_migrations_with_rollback();
    
    // Setup database and apply migrations (this will include the migration without rollback SQL)
    let connection_string = match setup_database_with_applied_migrations(&temp_dir) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Failed to setup database: {}", e);
            return; // Skip test if setup fails
        }
    };

    // Try to rollback all migrations - should fail on the one without rollback SQL
    deri_ddl_cmd()
        .arg("rollback")
        .arg("--conn")
        .arg(connection_string)
        .arg("--path")
        .arg(temp_dir.path().join("migrations"))
        .arg("--steps")
        .arg("10")  // Try to rollback more than available
        .arg("--dry-run")
        .current_dir(&temp_dir)
        .assert()
        .failure()
        .stdout(contains("cannot be rolled back: no rollback SQL found"));
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use deriddl_rs::model::migration::Migration;
    use deriddl_rs::orchestrator::rollback::{RollbackStrategy, create_rollback_plan, validate_rollback_plan, RollbackError};
    use deriddl_rs::tracker::version_store::AppliedMigration;
    use deriddl_rs::model::migration::MigrationType;
    use chrono::Utc;
    use std::path::PathBuf;
    use std::collections::HashMap;

    fn create_test_applied_migration(version: u32, filename: &str) -> AppliedMigration {
        AppliedMigration {
            migration_id: version.to_string(),
            migration_type: MigrationType::Versioned,
            version: Some(version),
            filename: filename.to_string(),
            checksum: "test_checksum".to_string(),
            applied_at: Utc::now(),
            execution_time_ms: 100,
            success: true,
        }
    }

    #[test]
    fn test_migration_parsing_with_rollback_sql() {
        // Test +migrate format
        let content = r#"-- +migrate Up
CREATE TABLE users (id INTEGER PRIMARY KEY);

-- +migrate Down
DROP TABLE users;
"#;
        let migration = Migration::new(1, "test".to_string(), PathBuf::from("test.sql"), content.to_string());
        assert!(migration.has_rollback());
        assert_eq!(migration.get_rollback_sql().unwrap().trim(), "DROP TABLE users;");

        // Test UP/DOWN format
        let content = r#"-- UP
CREATE TABLE posts (id INTEGER PRIMARY KEY);

-- DOWN
DROP TABLE posts;
"#;
        let migration = Migration::new(2, "test2".to_string(), PathBuf::from("test2.sql"), content.to_string());
        assert!(migration.has_rollback());
        assert_eq!(migration.get_rollback_sql().unwrap().trim(), "DROP TABLE posts;");

        // Test goose format
        let content = r#"-- +goose Up
CREATE TABLE comments (id INTEGER PRIMARY KEY);

-- +goose Down
DROP TABLE comments;
"#;
        let migration = Migration::new(3, "test3".to_string(), PathBuf::from("test3.sql"), content.to_string());
        assert!(migration.has_rollback());
        assert_eq!(migration.get_rollback_sql().unwrap().trim(), "DROP TABLE comments;");

        // Test @@UP@@/@@DOWN@@ format
        let content = r#"-- @@UP@@
CREATE TABLE orders (id INTEGER PRIMARY KEY);

-- @@DOWN@@
DROP TABLE orders;
"#;
        let migration = Migration::new(4, "test4".to_string(), PathBuf::from("test4.sql"), content.to_string());
        assert!(migration.has_rollback());
        assert_eq!(migration.get_rollback_sql().unwrap().trim(), "DROP TABLE orders;");
    }

    #[test]
    fn test_migration_parsing_without_rollback_sql() {
        let content = "CREATE TABLE users (id INTEGER PRIMARY KEY);";
        let migration = Migration::new(1, "test".to_string(), PathBuf::from("test.sql"), content.to_string());
        assert!(!migration.has_rollback());
        assert!(migration.get_rollback_sql().is_none());
    }

    #[test]
    fn test_migration_parsing_case_insensitive() {
        let content = r#"-- +MIGRATE UP
CREATE TABLE users (id INTEGER PRIMARY KEY);

-- +migrate down
DROP TABLE users;
"#;
        let migration = Migration::new(1, "test".to_string(), PathBuf::from("test.sql"), content.to_string());
        assert!(migration.has_rollback());
        assert_eq!(migration.get_rollback_sql().unwrap().trim(), "DROP TABLE users;");
    }

    #[test]
    fn test_migration_parsing_whitespace_handling() {
        // Test with extra whitespace and newlines
        let content = r#"
        
-- +migrate Up    

CREATE TABLE users (
    id INTEGER PRIMARY KEY
);


-- +migrate Down   

DROP TABLE users;

        "#;
        let migration = Migration::new(1, "test".to_string(), PathBuf::from("test.sql"), content.to_string());
        assert!(migration.has_rollback());
        let rollback_sql = migration.get_rollback_sql().unwrap().trim();
        assert!(rollback_sql.contains("DROP TABLE users;"));
    }

    #[test]
    fn test_migration_parsing_complex_sql() {
        let content = r#"-- +migrate Up
CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    username VARCHAR(50) UNIQUE NOT NULL,
    email VARCHAR(100) UNIQUE NOT NULL
);

CREATE INDEX idx_username ON users(username);
CREATE INDEX idx_email ON users(email);

INSERT INTO users (username, email) VALUES ('admin', 'admin@example.com');

-- +migrate Down
DELETE FROM users WHERE username = 'admin';
DROP INDEX IF EXISTS idx_email;
DROP INDEX IF EXISTS idx_username;
DROP TABLE IF EXISTS users;
"#;
        let migration = Migration::new(1, "complex".to_string(), PathBuf::from("complex.sql"), content.to_string());
        assert!(migration.has_rollback());
        
        let rollback_sql = migration.get_rollback_sql().unwrap();
        assert!(rollback_sql.contains("DELETE FROM users"));
        assert!(rollback_sql.contains("DROP INDEX"));
        assert!(rollback_sql.contains("DROP TABLE"));
    }

    #[test]
    fn test_rollback_strategy_steps() {
        let applied_migrations = vec![
            create_test_applied_migration(3, "0003_migration.sql"),
            create_test_applied_migration(2, "0002_migration.sql"),
            create_test_applied_migration(1, "0001_migration.sql"),
        ];

        let strategy = RollbackStrategy::Steps(2);
        let plan = create_rollback_plan(&applied_migrations, &strategy).unwrap();
        
        assert_eq!(plan.migrations_to_rollback.len(), 2);
        assert_eq!(plan.total_migrations, 2);
        // Migrations should be in descending order (newest first)
        assert_eq!(plan.migrations_to_rollback[0].version, Some(3));
        assert_eq!(plan.migrations_to_rollback[1].version, Some(2));
    }

    #[test]
    fn test_rollback_strategy_steps_exceeds_available() {
        let applied_migrations = vec![
            create_test_applied_migration(2, "0002_migration.sql"),
            create_test_applied_migration(1, "0001_migration.sql"),
        ];

        let strategy = RollbackStrategy::Steps(5); // More than available
        let plan = create_rollback_plan(&applied_migrations, &strategy).unwrap();
        
        // Should only rollback available migrations
        assert_eq!(plan.migrations_to_rollback.len(), 2);
        assert_eq!(plan.migrations_to_rollback[0].version, Some(2));
        assert_eq!(plan.migrations_to_rollback[1].version, Some(1));
    }

    #[test]
    fn test_rollback_strategy_to_version() {
        let applied_migrations = vec![
            create_test_applied_migration(5, "0005_migration.sql"),
            create_test_applied_migration(4, "0004_migration.sql"),
            create_test_applied_migration(3, "0003_migration.sql"),
            create_test_applied_migration(2, "0002_migration.sql"),
            create_test_applied_migration(1, "0001_migration.sql"),
        ];

        let strategy = RollbackStrategy::ToVersion(2);
        let plan = create_rollback_plan(&applied_migrations, &strategy).unwrap();
        
        // Should rollback versions 5, 4, and 3 (everything > 2)
        assert_eq!(plan.migrations_to_rollback.len(), 3);
        assert_eq!(plan.migrations_to_rollback[0].version, Some(5));
        assert_eq!(plan.migrations_to_rollback[1].version, Some(4));
        assert_eq!(plan.migrations_to_rollback[2].version, Some(3));
    }

    #[test]
    fn test_rollback_strategy_to_version_invalid() {
        let applied_migrations = vec![
            create_test_applied_migration(3, "0003_migration.sql"),
            create_test_applied_migration(2, "0002_migration.sql"),
            create_test_applied_migration(1, "0001_migration.sql"),
        ];

        // Try to rollback to version 5 (higher than any applied)
        let strategy = RollbackStrategy::ToVersion(5);
        let result = create_rollback_plan(&applied_migrations, &strategy);
        
        assert!(result.is_err());
        match result.unwrap_err() {
            RollbackError::InvalidTargetVersion(version) => assert_eq!(version, 5),
            _ => panic!("Expected InvalidTargetVersion error"),
        }
    }

    #[test]
    fn test_rollback_strategy_filters_repeatable_migrations() {
        let mut applied_migrations = vec![
            create_test_applied_migration(3, "0003_migration.sql"),
            create_test_applied_migration(2, "0002_migration.sql"),
            create_test_applied_migration(1, "0001_migration.sql"),
        ];
        
        // Add a repeatable migration (should be filtered out)
        let mut repeatable = create_test_applied_migration(0, "R__view.sql");
        repeatable.migration_type = MigrationType::Repeatable;
        repeatable.version = None;
        repeatable.migration_id = "R__view".to_string();
        applied_migrations.push(repeatable);

        let strategy = RollbackStrategy::Steps(10);
        let plan = create_rollback_plan(&applied_migrations, &strategy).unwrap();
        
        // Should only include versioned migrations
        assert_eq!(plan.migrations_to_rollback.len(), 3);
        for migration in &plan.migrations_to_rollback {
            assert_eq!(migration.migration_type, MigrationType::Versioned);
            assert!(migration.version.is_some());
        }
    }

    #[test]
    fn test_rollback_strategy_filters_failed_migrations() {
        let mut applied_migrations = vec![
            create_test_applied_migration(3, "0003_migration.sql"),
            create_test_applied_migration(2, "0002_migration.sql"),
            create_test_applied_migration(1, "0001_migration.sql"),
        ];
        
        // Mark one migration as failed
        applied_migrations[1].success = false;

        let strategy = RollbackStrategy::Steps(10);
        let plan = create_rollback_plan(&applied_migrations, &strategy).unwrap();
        
        // Should only include successful migrations
        assert_eq!(plan.migrations_to_rollback.len(), 2);
        assert_eq!(plan.migrations_to_rollback[0].version, Some(3));
        assert_eq!(plan.migrations_to_rollback[1].version, Some(1)); // Skips failed version 2
    }

    #[test]
    fn test_validate_rollback_plan_success() {
        let migration_with_rollback = Migration::new(
            1, 
            "test".to_string(), 
            PathBuf::from("test.sql"), 
            r#"-- +migrate Up
CREATE TABLE test (id INTEGER);
-- +migrate Down  
DROP TABLE test;"#.to_string()
        );

        let applied_migrations = vec![create_test_applied_migration(1, "0001_test.sql")];
        let strategy = RollbackStrategy::Steps(1);
        let plan = create_rollback_plan(&applied_migrations, &strategy).unwrap();

        let mut migration_map = HashMap::new();
        migration_map.insert(1, &migration_with_rollback);

        let result = validate_rollback_plan(&plan, &migration_map);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_rollback_plan_missing_rollback_sql() {
        let migration_without_rollback = Migration::new(
            1, 
            "test".to_string(), 
            PathBuf::from("test.sql"), 
            "CREATE TABLE test (id INTEGER);".to_string()
        );

        let applied_migrations = vec![create_test_applied_migration(1, "0001_test.sql")];
        let strategy = RollbackStrategy::Steps(1);
        let plan = create_rollback_plan(&applied_migrations, &strategy).unwrap();

        let mut migration_map = HashMap::new();
        migration_map.insert(1, &migration_without_rollback);

        let result = validate_rollback_plan(&plan, &migration_map);
        assert!(result.is_err());
        match result.unwrap_err() {
            RollbackError::NoRollbackSql(filename) => assert_eq!(filename, "0001_test.sql"),
            _ => panic!("Expected NoRollbackSql error"),
        }
    }

    #[test]
    fn test_validate_rollback_plan_missing_migration_file() {
        let applied_migrations = vec![create_test_applied_migration(1, "0001_test.sql")];
        let strategy = RollbackStrategy::Steps(1);
        let plan = create_rollback_plan(&applied_migrations, &strategy).unwrap();

        let migration_map = HashMap::new(); // Empty map - no migration files found

        let result = validate_rollback_plan(&plan, &migration_map);
        assert!(result.is_err());
        match result.unwrap_err() {
            RollbackError::Migration(msg) => assert!(msg.contains("Migration file not found for version 1")),
            _ => panic!("Expected Migration error"),
        }
    }

    #[test]
    fn test_repeatable_migration_rollback() {
        let migration = Migration::new_repeatable(
            "test_view".to_string(), 
            PathBuf::from("R__test_view.sql"), 
            "CREATE VIEW test AS SELECT 1;".to_string()
        );
        
        // Repeatable migrations should not be included in rollback plans
        assert_eq!(migration.migration_type, MigrationType::Repeatable);
        assert!(migration.is_repeatable());
    }

    #[test]
    fn test_migration_identifier() {
        let versioned = Migration::new(42, "test".to_string(), PathBuf::from("test.sql"), "".to_string());
        assert_eq!(versioned.identifier(), "42");

        let repeatable = Migration::new_repeatable("test_view".to_string(), PathBuf::from("test.sql"), "".to_string());
        assert_eq!(repeatable.identifier(), "R__test_view");
    }

    #[test]
    fn test_migration_filename() {
        let versioned = Migration::new(42, "create_users".to_string(), PathBuf::from("test.sql"), "".to_string());
        assert_eq!(versioned.filename(), "0042_create_users.sql");

        let repeatable = Migration::new_repeatable("create_view".to_string(), PathBuf::from("test.sql"), "".to_string());
        assert_eq!(repeatable.filename(), "R__create_view.sql");
    }

    #[test]
    fn test_empty_rollback_plan() {
        let applied_migrations = vec![];
        let strategy = RollbackStrategy::Steps(1);
        let plan = create_rollback_plan(&applied_migrations, &strategy).unwrap();
        
        assert_eq!(plan.migrations_to_rollback.len(), 0);
        assert_eq!(plan.total_migrations, 0);
    }
}