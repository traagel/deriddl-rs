# Databricks ODBC Driver Configuration

This document explains how to configure custom ODBC drivers for Databricks connections in deriDDL.

## Supported Drivers

deriDDL supports multiple Databricks ODBC drivers:

### 1. Official Databricks ODBC Driver
- **Vendor**: Databricks
- **Default Path**: `/usr/lib/databricks/odbc/lib/libdatabricksodbcw.so`
- **Features**: Arrow serialization, Cloud Fetch, OAuth 2.0, PAT
- **Download**: https://docs.databricks.com/integrations/odbc-jdbc.html

### 2. Simba Spark ODBC Driver
- **Vendor**: Simba Technologies  
- **Linux**: `/opt/simba/spark/lib/64/libsparkodbc64.so`
- **macOS**: `/opt/simba/spark/lib/libsparkodbc.dylib`
- **Windows**: `C:\Program Files\Simba Spark ODBC Driver\lib\simba.sparkodbc.dll`
- **Features**: Arrow serialization, OAuth 2.0, PAT

## Configuration Methods

### Method 1: Automatic Driver Detection
```toml
[databricks.drivers]
auto_detect = true
search_paths = [
    "/usr/lib",
    "/usr/local/lib", 
    "/opt",
    "/Library/ODBC"  # macOS
]

[databricks.odbc]
host = "dbc-abcd1234-5678.cloud.databricks.com"
http_path = "/sql/1.0/warehouses/12345678abcdef90"

[databricks.odbc.auth]
auth_mech = 3
pwd = "dapi-YOUR-PERSONAL-ACCESS-TOKEN-HERE"
```

### Method 2: Explicit Driver Path
```toml
[databricks.odbc]
driver_path = "/opt/simba/spark/lib/64/libsparkodbc64.so"
host = "dbc-abcd1234-5678.cloud.databricks.com"
http_path = "/sql/1.0/warehouses/12345678abcdef90"

[databricks.odbc.auth]
auth_mech = 3
pwd = "dapi-YOUR-PERSONAL-ACCESS-TOKEN-HERE"
```

### Method 3: Custom Driver Registration
```toml
[databricks.drivers]
preferred_driver = "my-custom-driver"

[databricks.drivers.drivers.my-custom-driver]
name = "My Custom Databricks Driver"
path = "/custom/path/to/databricks.so"
vendor = "Custom"
available = true

[databricks.drivers.drivers.my-custom-driver.capabilities]
supports_arrow = true
supports_oauth = true
supports_pat = true
```

## Programmatic Usage

### Driver Detection
```rust
use deriddl_rs::dialects::databricks::DatabricksDialect;

// Check available drivers
match DatabricksDialect::check_driver_availability() {
    Ok(drivers) => {
        println!("Available drivers: {:?}", drivers);
    }
    Err(guidance) => {
        println!("Driver installation guidance:\n{}", guidance);
    }
}

// Get detailed driver info
let driver_info = DatabricksDialect::get_driver_info();
for (name, info) in driver_info {
    println!("{}: {} ({})", name, info.name, info.vendor_name());
}
```

### Connection String Building
```rust
use deriddl_rs::dialects::{DatabricksConfig, DatabricksDialect};

// Method 1: With automatic driver detection
let config = DatabricksConfig::default();
let connection_string = DatabricksDialect::build_connection_string_with_drivers(&config)?;

// Method 2: With explicit ODBC config
let connection_string = DatabricksDialect::build_connection_string(&config.odbc)?;

// Method 3: Validate custom driver
let driver_info = DatabricksDialect::validate_driver("/path/to/custom/driver.so")?;
println!("Driver validated: {}", driver_info.name);
```

## Authentication Methods

### Personal Access Token (Recommended)
```toml
[databricks.odbc.auth]
auth_mech = 3
uid = "token"
pwd = "dapi-EXAMPLE-TOKEN-NOT-REAL-12345678"
```

### OAuth Machine-to-Machine (M2M)
```toml
[databricks.odbc.auth]
auth_mech = 11
auth_flow = 1
auth_client_id = "12345678-1234-1234-1234-123456789012"
auth_client_secret = "your-client-secret"
auth_scope = "all-apis"
```

### OAuth Token Pass-through
```toml
[databricks.odbc.auth]
auth_mech = 11
auth_flow = 0
auth_access_token = "your-oauth-token"
```

## Driver Installation

### Linux (Ubuntu/Debian)
```bash
# Download and install Databricks ODBC driver
wget https://databricks-bi-artifacts.s3.us-east-2.amazonaws.com/databricks-odbc/2.6.25/databricks-odbc-2.6.25.x86_64.deb
sudo dpkg -i databricks-odbc-2.6.25.x86_64.deb

# Or install Simba driver
wget https://databricks-bi-artifacts.s3.us-east-2.amazonaws.com/simba-spark-odbc/2.6.17/SimbaSparkODBC-2.6.17.1018-LinuxRPM-64bit.zip
# Extract and install according to vendor instructions
```

### macOS
```bash
# Download and install Databricks ODBC driver
curl -O https://databricks-bi-artifacts.s3.us-east-2.amazonaws.com/databricks-odbc/2.6.25/databricks-odbc-2.6.25.dmg
# Open DMG and install
```

### Windows
1. Download driver installer from Databricks documentation
2. Run installer as Administrator
3. Configure DSN via ODBC Data Source Administrator

## Troubleshooting

### Driver Not Found
```bash
# Check if driver file exists
ls -la /opt/simba/spark/lib/64/libsparkodbc64.so

# Check library dependencies
ldd /opt/simba/spark/lib/64/libsparkodbc64.so

# Add to library path if needed
export LD_LIBRARY_PATH="/opt/simba/spark/lib/64:$LD_LIBRARY_PATH"
```

### Connection Issues
1. Verify driver path is correct
2. Check authentication credentials
3. Ensure cluster/warehouse is running
4. Validate HTTP path format
5. Check network connectivity to Databricks workspace

### Logging
Enable ODBC driver logging for debugging:
```toml
[databricks.odbc.logging]
log_level = 2
log_path = "/tmp/databricks-odbc-logs"
log_file_count = 5
log_file_size = 10485760
```

## Driver Comparison

| Feature | Databricks Driver | Simba Driver |
|---------|------------------|--------------|
| Arrow Serialization | ✅ (v2.6.15+) | ✅ |
| Cloud Fetch | ✅ (v2.6.17+) | ❌ |
| OAuth 2.0 | ✅ | ✅ |
| Personal Access Token | ✅ | ✅ |
| Platform Support | Linux, macOS, Windows | Linux, macOS, Windows |
| License | Free | Check Simba licensing |

Choose the Databricks driver for best performance and latest features, or Simba for broader compatibility.