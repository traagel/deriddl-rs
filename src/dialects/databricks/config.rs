use serde::{Deserialize, Serialize};
use super::drivers::DatabricksDriverConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabricksConfig {
    /// ODBC connection parameters for Databricks
    #[serde(default)]
    pub odbc: DatabricksOdbcConfig,
    
    /// Driver configuration and management
    #[serde(default)]
    pub drivers: DatabricksDriverConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabricksOdbcConfig {
    /// ODBC driver path (e.g., "/opt/simba/spark/lib/64/libsparkodbc64.so")
    pub driver_path: Option<String>,
    
    /// Databricks server hostname (e.g., "dbc-1234abcd-5678.cloud.databricks.com")
    pub host: Option<String>,
    
    /// Port number (typically 443 for HTTPS)
    #[serde(default = "default_databricks_port")]
    pub port: u16,
    
    /// HTTP path to cluster/warehouse (e.g., "/sql/1.0/warehouses/abcd1234efgh5678")
    pub http_path: Option<String>,
    
    /// Authentication method configuration
    #[serde(default)]
    pub auth: DatabricksAuthConfig,
    
    /// SSL configuration
    #[serde(default = "default_ssl_enabled")]
    pub ssl: bool,
    
    /// Thrift transport mode (2 for HTTP)
    #[serde(default = "default_thrift_transport")]
    pub thrift_transport: u8,
    
    /// Initial schema/database to connect to
    pub schema: Option<String>,
    
    /// Use native Databricks SQL queries (recommended)
    #[serde(default = "default_use_native_query")]
    pub use_native_query: bool,
    
    /// Logging configuration for ODBC driver
    #[serde(default)]
    pub logging: DatabricksLoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabricksAuthConfig {
    /// Authentication mechanism
    /// 3 = Personal Access Token, 11 = OAuth 2.0
    #[serde(default = "default_auth_mech")]
    pub auth_mech: u8,
    
    /// OAuth flow type (when auth_mech = 11)
    /// 0 = Token pass-through, 1 = M2M, 2 = U2M
    pub auth_flow: Option<u8>,
    
    /// User ID (typically "token" for PAT authentication)
    pub uid: Option<String>,
    
    /// Password/Token (Personal Access Token or password for encryption)
    pub pwd: Option<String>,
    
    /// OAuth access token (for token pass-through)
    pub auth_access_token: Option<String>,
    
    /// OAuth client ID (for M2M authentication)
    pub auth_client_id: Option<String>,
    
    /// OAuth client secret (for M2M authentication)
    pub auth_client_secret: Option<String>,
    
    /// OAuth scope (typically "all-apis")
    pub auth_scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabricksLoggingConfig {
    /// Log level (1-6)
    pub log_level: Option<u8>,
    
    /// Path to log directory
    pub log_path: Option<String>,
    
    /// Maximum number of log files to keep
    pub log_file_count: Option<u32>,
    
    /// Maximum size of each log file in bytes
    pub log_file_size: Option<u64>,
}

// Default functions
fn default_databricks_port() -> u16 {
    443
}

fn default_ssl_enabled() -> bool {
    true
}

fn default_thrift_transport() -> u8 {
    2
}

fn default_use_native_query() -> bool {
    true
}

fn default_auth_mech() -> u8 {
    3 // Personal Access Token by default
}

// Default implementations
impl Default for DatabricksConfig {
    fn default() -> Self {
        Self {
            odbc: DatabricksOdbcConfig::default(),
            drivers: DatabricksDriverConfig::default(),
        }
    }
}

impl Default for DatabricksOdbcConfig {
    fn default() -> Self {
        Self {
            driver_path: None,
            host: None,
            port: default_databricks_port(),
            http_path: None,
            auth: DatabricksAuthConfig::default(),
            ssl: default_ssl_enabled(),
            thrift_transport: default_thrift_transport(),
            schema: None,
            use_native_query: default_use_native_query(),
            logging: DatabricksLoggingConfig::default(),
        }
    }
}

impl Default for DatabricksAuthConfig {
    fn default() -> Self {
        Self {
            auth_mech: default_auth_mech(),
            auth_flow: None,
            uid: None,
            pwd: None,
            auth_access_token: None,
            auth_client_id: None,
            auth_client_secret: None,
            auth_scope: None,
        }
    }
}

impl Default for DatabricksLoggingConfig {
    fn default() -> Self {
        Self {
            log_level: None,
            log_path: None,
            log_file_count: None,
            log_file_size: None,
        }
    }
}