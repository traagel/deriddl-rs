use crate::dialects::base::{DatabaseDialect, DetectionResult, DialectConfig};
use regex::Regex;
use std::sync::OnceLock;

static CONFIG: OnceLock<DialectConfig> = OnceLock::new();

pub struct SqliteDialect {
    config: &'static DialectConfig,
}

impl SqliteDialect {
    pub fn new() -> Self {
        let config = CONFIG.get_or_init(|| {
            let config_str = include_str!("dialect.toml");
            toml::from_str(config_str).expect("Failed to parse SQLite dialect config")
        });

        Self { config }
    }
}

impl DatabaseDialect for SqliteDialect {
    fn config(&self) -> &DialectConfig {
        self.config
    }

    fn detect(&self, connection_string: &str) -> Option<DetectionResult> {
        let conn_lower = connection_string.to_lowercase();
        let mut confidence = 0.0f32;
        let mut matched_pattern = String::new();

        // Check connection patterns
        for pattern in &self.config.detection.connection_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(&conn_lower) {
                    confidence = 0.9;
                    matched_pattern = pattern.clone();
                    break;
                }
            }
        }

        // Check driver patterns
        if confidence == 0.0 {
            for pattern in &self.config.detection.driver_patterns {
                if let Ok(re) = Regex::new(pattern) {
                    if re.is_match(connection_string) {
                        confidence = 0.8;
                        matched_pattern = pattern.clone();
                        break;
                    }
                }
            }
        }

        // Fallback to simple string matching
        if confidence == 0.0 {
            if conn_lower.contains("sqlite") {
                confidence = 0.7;
                matched_pattern = "sqlite".to_string();
            }
        }

        if confidence > 0.0 {
            Some(DetectionResult {
                dialect_name: self.name().to_string(),
                confidence,
                matched_pattern,
            })
        } else {
            None
        }
    }

    fn create_migrations_table_sql(&self) -> String {
        let types = &self.config.types;
        format!(
            r#"CREATE TABLE IF NOT EXISTS schema_migrations (
    migration_id {} PRIMARY KEY NOT NULL,
    migration_type {} NOT NULL DEFAULT 'versioned',
    version INTEGER,
    filename {} NOT NULL,
    checksum {} NOT NULL,
    applied_at {} NOT NULL DEFAULT {},
    execution_time_ms {} NOT NULL,
    success {} NOT NULL DEFAULT {}
)"#,
            types.migration_id,
            types.migration_type,
            types.filename,
            types.checksum,
            types.applied_at,
            self.current_timestamp(),
            types.execution_time_ms,
            types.success,
            self.boolean_true()
        )
    }

    fn schema_introspection_queries(&self) -> Vec<String> {
        vec![
            // List all user tables (excluding SQLite system tables)
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != 'schema_migrations'".to_string(),
            // List all views
            "SELECT name FROM sqlite_master WHERE type='view'".to_string(),
            // List all indexes
            "SELECT name FROM sqlite_master WHERE type='index' AND sql NOT NULL".to_string(),
        ]
    }

    fn list_tables_sql(&self) -> String {
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != 'schema_migrations'".to_string()
    }
}

