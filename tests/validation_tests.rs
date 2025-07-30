use deriddl_rs::model::Migration;
use deriddl_rs::orchestrator::validator::Validator;
use std::path::PathBuf;

fn make_migration(version: u32, name: &str) -> Migration {
    let filename = format!("{:04}_{}.sql", version, name);
    Migration::new(
        version,
        name.to_string(),
        PathBuf::from(&filename),
        format!("-- migration {}", version),
    )
}

#[test]
fn detects_version_gap() {
    let migrations = vec![make_migration(1, "0001.sql"), make_migration(3, "0003.sql")];

    let issues = Validator::validate_migration_sequence(&migrations);
    assert!(issues
        .iter()
        .any(|msg| msg.contains("Version gap detected")));
}

#[test]
fn detects_duplicate_versions() {
    let migrations = vec![
        make_migration(2, "0002.sql"),
        make_migration(2, "0002_dup.sql"),
    ];

    let issues = Validator::validate_migration_sequence(&migrations);
    assert!(issues.iter().any(|msg| msg.contains("Duplicate version")));
}

#[test]
fn passes_valid_sequence() {
    let migrations = vec![
        make_migration(1, "0001.sql"),
        make_migration(2, "0002.sql"),
        make_migration(3, "0003.sql"),
    ];

    let issues = Validator::validate_migration_sequence(&migrations);
    for issue in &issues {
        eprintln!("Issue: {}", issue);
    }
    assert!(issues.is_empty());
}

