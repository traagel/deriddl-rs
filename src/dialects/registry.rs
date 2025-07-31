use crate::dialects::base::{DatabaseDialect, DetectionResult, DialectError};
use std::collections::HashMap;
use std::sync::Arc;
use log::{debug, warn};

/// Central registry for all available database dialects
pub struct DialectRegistry {
    dialects: HashMap<String, Arc<dyn DatabaseDialect>>,
    aliases: HashMap<String, String>, // alias -> dialect_name mapping
}

impl DialectRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            dialects: HashMap::new(),
            aliases: HashMap::new(),
        }
    }
    
    /// Register a dialect in the registry
    pub fn register(&mut self, dialect: Arc<dyn DatabaseDialect>) {
        let name = dialect.name().to_string();
        debug!("Registering dialect: {}", name);
        
        // Register aliases
        for alias in dialect.aliases() {
            self.aliases.insert(alias.clone(), name.clone());
        }
        
        // Register the dialect itself
        self.dialects.insert(name.clone(), dialect);
    }
    
    /// Get a dialect by name (including aliases)
    pub fn get(&self, name: &str) -> Option<Arc<dyn DatabaseDialect>> {
        // Try direct name lookup first
        if let Some(dialect) = self.dialects.get(name) {
            return Some(dialect.clone());
        }
        
        // Try alias lookup
        if let Some(dialect_name) = self.aliases.get(name) {
            return self.dialects.get(dialect_name).cloned();
        }
        
        None
    }
    
    /// Detect dialect from connection string
    pub fn detect(&self, connection_string: &str) -> Result<Arc<dyn DatabaseDialect>, DialectError> {
        let mut candidates: Vec<(Arc<dyn DatabaseDialect>, DetectionResult)> = Vec::new();
        
        debug!("Detecting dialect for connection string (length: {})", connection_string.len());
        
        // Try each dialect
        for dialect in self.dialects.values() {
            if let Some(result) = dialect.detect(connection_string) {
                debug!("Dialect '{}' matched with confidence {}", result.dialect_name, result.confidence);
                candidates.push((dialect.clone(), result));
            }
        }
        
        if candidates.is_empty() {
            warn!("No dialect detected for connection string");
            return Err(DialectError::NotFound("No matching dialect found".to_string()));
        }
        
        // Sort by confidence (highest first)
        candidates.sort_by(|a, b| b.1.confidence.partial_cmp(&a.1.confidence).unwrap_or(std::cmp::Ordering::Equal));
        
        // Check for ambiguity (multiple high-confidence matches)
        if candidates.len() > 1 && (candidates[0].1.confidence - candidates[1].1.confidence).abs() < 0.1 {
            let names: Vec<String> = candidates.iter().map(|(d, _)| d.name().to_string()).collect();
            return Err(DialectError::Ambiguous(names));
        }
        
        let selected = &candidates[0];
        debug!("Selected dialect: {} (confidence: {})", selected.0.name(), selected.1.confidence);
        
        Ok(selected.0.clone())
    }
    
    /// List all registered dialect names
    pub fn list_dialects(&self) -> Vec<String> {
        self.dialects.keys().cloned().collect()
    }
    
    /// Get all aliases for a dialect
    pub fn get_aliases(&self, dialect_name: &str) -> Vec<String> {
        self.aliases
            .iter()
            .filter(|(_, name)| *name == dialect_name)
            .map(|(alias, _)| alias.clone())
            .collect()
    }
}

impl Default for DialectRegistry {
    fn default() -> Self {
        Self::new()
    }
}

use std::sync::{Mutex, OnceLock};

/// Global registry instance
static GLOBAL_REGISTRY: OnceLock<Mutex<DialectRegistry>> = OnceLock::new();

/// Get the global dialect registry (initialized lazily)
pub fn get_registry() -> &'static Mutex<DialectRegistry> {
    GLOBAL_REGISTRY.get_or_init(|| {
        Mutex::new(create_default_registry())
    })
}

/// Create registry with all built-in dialects
fn create_default_registry() -> DialectRegistry {
    let mut registry = DialectRegistry::new();
    
    // Register built-in dialects
    registry.register(Arc::new(crate::dialects::postgres::PostgresDialect::new()));
    registry.register(Arc::new(crate::dialects::mysql::MysqlDialect::new()));
    registry.register(Arc::new(crate::dialects::sqlite::SqliteDialect::new()));
    registry.register(Arc::new(crate::dialects::databricks::DatabricksDialect::new()));
    registry.register(Arc::new(crate::dialects::generic::GenericDialect::new()));
    
    registry
}