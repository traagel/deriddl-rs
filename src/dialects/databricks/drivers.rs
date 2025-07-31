use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Databricks ODBC driver configuration and management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabricksDriverConfig {
    /// Available driver configurations
    #[serde(default)]
    pub drivers: HashMap<String, DriverInfo>,
    
    /// Preferred driver name (key from drivers map)
    pub preferred_driver: Option<String>,
    
    /// Auto-detect available drivers on system
    #[serde(default = "default_auto_detect")]
    pub auto_detect: bool,
    
    /// Custom driver search paths
    #[serde(default)]
    pub search_paths: Vec<PathBuf>,
}

/// Information about a specific ODBC driver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverInfo {
    /// Display name for the driver
    pub name: String,
    
    /// Driver file path (can be absolute or relative)
    pub path: PathBuf,
    
    /// Driver version (if known)
    pub version: Option<String>,
    
    /// Driver vendor/source
    pub vendor: DriverVendor,
    
    /// Driver capabilities and features
    pub capabilities: DriverCapabilities,
    
    /// Installation instructions or download URL
    pub installation_info: Option<String>,
    
    /// Whether this driver is currently available on the system
    #[serde(default)]
    pub available: bool,
}

/// Known ODBC driver vendors for Databricks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DriverVendor {
    /// Official Databricks ODBC driver
    Databricks,
    /// Simba Spark ODBC driver  
    Simba,
    /// Custom/third-party driver
    Custom(String),
}

/// Driver capabilities and feature support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverCapabilities {
    /// Supports Arrow format (performance optimization)
    #[serde(default)]
    pub supports_arrow: bool,
    
    /// Supports Cloud Fetch for large results
    #[serde(default)]
    pub supports_cloud_fetch: bool,
    
    /// OAuth 2.0 authentication support
    #[serde(default = "default_true")]
    pub supports_oauth: bool,
    
    /// Personal Access Token support
    #[serde(default = "default_true")]
    pub supports_pat: bool,
    
    /// Minimum driver version required
    pub min_version: Option<String>,
    
    /// Maximum tested/supported version
    pub max_version: Option<String>,
}

impl Default for DatabricksDriverConfig {
    fn default() -> Self {
        let mut drivers = HashMap::new();
        
        // Add common driver configurations
        drivers.insert("databricks".to_string(), DriverInfo {
            name: "Databricks ODBC Driver".to_string(),
            path: PathBuf::from("/usr/lib/databricks/odbc/lib/libdatabricksodbcw.so"),
            version: None,
            vendor: DriverVendor::Databricks,
            capabilities: DriverCapabilities {
                supports_arrow: true,
                supports_cloud_fetch: true,
                supports_oauth: true,
                supports_pat: true,
                min_version: Some("2.6.15".to_string()),
                max_version: None,
            },
            installation_info: Some("Download from: https://docs.databricks.com/integrations/odbc-jdbc.html".to_string()),
            available: false,
        });
        
        drivers.insert("simba".to_string(), DriverInfo {
            name: "Simba Spark ODBC Driver".to_string(),
            path: PathBuf::from("/opt/simba/spark/lib/64/libsparkodbc64.so"),
            version: None,
            vendor: DriverVendor::Simba,
            capabilities: DriverCapabilities {
                supports_arrow: true,
                supports_cloud_fetch: false,
                supports_oauth: true,
                supports_pat: true,
                min_version: Some("1.6.0".to_string()),
                max_version: None,
            },
            installation_info: Some("Download from Simba or Databricks documentation".to_string()),
            available: false,
        });

        drivers.insert("simba-macos".to_string(), DriverInfo {
            name: "Simba Spark ODBC Driver (macOS)".to_string(),
            path: PathBuf::from("/opt/simba/spark/lib/libsparkodbc.dylib"),
            version: None,
            vendor: DriverVendor::Simba,
            capabilities: DriverCapabilities {
                supports_arrow: true,
                supports_cloud_fetch: false,
                supports_oauth: true,
                supports_pat: true,
                min_version: Some("1.6.0".to_string()),
                max_version: None,
            },
            installation_info: Some("Download from Simba or Databricks documentation".to_string()),
            available: false,
        });

        drivers.insert("simba-windows".to_string(), DriverInfo {
            name: "Simba Spark ODBC Driver (Windows)".to_string(),
            path: PathBuf::from("C:\\Program Files\\Simba Spark ODBC Driver\\lib\\simba.sparkodbc.dll"),
            version: None,
            vendor: DriverVendor::Simba,
            capabilities: DriverCapabilities {
                supports_arrow: true,
                supports_cloud_fetch: false,
                supports_oauth: true,
                supports_pat: true,
                min_version: Some("1.6.0".to_string()),
                max_version: None,
            },
            installation_info: Some("Download from Simba or Databricks documentation".to_string()),
            available: false,
        });
        
        Self {
            drivers,
            preferred_driver: None,
            auto_detect: true,
            search_paths: vec![
                PathBuf::from("/usr/lib"),
                PathBuf::from("/usr/local/lib"),
                PathBuf::from("/opt"),
                PathBuf::from("/Library/ODBC"),  // macOS
            ],
        }
    }
}

