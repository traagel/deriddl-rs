use crate::dialects::base::{DatabaseDialect, DialectConfig, DetectionResult};
use super::config::{DatabricksOdbcConfig, DatabricksConfig};
use super::drivers::{DatabricksDriverConfig, DriverInfo};
use std::sync::OnceLock;

static CONFIG: OnceLock<DialectConfig> = OnceLock::new();

pub struct DatabricksDialect {
    config: &'static DialectConfig,
}

impl DatabricksDialect {
    pub fn new() -> Self {
        let config = CONFIG.get_or_init(|| {
            let config_str = include_str!("dialect.toml");
            toml::from_str(config_str).expect("Failed to parse Databricks dialect config")
        });
        
        Self { config }
    }
}

impl DatabaseDialect for DatabricksDialect {
    fn config(&self) -> &DialectConfig {
        self.config
    }
    
    fn detect(&self, _connection_string: &str) -> Option<DetectionResult> {
        // Detection not used - dialect selection is config-based
        None
    }
    
    fn create_migrations_table_sql(&self) -> String {
        let types = &self.config.types;
        format!(
            r#"CREATE TABLE IF NOT EXISTS schema_migrations (
    migration_id {} NOT NULL,
    migration_type {} NOT NULL,
    version {},
    filename {} NOT NULL,
    checksum {} NOT NULL,
    applied_at {} NOT NULL,
    execution_time_ms {} NOT NULL,
    success {} NOT NULL
) USING DELTA"#,
            types.migration_id,
            types.migration_type,
            types.version,
            types.filename,
            types.checksum,
            types.applied_at,
            types.execution_time_ms,
            types.success
        )
    }
    
    fn schema_introspection_queries(&self) -> Vec<String> {
        vec![
            // List all user tables (Databricks/Spark SQL specific)
            "SHOW TABLES".to_string(),
            // List all databases/schemas
            "SHOW DATABASES".to_string(),
            // List tables in current database with details
            "SHOW TABLE EXTENDED LIKE '*'".to_string(),
        ]
    }
    
    fn list_tables_sql(&self) -> String {
        // Use Spark SQL syntax to list tables, excluding schema_migrations
        "SHOW TABLES LIKE '*' WHERE NOT isTemporary AND tableName != 'schema_migrations'".to_string()
    }
}

impl Default for DatabricksDialect {
    fn default() -> Self {
        Self::new()
    }
}

impl DatabricksDialect {
    /// Build ODBC connection string with automatic driver detection
    pub fn build_connection_string_with_drivers(config: &DatabricksConfig) -> Result<String, String> {
        let mut driver_config = config.drivers.clone();
        
        // Detect available drivers
        let available = driver_config.detect_available_drivers();
        if available.is_empty() {
            return Err(driver_config.validate_and_guide().unwrap_err());
        }
        
        // Get the best available driver
        let driver = driver_config.get_driver()
            .ok_or("No suitable driver found")?;
        
        // Use the detected driver path in ODBC config
        let mut odbc_config = config.odbc.clone();
        odbc_config.driver_path = Some(driver.path.to_string_lossy().to_string());
        
        Self::build_connection_string(&odbc_config)
    }

