use crate::dialects::base::{DatabaseDialect, DialectConfig, DetectionResult};
use std::sync::OnceLock;

static CONFIG: OnceLock<DialectConfig> = OnceLock::new();

pub struct GenericDialect {
    config: &'static DialectConfig,
}

impl GenericDialect {
    pub fn new() -> Self {
        let config = CONFIG.get_or_init(|| {
            let config_str = include_str!("dialect.toml");
            toml::from_str(config_str).expect("Failed to parse Generic dialect config")
        });
        
        Self { config }
    }
}

impl DatabaseDialect for GenericDialect {
    fn config(&self) -> &DialectConfig {
        self.config
    }
    
    fn detect(&self, _connection_string: &str) -> Option<DetectionResult> {
        // Generic dialect always matches with very low confidence as fallback
        Some(DetectionResult {
            dialect_name: self.name().to_string(),
            confidence: 0.1,
            matched_pattern: "fallback".to_string(),
        })
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
            // Basic table listing - this may not work on all databases
            "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'".to_string(),
        ]
    }
    
    fn list_tables_sql(&self) -> String {
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name != 'schema_migrations'".to_string()
    }
}