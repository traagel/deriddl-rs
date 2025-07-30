use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use log::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub database: DatabaseConfig,
    
    #[serde(default)]
    pub migrations: MigrationsConfig,
    
    #[serde(default)]
    pub logging: LoggingConfig,
    
    #[serde(default)]
    pub behavior: BehaviorConfig,
    
    #[serde(default)]
    pub validation: ValidationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub connection_string: Option<String>,
    
    #[serde(default = "default_timeout")]
    pub timeout: u32,
    
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationsConfig {
    #[serde(default = "default_migrations_path")]
    pub path: String,
    
    #[serde(default = "default_dialect")]
    pub dialect: String,
    
    #[serde(default = "default_validate_sql")]
    pub validate_sql: bool,
    
    #[serde(default = "default_file_pattern")]
    pub file_pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    
    #[serde(default = "default_colored")]
    pub colored: bool,
    
    #[serde(default = "default_log_format")]
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorConfig {
    #[serde(default)]
    pub auto_create_migrations_dir: bool,
    
    #[serde(default = "default_require_confirmation")]
    pub require_confirmation: bool,
    
    #[serde(default)]
    pub default_dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    #[serde(default = "default_enable_sqlglot")]
    pub enable_sqlglot: bool,
    
    #[serde(default)]
    pub strict_validation: bool,
    
    #[serde(default = "default_max_file_size_mb")]
    pub max_file_size_mb: u32,
}

// Default values
fn default_timeout() -> u32 { 30 }
fn default_max_retries() -> u32 { 3 }
fn default_migrations_path() -> String { "./migrations".to_string() }
fn default_dialect() -> String { "postgres".to_string() }
fn default_validate_sql() -> bool { true }
fn default_file_pattern() -> String { r"^\d{4}_.*\.sql$".to_string() }
fn default_log_level() -> String { "info".to_string() }
fn default_colored() -> bool { true }
fn default_log_format() -> String { "pretty".to_string() }
fn default_require_confirmation() -> bool { true }
fn default_enable_sqlglot() -> bool { true }
fn default_max_file_size_mb() -> u32 { 10 }

impl Default for Config {
    fn default() -> Self {
        Self {
            database: DatabaseConfig::default(),
            migrations: MigrationsConfig::default(),
            logging: LoggingConfig::default(),
            behavior: BehaviorConfig::default(),
            validation: ValidationConfig::default(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            connection_string: None,
            timeout: default_timeout(),
            max_retries: default_max_retries(),
        }
    }
}

impl Default for MigrationsConfig {
    fn default() -> Self {
        Self {
            path: default_migrations_path(),
            dialect: default_dialect(),
            validate_sql: default_validate_sql(),
            file_pattern: default_file_pattern(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            colored: default_colored(),
            format: default_log_format(),
        }
    }
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            auto_create_migrations_dir: false,
            require_confirmation: default_require_confirmation(),
            default_dry_run: false,
        }
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            enable_sqlglot: default_enable_sqlglot(),
            strict_validation: false,
            max_file_size_mb: default_max_file_size_mb(),
        }
    }
}

impl Config {
    /// Load configuration from file with environment override support
    pub fn load(config_path: Option<&str>, environment: Option<&str>) -> Result<Self, ConfigError> {
        let mut config = Config::default();
        
        // Load base configuration file
        if let Some(path) = config_path {
            config = Self::load_from_file(path)?;
        } else {
            // Try loading from standard locations
            for standard_path in Self::standard_config_paths() {
                if standard_path.exists() {
                    debug!("Loading config from: {}", standard_path.display());
                    config = Self::load_from_file(standard_path.to_str().unwrap())?;
                    break;
                }
            }
        }
        
        // Load environment-specific overrides
        if let Some(env) = environment {
            if let Ok(env_config) = Self::load_environment_config(env) {
                debug!("Applying environment config for: {}", env);
                config = config.merge(env_config);
            }
        }
        
        // Load local overrides (always last)
        if let Ok(local_config) = Self::load_from_file("config/local.toml") {
            debug!("Applying local config overrides");
            config = config.merge(local_config);
        }
        
        Ok(config)
    }
    
    /// Load configuration from a specific file
    pub fn load_from_file(path: &str) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)
            .map_err(|e| ConfigError::FileRead(path.to_string(), e.to_string()))?;
            
        toml::from_str(&content)
            .map_err(|e| ConfigError::Parse(path.to_string(), e.to_string()))
    }
    
    /// Load environment-specific configuration
    fn load_environment_config(environment: &str) -> Result<Self, ConfigError> {
        let env_path = format!("config/{}.toml", environment);
        Self::load_from_file(&env_path)
    }
    
    /// Get standard configuration file paths in order of precedence
    fn standard_config_paths() -> Vec<PathBuf> {
        vec![
            PathBuf::from("config.toml"),
            PathBuf::from("config/default.toml"),
        ]
    }
    
    /// Merge this config with another, with the other taking precedence
    pub fn merge(mut self, other: Self) -> Self {
        // Merge database config
        if other.database.connection_string.is_some() {
            self.database.connection_string = other.database.connection_string;
        }
        self.database.timeout = other.database.timeout;
        self.database.max_retries = other.database.max_retries;
        
        // Merge migrations config
        self.migrations.path = other.migrations.path;
        self.migrations.dialect = other.migrations.dialect;
        self.migrations.validate_sql = other.migrations.validate_sql;
        self.migrations.file_pattern = other.migrations.file_pattern;
        
        // Merge logging config
        self.logging.level = other.logging.level;
        self.logging.colored = other.logging.colored;
        self.logging.format = other.logging.format;
        
        // Merge behavior config
        self.behavior.auto_create_migrations_dir = other.behavior.auto_create_migrations_dir;
        self.behavior.require_confirmation = other.behavior.require_confirmation;
        self.behavior.default_dry_run = other.behavior.default_dry_run;
        
        // Merge validation config
        self.validation.enable_sqlglot = other.validation.enable_sqlglot;
        self.validation.strict_validation = other.validation.strict_validation;
        self.validation.max_file_size_mb = other.validation.max_file_size_mb;
        
        self
    }
    
    /// Generate a default configuration file
    pub fn generate_default_config(path: &str) -> Result<(), ConfigError> {
        let config = Config::default();
        let toml_content = toml::to_string_pretty(&config)
            .map_err(|e| ConfigError::Serialize(e.to_string()))?;
            
        fs::write(path, toml_content)
            .map_err(|e| ConfigError::FileWrite(path.to_string(), e.to_string()))?;
            
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file '{0}': {1}")]
    FileRead(String, String),
    
    #[error("Failed to parse config file '{0}': {1}")]
    Parse(String, String),
    
    #[error("Failed to write config file '{0}': {1}")]
    FileWrite(String, String),
    
    #[error("Failed to serialize config: {0}")]
    Serialize(String),
}