    /// Build ODBC connection string from configuration parameters
    pub fn build_connection_string(config: &DatabricksOdbcConfig) -> Result<String, String> {
        // Validate required parameters
        let driver_path = config.driver_path.as_ref()
            .ok_or("driver_path is required for Databricks ODBC connection")?;
        let host = config.host.as_ref()
            .ok_or("host is required for Databricks ODBC connection")?;
        let http_path = config.http_path.as_ref()
            .ok_or("http_path is required for Databricks ODBC connection")?;

        let mut connection_parts = Vec::new();

        // Core connection parameters
        connection_parts.push(format!("Driver={}", driver_path));
        connection_parts.push(format!("Host={}", host));
        connection_parts.push(format!("Port={}", config.port));
        connection_parts.push(format!("HTTPPath={}", http_path));
        connection_parts.push(format!("SSL={}", if config.ssl { 1 } else { 0 }));
        connection_parts.push(format!("ThriftTransport={}", config.thrift_transport));

        // Authentication parameters
        connection_parts.push(format!("AuthMech={}", config.auth.auth_mech));

        match config.auth.auth_mech {
            3 => {
                // Personal Access Token authentication
                if let Some(uid) = &config.auth.uid {
                    connection_parts.push(format!("UID={}", uid));
                } else {
                    connection_parts.push("UID=token".to_string());
                }
                
                if let Some(pwd) = &config.auth.pwd {
                    connection_parts.push(format!("PWD={}", pwd));
                } else {
                    return Err("pwd (Personal Access Token) is required for AuthMech=3".to_string());
                }
            }
            11 => {
                // OAuth 2.0 authentication
                let auth_flow = config.auth.auth_flow
                    .ok_or("auth_flow is required for OAuth authentication (AuthMech=11)")?;
                connection_parts.push(format!("Auth_Flow={}", auth_flow));

                match auth_flow {
                    0 => {
                        // Token pass-through
                        if let Some(token) = &config.auth.auth_access_token {
                            connection_parts.push(format!("Auth_AccessToken={}", token));
                        } else {
                            return Err("auth_access_token is required for OAuth token pass-through (Auth_Flow=0)".to_string());
                        }
                    }
                    1 => {
                        // Machine-to-Machine (M2M)
                        if let Some(client_id) = &config.auth.auth_client_id {
                            connection_parts.push(format!("Auth_Client_ID={}", client_id));
                        } else {
                            return Err("auth_client_id is required for OAuth M2M (Auth_Flow=1)".to_string());
                        }
                        
                        if let Some(client_secret) = &config.auth.auth_client_secret {
                            connection_parts.push(format!("Auth_Client_Secret={}", client_secret));
                        } else {
                            return Err("auth_client_secret is required for OAuth M2M (Auth_Flow=1)".to_string());
                        }
                        
                        if let Some(scope) = &config.auth.auth_scope {
                            connection_parts.push(format!("Auth_Scope={}", scope));
                        } else {
                            connection_parts.push("Auth_Scope=all-apis".to_string());
                        }
                    }
                    2 => {
                        // User-to-Machine (U2M)
                        if let Some(pwd) = &config.auth.pwd {
                            connection_parts.push(format!("PWD={}", pwd));
                        } else {
                            return Err("pwd (password for refresh token encryption) is required for OAuth U2M (Auth_Flow=2)".to_string());
                        }
                    }
                    _ => {
                        return Err(format!("Unsupported OAuth flow: {}. Supported flows: 0 (token pass-through), 1 (M2M), 2 (U2M)", auth_flow));
                    }
                }
            }
            _ => {
                return Err(format!("Unsupported authentication mechanism: {}. Supported: 3 (PAT), 11 (OAuth)", config.auth.auth_mech));
            }
        }

        // Optional parameters
        if let Some(schema) = &config.schema {
            connection_parts.push(format!("Schema={}", schema));
        }

        if config.use_native_query {
            connection_parts.push("UseNativeQuery=1".to_string());
        }

        // Logging parameters (non-Windows)
        if let Some(log_level) = config.logging.log_level {
            connection_parts.push(format!("LogLevel={}", log_level));
        }
        if let Some(log_path) = &config.logging.log_path {
            connection_parts.push(format!("LogPath={}", log_path));
        }
        if let Some(log_file_count) = config.logging.log_file_count {
            connection_parts.push(format!("LogFileCount={}", log_file_count));
        }
        if let Some(log_file_size) = config.logging.log_file_size {
            connection_parts.push(format!("LogFileSize={}", log_file_size));
        }

        Ok(connection_parts.join("; "))
    }

