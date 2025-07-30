use crate::model::Migration;
use log::{debug, warn, error};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub error_message: Option<String>,
}

pub struct Validator;

impl Validator {
    /// Validate SQL using sqlglot CLI
    pub fn validate_sql(sql_content: &str, dialect: &str) -> ValidationResult {
        debug!("Validating SQL with dialect: {}", dialect);
        
        // Check if sqlglot is available
        if !Self::is_sqlglot_available() {
            warn!("sqlglot CLI not found, skipping validation");
            return ValidationResult {
                is_valid: true, // Don't fail if sqlglot not available
                error_message: Some("sqlglot CLI not found".to_string()),
            };
        }

        // Use sqlglot CLI to parse and validate SQL
        let output = Command::new("python")
            .arg("-m")
            .arg("sqlglot")
            .arg("--parse")
            .arg("--read")
            .arg(dialect)
            .arg(sql_content)
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    debug!("SQL validation passed");
                    ValidationResult {
                        is_valid: true,
                        error_message: None,
                    }
                } else {
                    let error_msg = String::from_utf8_lossy(&result.stderr);
                    debug!("SQL validation failed: {}", error_msg);
                    ValidationResult {
                        is_valid: false,
                        error_message: Some(error_msg.to_string()),
                    }
                }
            }
            Err(e) => {
                error!("Failed to run sqlglot: {}", e);
                ValidationResult {
                    is_valid: false,
                    error_message: Some(format!("Failed to run sqlglot: {}", e)),
                }
            }
        }
    }

    /// Check for common migration issues (gaps, duplicates, etc.)
    pub fn validate_migration_sequence(migrations: &[Migration]) -> Vec<String> {
        let mut issues = Vec::new();

        // Check for version gaps
        for (i, migration) in migrations.iter().enumerate() {
            let expected_version = (i + 1) as u32;
            if migration.version != expected_version {
                issues.push(format!(
                    "Version gap detected: expected {}, found {} in {}",
                    expected_version, migration.version, migration.filename()
                ));
            }
        }

        // Check for duplicate versions
        let mut versions = std::collections::HashSet::new();
        for migration in migrations {
            if !versions.insert(migration.version) {
                issues.push(format!(
                    "Duplicate version {} found in {}",
                    migration.version, migration.filename()
                ));
            }
        }

        issues
    }

    /// Check if sqlglot CLI is available
    fn is_sqlglot_available() -> bool {
        Command::new("python")
            .arg("-m")
            .arg("sqlglot")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}