impl DatabricksDriverConfig {
    /// Detect available drivers on the system
    pub fn detect_available_drivers(&mut self) -> Vec<String> {
        let mut available_drivers = Vec::new();
        let search_paths = self.search_paths.clone();
        let auto_detect = self.auto_detect;
        
        for (key, driver) in self.drivers.iter_mut() {
            // Check if driver file exists
            if driver.path.exists() {
                driver.available = true;
                available_drivers.push(key.clone());
            } else if auto_detect {
                // Try to find driver in search paths
                if let Some(found_path) = Self::search_for_driver_in_paths(&driver.path, &search_paths) {
                    driver.path = found_path;
                    driver.available = true;
                    available_drivers.push(key.clone());
                }
            }
        }
        
        available_drivers
    }
    
    /// Search for a driver in configured search paths
    fn search_for_driver(&self, driver_filename: &Path) -> Option<PathBuf> {
        Self::search_for_driver_in_paths(driver_filename, &self.search_paths)
    }
    
    /// Search for a driver in given search paths (static helper)
    fn search_for_driver_in_paths(driver_filename: &Path, search_paths: &[PathBuf]) -> Option<PathBuf> {
        let filename = driver_filename.file_name()?;
        
        for search_path in search_paths {
            let candidate = search_path.join(filename);
            if candidate.exists() {
                return Some(candidate);
            }
            
            // Also search recursively in common subdirectories
            let subdirs = ["odbc", "lib", "lib64", "drivers"];
            for subdir in &subdirs {
                let subdir_candidate = search_path.join(subdir).join(filename);
                if subdir_candidate.exists() {
                    return Some(subdir_candidate);
                }
            }
        }
        
        None
    }
    
    /// Get the preferred driver or first available driver
    pub fn get_driver(&self) -> Option<&DriverInfo> {
        // Try preferred driver first
        if let Some(ref preferred) = self.preferred_driver {
            if let Some(driver) = self.drivers.get(preferred) {
                if driver.available {
                    return Some(driver);
                }
            }
        }
        
        // Fall back to first available driver
        self.drivers.values().find(|driver| driver.available)
    }
    
    /// Get driver by specific name
    pub fn get_driver_by_name(&self, name: &str) -> Option<&DriverInfo> {
        self.drivers.get(name)
    }
    
    /// Add a custom driver configuration
    pub fn add_custom_driver(&mut self, key: String, driver: DriverInfo) {
        self.drivers.insert(key, driver);
    }
    
    /// List all available drivers
    pub fn list_available_drivers(&self) -> Vec<(&String, &DriverInfo)> {
        self.drivers.iter()
            .filter(|(_, driver)| driver.available)
            .collect()
    }
    
    /// Validate driver configuration and provide installation guidance
    pub fn validate_and_guide(&self) -> Result<(), String> {
        let available = self.list_available_drivers();
        
        if available.is_empty() {
            let mut guidance = String::from("No ODBC drivers found for Databricks. Install one of:\n\n");
            
            for (key, driver) in &self.drivers {
                guidance.push_str(&format!("{}. {} ({})\n", 
                    key, driver.name, driver.vendor_name()));
                guidance.push_str(&format!("   Path: {}\n", driver.path.display()));
                if let Some(ref install_info) = driver.installation_info {
                    guidance.push_str(&format!("   {}\n", install_info));
                }
                guidance.push('\n');
            }
            
            return Err(guidance);
        }
        
        Ok(())
    }
}

impl DriverInfo {
    /// Get a human-readable vendor name
    pub fn vendor_name(&self) -> String {
        match &self.vendor {
            DriverVendor::Databricks => "Databricks".to_string(),
            DriverVendor::Simba => "Simba Technologies".to_string(),
            DriverVendor::Custom(name) => name.clone(),
        }
    }
    
    /// Check if driver supports a specific feature
    pub fn supports_feature(&self, feature: &str) -> bool {
        match feature.to_lowercase().as_str() {
            "arrow" => self.capabilities.supports_arrow,
            "cloud_fetch" => self.capabilities.supports_cloud_fetch,
            "oauth" => self.capabilities.supports_oauth,
            "pat" => self.capabilities.supports_pat,
            _ => false,
        }
    }
}

// Default helper functions
fn default_auto_detect() -> bool {
    true
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_driver_config() {
        let config = DatabricksDriverConfig::default();
        
        assert!(config.auto_detect);
        assert!(config.drivers.contains_key("databricks"));
        assert!(config.drivers.contains_key("simba"));
        assert!(!config.search_paths.is_empty());
    }
    
    #[test]
    fn test_driver_capabilities() {
        let config = DatabricksDriverConfig::default();
        let databricks_driver = config.get_driver_by_name("databricks").unwrap();
        
        assert!(databricks_driver.supports_feature("arrow"));
        assert!(databricks_driver.supports_feature("oauth"));
        assert!(!databricks_driver.supports_feature("unknown_feature"));
    }
    
    #[test]
    fn test_vendor_names() {
        assert_eq!(DriverVendor::Databricks.to_string(), "Databricks");
        assert_eq!(DriverVendor::Simba.to_string(), "Simba Technologies");
        assert_eq!(DriverVendor::Custom("Test".to_string()).to_string(), "Test");
    }
}

impl std::fmt::Display for DriverVendor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriverVendor::Databricks => write!(f, "Databricks"),
            DriverVendor::Simba => write!(f, "Simba Technologies"),
            DriverVendor::Custom(name) => write!(f, "{}", name),
        }
    }
}