    /// Parse connection string and validate Databricks-specific parameters
    pub fn validate_connection_string(connection_string: &str) -> Result<(), String> {
        let params: std::collections::HashMap<String, String> = connection_string
            .split(';')
            .filter_map(|pair| {
                let mut parts = pair.split('=');
                match (parts.next(), parts.next()) {
                    (Some(key), Some(value)) => Some((key.trim().to_lowercase(), value.trim().to_string())),
                    _ => None,
                }
            })
            .collect();

        // Check required parameters
        let required_params = ["driver", "host", "httppath"];
        for param in &required_params {
            if !params.contains_key(*param) {
                return Err(format!("Missing required parameter: {}", param));
            }
        }

        // Validate authentication
        if let Some(auth_mech) = params.get("authmech") {
            match auth_mech.as_str() {
                "3" => {
                    if !params.contains_key("pwd") {
                        return Err("PWD (Personal Access Token) is required for AuthMech=3".to_string());
                    }
                }
                "11" => {
                    if let Some(auth_flow) = params.get("auth_flow") {
                        match auth_flow.as_str() {
                            "0" => {
                                if !params.contains_key("auth_accesstoken") {
                                    return Err("Auth_AccessToken is required for OAuth token pass-through".to_string());
                                }
                            }
                            "1" => {
                                if !params.contains_key("auth_client_id") || !params.contains_key("auth_client_secret") {
                                    return Err("Auth_Client_ID and Auth_Client_Secret are required for OAuth M2M".to_string());
                                }
                            }
                            "2" => {
                                if !params.contains_key("pwd") {
                                    return Err("PWD is required for OAuth U2M".to_string());
                                }
                            }
                            _ => return Err(format!("Invalid Auth_Flow: {}", auth_flow)),
                        }
                    } else {
                        return Err("Auth_Flow is required for OAuth authentication".to_string());
                    }
                }
                _ => return Err(format!("Unsupported AuthMech: {}", auth_mech)),
            }
        } else {
            return Err("AuthMech parameter is required".to_string());
        }

        Ok(())
    }
    
    /// Detect and list available ODBC drivers
    pub fn detect_drivers() -> DatabricksDriverConfig {
        let mut config = DatabricksDriverConfig::default();
        config.detect_available_drivers();
        config
    }
    
    /// Check if any Databricks ODBC drivers are available
    pub fn check_driver_availability() -> Result<Vec<String>, String> {
        let mut config = DatabricksDriverConfig::default();
        let available = config.detect_available_drivers();
        
        if available.is_empty() {
            Err(config.validate_and_guide().unwrap_err())
        } else {
            Ok(available)
        }
    }
    
    /// Get detailed information about available drivers
    pub fn get_driver_info() -> Vec<(String, DriverInfo)> {
        let mut config = DatabricksDriverConfig::default();
        config.detect_available_drivers();
        
        config.list_available_drivers()
            .into_iter()
            .map(|(key, driver)| (key.clone(), driver.clone()))
            .collect()
    }
    
    /// Validate a specific driver configuration
    pub fn validate_driver(driver_path: &str) -> Result<DriverInfo, String> {
        use std::path::Path;
        
        let path = Path::new(driver_path);
        if !path.exists() {
            return Err(format!("Driver file not found: {}", driver_path));
        }
        
        // Try to determine driver type based on path/filename
        let filename = path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        
        let vendor = if filename.contains("databricks") {
            super::drivers::DriverVendor::Databricks
        } else if filename.contains("simba") || filename.contains("spark") {
            super::drivers::DriverVendor::Simba
        } else {
            super::drivers::DriverVendor::Custom("Unknown".to_string())
        };
        
        Ok(DriverInfo {
            name: format!("Custom Driver ({})", filename),
            path: path.to_path_buf(),
            version: None,
            vendor,
            capabilities: super::drivers::DriverCapabilities {
                supports_arrow: false, // Unknown, assume conservative
                supports_cloud_fetch: false,
                supports_oauth: true,
                supports_pat: true,
                min_version: None,
                max_version: None,
            },
            installation_info: None,
            available: true,
        })
    }
}