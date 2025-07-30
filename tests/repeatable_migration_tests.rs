use deriddl_rs::model::{Migration, MigrationType};
use deriddl_rs::orchestrator::{MigrationLoader, Validator};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn create_test_migration_file(dir: &TempDir, filename: &str, content: &str) -> PathBuf {
    let file_path = dir.path().join(filename);
    fs::write(&file_path, content).expect("Failed to write test migration file");
    file_path
}

fn make_versioned_migration(version: u32, name: &str) -> Migration {
    Migration::new(
        version,
        name.to_string(),
        PathBuf::from(format!("{:04}_{}.sql", version, name)),
        format!("-- versioned migration {}", version),
    )
}

fn make_repeatable_migration(name: &str) -> Migration {
    Migration::new_repeatable(
        name.to_string(),
        PathBuf::from(format!("R__{}.sql", name)),
        format!("-- repeatable migration {}", name),
    )
}

#[test]
fn test_repeatable_migration_creation() {
    let migration = make_repeatable_migration("create_views");
    
    assert_eq!(migration.migration_type, MigrationType::Repeatable);
    assert_eq!(migration.version, None);
    assert_eq!(migration.name, "create_views");
    assert_eq!(migration.filename(), "R__create_views.sql");
    assert_eq!(migration.identifier(), "R__create_views");
    assert!(migration.is_repeatable());
}

#[test]
fn test_versioned_migration_creation() {
    let migration = make_versioned_migration(1, "init_schema");
    
    assert_eq!(migration.migration_type, MigrationType::Versioned);
    assert_eq!(migration.version, Some(1));
    assert_eq!(migration.name, "init_schema");
    assert_eq!(migration.filename(), "0001_init_schema.sql");
    assert_eq!(migration.identifier(), "1");
    assert!(!migration.is_repeatable());
}

#[test]
fn test_migration_loader_parses_r_prefix_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    // Create test files
    create_test_migration_file(&temp_dir, "0001_init.sql", "CREATE TABLE test (id INT);");
    create_test_migration_file(&temp_dir, "R__create_views.sql", "CREATE VIEW test_view AS SELECT * FROM test;");
    create_test_migration_file(&temp_dir, "R__update_functions.sql", "CREATE FUNCTION test_func() RETURNS INT AS $$ BEGIN RETURN 1; END; $$;");
    create_test_migration_file(&temp_dir, "0002_add_data.sql", "INSERT INTO test VALUES (1);");
    
    let migrations = MigrationLoader::load_migrations(temp_dir.path().to_str().unwrap())
        .expect("Failed to load migrations");
    
    assert_eq!(migrations.len(), 4);
    
    // Check sorting: versioned first, then repeatable
    assert_eq!(migrations[0].filename(), "0001_init.sql");
    assert_eq!(migrations[0].migration_type, MigrationType::Versioned);
    
    assert_eq!(migrations[1].filename(), "0002_add_data.sql");
    assert_eq!(migrations[1].migration_type, MigrationType::Versioned);
    
    assert_eq!(migrations[2].filename(), "R__create_views.sql");
    assert_eq!(migrations[2].migration_type, MigrationType::Repeatable);
    
    assert_eq!(migrations[3].filename(), "R__update_functions.sql");
    assert_eq!(migrations[3].migration_type, MigrationType::Repeatable);
}

#[test]
fn test_migration_loader_ignores_invalid_r_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    // Create files with invalid R__ patterns
    create_test_migration_file(&temp_dir, "R_.sql", "-- missing name");
    create_test_migration_file(&temp_dir, "R__", "-- missing .sql extension");
    create_test_migration_file(&temp_dir, "R_test.sql", "-- single underscore");
    create_test_migration_file(&temp_dir, "R__valid_file.sql", "-- valid repeatable");
    
    let migrations = MigrationLoader::load_migrations(temp_dir.path().to_str().unwrap())
        .expect("Failed to load migrations");
    
    // Only the valid file should be loaded
    assert_eq!(migrations.len(), 1);
    assert_eq!(migrations[0].filename(), "R__valid_file.sql");
    assert_eq!(migrations[0].name, "valid_file");
}

