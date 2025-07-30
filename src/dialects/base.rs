use serde::{Deserialize, Serialize};

/// Configuration metadata for a database dialect
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DialectConfig {
    pub metadata: DialectMetadata,
    pub detection: DetectionConfig,
    pub features: FeatureConfig,
    pub sql: SqlConfig,
    pub types: TypeMappings,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DialectMetadata {
    pub name: String,
    pub version: String,
    pub aliases: Vec<String>,
    pub description: String,
    pub min_version: Option<String>,
    pub max_version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DetectionConfig {
    pub connection_patterns: Vec<String>,
    pub driver_patterns: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FeatureConfig {
    pub supports_transactions: bool,
    pub supports_savepoints: bool,
    pub supports_schemas: bool,
    pub supports_sequences: bool,
    pub supports_arrays: bool,
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SqlConfig {
    pub quote_identifier: String,
    pub escape_identifier: String,
    pub current_timestamp: String,
    pub boolean_true: String,
    pub boolean_false: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TypeMappings {
    pub migration_id: String,
    pub migration_type: String,
    pub version: String,
    pub filename: String,
    pub checksum: String,
    pub applied_at: String,
    pub execution_time_ms: String,
    pub success: String,
}

/// Result of dialect detection
#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub dialect_name: String,
    pub confidence: f32,
    pub matched_pattern: String,
}

/// Base trait that all database dialects must implement
pub trait DatabaseDialect: Send + Sync {
    /// Get the dialect configuration
    fn config(&self) -> &DialectConfig;
    
    /// Get the dialect name
    fn name(&self) -> &str {
        &self.config().metadata.name
    }
    
    /// Get dialect aliases
    fn aliases(&self) -> &[String] {
        &self.config().metadata.aliases
    }
    
    /// Detect if this dialect matches the given connection string
    fn detect(&self, connection_string: &str) -> Option<DetectionResult>;
    
    /// Generate SQL for creating the schema_migrations table
    fn create_migrations_table_sql(&self) -> String;
    
    /// Generate SQL for querying schema information
    fn schema_introspection_queries(&self) -> Vec<String>;
    
    /// Generate SQL for listing tables (excluding system tables)
    fn list_tables_sql(&self) -> String;
    
    /// Quote an identifier according to dialect rules
    fn quote_identifier(&self, identifier: &str) -> String {
        let quote = &self.config().sql.quote_identifier;
        let escape = &self.config().sql.escape_identifier;
        let escaped = identifier.replace(quote, escape);
        format!("{}{}{}", quote, escaped, quote)
    }
    
    /// Get current timestamp expression
    fn current_timestamp(&self) -> &str {
        &self.config().sql.current_timestamp
    }
    
    /// Get boolean true value
    fn boolean_true(&self) -> &str {
        &self.config().sql.boolean_true
    }
    
    /// Get boolean false value
    fn boolean_false(&self) -> &str {
        &self.config().sql.boolean_false
    }
}

/// Error types for dialect operations
#[derive(Debug, thiserror::Error)]
pub enum DialectError {
    #[error("Dialect not found: {0}")]
    NotFound(String),
    
    #[error("Multiple dialects detected: {0:?}")]
    Ambiguous(Vec<String>),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Feature not supported: {0}")]
    UnsupportedFeature(String),
}