use log::debug;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
fn default_timeout() -> u32 {
    30
}
fn default_max_retries() -> u32 {
    3
}
fn default_migrations_path() -> String {
    "./migrations".to_string()
}
fn default_dialect() -> String {
    "postgres".to_string()
}
fn default_validate_sql() -> bool {
    true
}
fn default_file_pattern() -> String {
    r"^\d{4}_.*\.sql$".to_string()
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_colored() -> bool {
    true
}
fn default_log_format() -> String {
    "pretty".to_string()
}
fn default_require_confirmation() -> bool {
    true
}
fn default_enable_sqlglot() -> bool {
    true
}
fn default_max_file_size_mb() -> u32 {
    10
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

        toml::from_str(&content).map_err(|e| ConfigError::Parse(path.to_string(), e.to_string()))
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
        let toml_content =
            toml::to_string_pretty(&config).map_err(|e| ConfigError::Serialize(e.to_string()))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::{tempdir, NamedTempFile};

    #[test]
    fn test_config_default_values() {
        let config = Config::default();

        // Test database defaults
        assert_eq!(config.database.connection_string, None);
        assert_eq!(config.database.timeout, 30);
        assert_eq!(config.database.max_retries, 3);

        // Test migrations defaults
        assert_eq!(config.migrations.path, "./migrations");
        assert_eq!(config.migrations.dialect, "postgres");
        assert!(config.migrations.validate_sql);
        assert_eq!(config.migrations.file_pattern, r"^\d{4}_.*\.sql$");

        // Test logging defaults
        assert_eq!(config.logging.level, "info");
        assert!(config.logging.colored);
        assert_eq!(config.logging.format, "pretty");

        // Test behavior defaults
        assert!(!config.behavior.auto_create_migrations_dir);
        assert!(config.behavior.require_confirmation);
        assert!(!config.behavior.default_dry_run);

        // Test validation defaults
        assert!(config.validation.enable_sqlglot);
        assert!(!config.validation.strict_validation);
        assert_eq!(config.validation.max_file_size_mb, 10);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();

        // Verify key sections exist
        assert!(toml_str.contains("[database]"));
        assert!(toml_str.contains("[migrations]"));
        assert!(toml_str.contains("[logging]"));
        assert!(toml_str.contains("[behavior]"));
        assert!(toml_str.contains("[validation]"));

        // Verify some specific values
        assert!(toml_str.contains("timeout = 30"));
        assert!(toml_str.contains("path = \"./migrations\""));
        assert!(toml_str.contains("dialect = \"postgres\""));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_content = r#"
[database]
timeout = 60
max_retries = 5

[migrations]
path = "./custom-migrations"
dialect = "mysql"
validate_sql = false

[logging]
level = "debug"
colored = false
format = "json"

[behavior]
auto_create_migrations_dir = true
require_confirmation = false
default_dry_run = true

[validation]
enable_sqlglot = false
strict_validation = true
max_file_size_mb = 20
        "#;

        let config: Config = toml::from_str(toml_content).unwrap();

        // Verify custom values were loaded
        assert_eq!(config.database.timeout, 60);
        assert_eq!(config.database.max_retries, 5);
        assert_eq!(config.migrations.path, "./custom-migrations");
        assert_eq!(config.migrations.dialect, "mysql");
        assert!(!config.migrations.validate_sql);
        assert_eq!(config.logging.level, "debug");
        assert!(!config.logging.colored);
        assert_eq!(config.logging.format, "json");
        assert!(config.behavior.auto_create_migrations_dir);
        assert!(!config.behavior.require_confirmation);
        assert!(config.behavior.default_dry_run);
        assert!(!config.validation.enable_sqlglot);
        assert!(config.validation.strict_validation);
        assert_eq!(config.validation.max_file_size_mb, 20);
    }

    #[test]
    fn test_config_partial_deserialization() {
        let toml_content = r#"
[database]
timeout = 45

[migrations]
dialect = "sqlite"
        "#;

        let config: Config = toml::from_str(toml_content).unwrap();

        // Verify overridden values
        assert_eq!(config.database.timeout, 45);
        assert_eq!(config.migrations.dialect, "sqlite");

        // Verify defaults are still used for unspecified values
        assert_eq!(config.database.max_retries, 3);
        assert_eq!(config.migrations.path, "./migrations");
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_config_load_from_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_content = r#"
[database]
timeout = 120

[migrations]
path = "./test-migrations"
        "#;

        fs::write(temp_file.path(), config_content).unwrap();

        let config = Config::load_from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.database.timeout, 120);
        assert_eq!(config.migrations.path, "./test-migrations");
    }

    #[test]
    fn test_config_load_from_nonexistent_file() {
        let result = Config::load_from_file("/nonexistent/config.toml");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::FileRead(_, _)));
    }

    #[test]
    fn test_config_load_invalid_toml() {
        let temp_file = NamedTempFile::new().unwrap();
        let invalid_content = "invalid toml content [[[";

        fs::write(temp_file.path(), invalid_content).unwrap();

        let result = Config::load_from_file(temp_file.path().to_str().unwrap());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::Parse(_, _)));
    }

    #[test]
    fn test_config_merge() {
        let base_config = Config {
            database: DatabaseConfig {
                connection_string: Some("base-connection".to_string()),
                timeout: 30,
                max_retries: 3,
            },
            migrations: MigrationsConfig {
                path: "./base-migrations".to_string(),
                dialect: "postgres".to_string(),
                validate_sql: true,
                file_pattern: "base-pattern".to_string(),
            },
            ..Config::default()
        };

        let override_config = Config {
            database: DatabaseConfig {
                connection_string: Some("override-connection".to_string()),
                timeout: 60,
                max_retries: 5,
            },
            migrations: MigrationsConfig {
                path: "./override-migrations".to_string(),
                dialect: "mysql".to_string(),
                validate_sql: false,
                file_pattern: "override-pattern".to_string(),
            },
            ..Config::default()
        };

        let merged = base_config.merge(override_config);

        // Verify override values took precedence
        assert_eq!(
            merged.database.connection_string,
            Some("override-connection".to_string())
        );
        assert_eq!(merged.database.timeout, 60);
        assert_eq!(merged.database.max_retries, 5);
        assert_eq!(merged.migrations.path, "./override-migrations");
        assert_eq!(merged.migrations.dialect, "mysql");
        assert!(!merged.migrations.validate_sql);
        assert_eq!(merged.migrations.file_pattern, "override-pattern");
    }

    #[test]
    fn test_config_merge_none_connection_string() {
        let base_config = Config {
            database: DatabaseConfig {
                connection_string: Some("base-connection".to_string()),
                timeout: 30,
                max_retries: 3,
            },
            ..Config::default()
        };

        let override_config = Config {
            database: DatabaseConfig {
                connection_string: None,
                timeout: 60,
                max_retries: 5,
            },
            ..Config::default()
        };

        let merged = base_config.merge(override_config);

        // None connection string should not override existing one
        assert_eq!(
            merged.database.connection_string,
            Some("base-connection".to_string())
        );
        assert_eq!(merged.database.timeout, 60);
        assert_eq!(merged.database.max_retries, 5);
    }

    #[test]
    fn test_generate_default_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("generated-config.toml");

        Config::generate_default_config(config_path.to_str().unwrap()).unwrap();

        // Verify file was created
        assert!(config_path.exists());

        // Verify we can load it back
        let loaded_config = Config::load_from_file(config_path.to_str().unwrap()).unwrap();
        let default_config = Config::default();

        // Verify it matches defaults (we can't use PartialEq because of the complexity)
        assert_eq!(
            loaded_config.database.timeout,
            default_config.database.timeout
        );
        assert_eq!(
            loaded_config.migrations.path,
            default_config.migrations.path
        );
        assert_eq!(loaded_config.logging.level, default_config.logging.level);
    }

    #[test]
    fn test_config_load_with_no_files() {
        let temp_dir = tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Load config with no files present
        let config = Config::load(None, None).unwrap();

        // Should get default config
        let default_config = Config::default();
        assert_eq!(config.database.timeout, default_config.database.timeout);
        assert_eq!(config.migrations.path, default_config.migrations.path);
    }

    #[test]
    fn test_config_load_with_base_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
[database]
timeout = 90

[migrations]
dialect = "mysql"
    "#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load_from_file(config_path.to_str().unwrap()).unwrap();

        assert_eq!(config.database.timeout, 90);
        assert_eq!(config.migrations.dialect, "mysql");
        assert_eq!(config.migrations.path, "./migrations"); // default
    }

    #[test]
    fn test_config_error_display() {
        let errors = vec![
            ConfigError::FileRead("test.toml".to_string(), "Not found".to_string()),
            ConfigError::Parse("test.toml".to_string(), "Invalid syntax".to_string()),
            ConfigError::FileWrite("test.toml".to_string(), "Permission denied".to_string()),
            ConfigError::Serialize("Invalid value".to_string()),
        ];

        for error in errors {
            let error_string = format!("{}", error);
            assert!(!error_string.is_empty());
            // Each error should contain relevant information
            match error {
                ConfigError::FileRead(path, _) => assert!(error_string.contains(&path)),
                ConfigError::Parse(path, _) => assert!(error_string.contains(&path)),
                ConfigError::FileWrite(path, _) => assert!(error_string.contains(&path)),
                ConfigError::Serialize(_) => assert!(error_string.contains("serialize")),
            }
        }
    }
}

