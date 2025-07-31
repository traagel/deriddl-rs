//! Database dialect system for deriDDL
//! 
//! This module provides a pluggable dialect system for supporting different database types.
//! Each dialect is configured via TOML files and implements the DatabaseDialect trait.

pub mod base;
pub mod registry;

// Dialect modules
pub mod postgres;
pub mod mysql;
pub mod sqlite;
pub mod databricks;
pub mod generic;

// Re-export main types
pub use base::{DatabaseDialect, DialectError};
pub use registry::get_registry;

// Re-export dialect-specific config types
pub use databricks::{
    DatabricksConfig, DatabricksOdbcConfig, DatabricksAuthConfig, DatabricksLoggingConfig,
    DatabricksDriverConfig, DriverInfo, DriverVendor, DriverCapabilities
};

/// Get dialect by name 
pub fn get_dialect(name: &str) -> Option<std::sync::Arc<dyn DatabaseDialect>> {
    let registry = get_registry().lock().unwrap();
    registry.get(name)
}

/// Get dialect by name with config fallback (no auto-detection)
pub fn get_dialect_with_config(
    explicit_name: Option<&str>, 
    _connection_string: Option<&str>,
    config_dialect: Option<&str>
) -> Result<std::sync::Arc<dyn DatabaseDialect>, DialectError> {
    let registry = get_registry().lock().unwrap();
    
    // Priority: explicit name > config dialect > generic fallback
    if let Some(name) = explicit_name {
        if let Some(dialect) = registry.get(name) {
            return Ok(dialect);
        }
    }
    
    if let Some(config_name) = config_dialect {
        if let Some(dialect) = registry.get(config_name) {
            return Ok(dialect);
        }
    }
    
    // Fallback to generic
    registry.get("generic").ok_or_else(|| DialectError::NotFound("No dialect available".to_string()))
}

/// List all available dialect names
pub fn list_dialects() -> Vec<String> {
    let registry = get_registry().lock().unwrap();
    registry.list_dialects()
}