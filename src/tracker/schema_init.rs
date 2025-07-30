use crate::dialects;
use crate::executor::{ConnectionError, ConnectionManager, DatabaseExecutor};
use log::{debug, error, info};

pub fn init_migration_table(conn_string: &str) -> Result<(), ConnectionError> {
    init_migration_table_with_config(conn_string, None)
}

pub fn init_migration_table_with_config(
    conn_string: &str,
    config_dialect: Option<&str>,
) -> Result<(), ConnectionError> {
    info!("Initializing schema_migrations table");
    debug!("Connection string length: {}", conn_string.len());

    let connection_manager = ConnectionManager::new()?;
    let connection = connection_manager.connect(conn_string)?;
    let mut executor = DatabaseExecutor::new(connection);

    // Get dialect with config support
    let dialect = match dialects::get_dialect_with_config(None, Some(conn_string), config_dialect) {
        Ok(dialect) => {
            info!(
                "Using database dialect: {} (source: {})",
                dialect.name(),
                if config_dialect.is_some() {
                    "config"
                } else {
                    "auto-detected"
                }
            );
            dialect
        }
        Err(e) => {
            error!("Failed to get dialect: {}", e);
            return Err(ConnectionError::Other(format!("Dialect error: {}", e)));
        }
    };

    let create_table_sql = dialect.create_migrations_table_sql();
    debug!(
        "Creating schema_migrations table with dialect: {}",
        dialect.name()
    );
    executor.execute_query(&create_table_sql)?;

    // Verify table was created by querying it
    match executor.query_single_value("SELECT COUNT(*) FROM schema_migrations") {
        Ok(_) => {
            info!("✅ schema_migrations table initialized successfully");
            Ok(())
        }
        Err(e) => {
            error!("Failed to verify schema_migrations table: {}", e);
            Err(e)
        }
    }
}

pub fn check_migration_table_exists(conn_string: &str) -> Result<bool, ConnectionError> {
    debug!("Checking if schema_migrations table exists");

    let connection_manager = ConnectionManager::new()?;
    let connection = connection_manager.connect(conn_string)?;
    let mut executor = DatabaseExecutor::new(connection);

    // Try to query the table - if it fails, it probably doesn't exist
    match executor.query_single_value("SELECT COUNT(*) FROM schema_migrations") {
        Ok(_) => {
            debug!("schema_migrations table exists");
            Ok(true)
        }
        Err(_) => {
            debug!("schema_migrations table does not exist");
            Ok(false)
        }
    }
}