#[test]
fn test_validator_handles_mixed_migration_types() {
    let migrations = vec![
        make_versioned_migration(1, "init"),
        make_versioned_migration(2, "add_table"),
        make_repeatable_migration("create_views"),
        make_repeatable_migration("update_functions"),
    ];
    
    let issues = Validator::validate_migration_sequence(&migrations);
    assert!(issues.is_empty(), "Should not have validation issues for mixed types");
}

#[test]
fn test_validator_detects_duplicate_repeatable_names() {
    let migrations = vec![
        make_repeatable_migration("create_views"),
        make_repeatable_migration("create_views"), // duplicate name
    ];
    
    let issues = Validator::validate_migration_sequence(&migrations);
    assert!(issues.iter().any(|msg| msg.contains("Duplicate repeatable migration name")));
}

#[test]
fn test_validator_allows_version_gaps_with_repeatables() {
    let migrations = vec![
        make_versioned_migration(1, "init"),
        make_versioned_migration(3, "skip_two"), // gap in versioned
        make_repeatable_migration("views"),
        make_repeatable_migration("functions"),
    ];
    
    let issues = Validator::validate_migration_sequence(&migrations);
    // Should detect version gap but allow repeatables
    assert!(issues.iter().any(|msg| msg.contains("Version gap detected")));
    assert!(!issues.iter().any(|msg| msg.contains("repeatable")));
}

#[test]
fn test_repeatable_migration_checksum_changes() {
    let migration1 = make_repeatable_migration("test");
    let migration2 = Migration::new_repeatable(
        "test".to_string(),
        PathBuf::from("R__test.sql"),
        "-- different content".to_string(),
    );
    
    // Same name but different content should have different checksums
    assert_eq!(migration1.name, migration2.name);
    assert_ne!(migration1.checksum, migration2.checksum);
}

#[test]
fn test_migration_sorting_with_mixed_types() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    // Create files in non-sorted order
    create_test_migration_file(&temp_dir, "R__zebra.sql", "-- zebra");
    create_test_migration_file(&temp_dir, "0003_third.sql", "-- third");
    create_test_migration_file(&temp_dir, "R__alpha.sql", "-- alpha");
    create_test_migration_file(&temp_dir, "0001_first.sql", "-- first");
    create_test_migration_file(&temp_dir, "0002_second.sql", "-- second");
    create_test_migration_file(&temp_dir, "R__beta.sql", "-- beta");
    
    let migrations = MigrationLoader::load_migrations(temp_dir.path().to_str().unwrap())
        .expect("Failed to load migrations");
    
    // Should be sorted: versioned by version, then repeatable by name
    let expected_order = vec![
        "0001_first.sql",
        "0002_second.sql", 
        "0003_third.sql",
        "R__alpha.sql",
        "R__beta.sql",
        "R__zebra.sql",
    ];
    
    for (i, expected) in expected_order.iter().enumerate() {
        assert_eq!(migrations[i].filename(), *expected);
    }
}

#[test]
fn test_repeatable_migration_identifiers() {
    let migrations = vec![
        make_versioned_migration(1, "init"),
        make_versioned_migration(42, "answer"),
        make_repeatable_migration("create_views"),
        make_repeatable_migration("update_functions_v2"),
    ];
    
    assert_eq!(migrations[0].identifier(), "1");
    assert_eq!(migrations[1].identifier(), "42");
    assert_eq!(migrations[2].identifier(), "R__create_views");
    assert_eq!(migrations[3].identifier(), "R__update_functions_v2");
}

#[test]
fn test_empty_migration_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    let migrations = MigrationLoader::load_migrations(temp_dir.path().to_str().unwrap())
        .expect("Failed to load migrations from empty directory");
    
    assert!(migrations.is_empty());
}

#[test]
fn test_only_repeatable_migrations() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    create_test_migration_file(&temp_dir, "R__views.sql", "-- views");
    create_test_migration_file(&temp_dir, "R__functions.sql", "-- functions");
    create_test_migration_file(&temp_dir, "R__procedures.sql", "-- procedures");
    
    let migrations = MigrationLoader::load_migrations(temp_dir.path().to_str().unwrap())
        .expect("Failed to load migrations");
    
    assert_eq!(migrations.len(), 3);
    assert!(migrations.iter().all(|m| m.is_repeatable()));
    
    // Should be sorted alphabetically by name
    assert_eq!(migrations[0].name, "functions");
    assert_eq!(migrations[1].name, "procedures");  
    assert_eq!(migrations[2].name, "views");
}