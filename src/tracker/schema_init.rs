use crate::executor::{ConnectionManager, ConnectionError, DatabaseExecutor};
use log::{info, debug, error};

const SCHEMA_MIGRATIONS_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY NOT NULL,
    filename VARCHAR(255) NOT NULL,
    checksum VARCHAR(64) NOT NULL,
    applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    execution_time_ms INTEGER NOT NULL,
    success BOOLEAN NOT NULL DEFAULT TRUE
)
"#;

const SCHEMA_MIGRATIONS_TABLE_SQL_POSTGRES: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY NOT NULL,
    filename VARCHAR(255) NOT NULL,
    checksum VARCHAR(64) NOT NULL,
    applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    execution_time_ms INTEGER NOT NULL,
    success BOOLEAN NOT NULL DEFAULT TRUE
)
"#;

const SCHEMA_MIGRATIONS_TABLE_SQL_MYSQL: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY NOT NULL,
    filename VARCHAR(255) NOT NULL,
    checksum VARCHAR(64) NOT NULL,
    applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    execution_time_ms INTEGER NOT NULL,
    success BOOLEAN NOT NULL DEFAULT TRUE
)
"#;

const SCHEMA_MIGRATIONS_TABLE_SQL_SQLITE: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY NOT NULL,
    filename TEXT NOT NULL,
    checksum TEXT NOT NULL,
    applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    execution_time_ms INTEGER NOT NULL,
    success BOOLEAN NOT NULL DEFAULT 1
)
"#;

pub fn init_migration_table(conn_string: &str) -> Result<(), ConnectionError> {
    info!("Initializing schema_migrations table");
    debug!("Connection string length: {}", conn_string.len());
    
    let connection_manager = ConnectionManager::new()?;
    let connection = connection_manager.connect(conn_string)?;
    let mut executor = DatabaseExecutor::new(connection);
    
    // Try to detect database type from connection string and use appropriate SQL
    let create_table_sql = detect_database_type_and_get_sql(conn_string);
    
    debug!("Creating schema_migrations table");
    executor.execute_query(create_table_sql)?;
    
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

fn detect_database_type_and_get_sql(conn_string: &str) -> &'static str {
    let conn_lower = conn_string.to_lowercase();
    
    if conn_lower.contains("postgresql") || conn_lower.contains("postgres") {
        debug!("Detected PostgreSQL database");
        SCHEMA_MIGRATIONS_TABLE_SQL_POSTGRES
    } else if conn_lower.contains("mysql") || conn_lower.contains("mariadb") {
        debug!("Detected MySQL/MariaDB database");
        SCHEMA_MIGRATIONS_TABLE_SQL_MYSQL
    } else if conn_lower.contains("sqlite") {
        debug!("Detected SQLite database");
        SCHEMA_MIGRATIONS_TABLE_SQL_SQLITE
    } else {
        debug!("Using generic SQL for unknown database type");
        SCHEMA_MIGRATIONS_TABLE_SQL
    }
}