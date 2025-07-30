use crate::model::Migration;

pub struct Validator;

impl Validator {
    /// Check for common migration issues (gaps, duplicates, etc.)
    pub fn validate_migration_sequence(migrations: &[Migration]) -> Vec<String> {
        let mut issues = Vec::new();

        // Separate versioned and repeatable migrations
        let versioned_migrations: Vec<_> = migrations.iter().filter(|m| !m.is_repeatable()).collect();
        let repeatable_migrations: Vec<_> = migrations.iter().filter(|m| m.is_repeatable()).collect();

        // Check for version gaps in versioned migrations only
        for (i, migration) in versioned_migrations.iter().enumerate() {
            let expected_version = (i + 1) as u32;
            if migration.version != Some(expected_version) {
                issues.push(format!(
                    "Version gap detected: expected {}, found {:?} in {}",
                    expected_version, migration.version, migration.filename()
                ));
            }
        }

        // Check for duplicate versions in versioned migrations
        let mut versions = std::collections::HashSet::new();
        for migration in &versioned_migrations {
            if let Some(version) = migration.version {
                if !versions.insert(version) {
                    issues.push(format!(
                        "Duplicate version {} found in {}",
                        version, migration.filename()
                    ));
                }
            }
        }

        // Check for duplicate names in repeatable migrations
        let mut repeatable_names = std::collections::HashSet::new();
        for migration in &repeatable_migrations {
            if !repeatable_names.insert(&migration.name) {
                issues.push(format!(
                    "Duplicate repeatable migration name '{}' found in {}",
                    migration.name, migration.filename()
                ));
            }
        }

        issues
    }
}