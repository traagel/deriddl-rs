use log::{debug, error, info};
use odbc_api::{
    buffers::TextRowSet, Connection, ConnectionOptions, Cursor, Environment, Error as OdbcError,
};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("ODBC error: {0}")]
    Odbc(#[from] OdbcError),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Query execution failed: {0}")]
    QueryFailed(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
}

pub struct ConnectionManager {
    environment: Arc<Environment>,
}

impl ConnectionManager {
    pub fn new() -> Result<Self, ConnectionError> {
        let environment = Environment::new()?;
        Ok(Self {
            environment: Arc::new(environment),
        })
    }

    pub fn connect(&self, connection_string: &str) -> Result<Connection<'_>, ConnectionError> {
        debug!(
            "Connecting to database with connection string length: {}",
            connection_string.len()
        );

        let connection = self
            .environment
            .connect_with_connection_string(connection_string, ConnectionOptions::default())
            .map_err(|e| {
                error!("Failed to connect to database: {}", e);
                ConnectionError::ConnectionFailed(e.to_string())
            })?;

        info!("Successfully connected to database");
        Ok(connection)
    }

    pub fn test_connection(&self, connection_string: &str) -> Result<(), ConnectionError> {
        debug!("Testing database connection");
        let connection = self.connect(connection_string)?;

        // Test with a simple query
        let query = "SELECT 1 as test_column";
        let mut prepared = connection
            .prepare(query)
            .map_err(|e| ConnectionError::QueryFailed(e.to_string()))?;

        let mut cursor = prepared
            .execute(())
            .map_err(|e| ConnectionError::QueryFailed(e.to_string()))?
            .unwrap();

        // Fetch one row to verify the connection works
        let mut buffer = TextRowSet::for_cursor(1, &mut cursor, Some(4096))?;
        let mut row_set_cursor = cursor.bind_buffer(&mut buffer)?;
        let _row_set = row_set_cursor.fetch()?;

        info!("Database connection test successful");
        Ok(())
    }
}

pub struct DatabaseExecutor<'a> {
    connection: Connection<'a>,
}

impl<'a> DatabaseExecutor<'a> {
    pub fn new(connection: Connection<'a>) -> Self {
        Self { connection }
    }

    fn split_sql_statements(sql: &str) -> Vec<String> {
        sql.lines()
            .map(str::trim)
            .filter(|line| !line.starts_with("--") && !line.is_empty())
            .collect::<Vec<&str>>()
            .join(" ")
            .split(';')
            .map(str::trim)
            .filter(|stmt| !stmt.is_empty())
            .map(String::from)
            .collect()
    }

    pub fn execute_query(&mut self, query: &str) -> Result<(), ConnectionError> {
        debug!("Executing query block");

        for stmt in Self::split_sql_statements(query) {
            let stmt_ref: &str = stmt.as_str();
            debug!("Executing SQL statement: {}", stmt);

            let mut prepared = self
                .connection
                .prepare(stmt_ref)
                .map_err(|e| ConnectionError::QueryFailed(e.to_string()))?;

            match prepared.execute(()) {
                Ok(Some(mut cursor)) => {
                    let mut buffer = TextRowSet::for_cursor(100, &mut cursor, Some(4096))?;
                    let mut row_set_cursor = cursor.bind_buffer(&mut buffer)?;
                    while row_set_cursor.fetch()?.is_some() {
                        // Consume results
                    }
                    debug!("Statement executed successfully with results");
                }
                Ok(None) => {
                    debug!("Statement executed successfully (no results)");
                }
                Err(e) => {
                    error!("Statement execution failed: {}", e);
                    return Err(ConnectionError::QueryFailed(e.to_string()));
                }
            }
        }

        Ok(())
    }

    pub fn execute_transaction<F>(&mut self, operations: F) -> Result<(), ConnectionError>
    where
        F: FnOnce(&mut Self) -> Result<(), ConnectionError>,
    {
        debug!("Starting transaction");

        // Begin transaction (most databases auto-commit by default)
        self.execute_query("BEGIN TRANSACTION").or_else(|_| {
            // Some databases use different syntax
            self.execute_query("START TRANSACTION").or_else(|_| {
                // PostgreSQL and others might not need explicit BEGIN for single statements
                debug!("Could not start explicit transaction, proceeding with auto-commit");
                Ok::<(), ConnectionError>(())
            })
        })?;

        match operations(self) {
            Ok(()) => {
                debug!("Transaction operations completed, committing");
                self.execute_query("COMMIT").or_else(|_| {
                    debug!("Explicit COMMIT failed, relying on auto-commit");
                    Ok::<(), ConnectionError>(())
                })?;
                info!("Transaction committed successfully");
                Ok(())
            }
            Err(e) => {
                error!("Transaction operations failed: {}, rolling back", e);
                self.execute_query("ROLLBACK").or_else(|_| {
                    debug!("Explicit ROLLBACK failed, relying on auto-rollback");
                    Ok::<(), ConnectionError>(())
                })?;
                Err(ConnectionError::TransactionFailed(e.to_string()))
            }
        }
    }

    pub fn query_single_value(&mut self, query: &str) -> Result<Option<String>, ConnectionError> {
        debug!("Querying single value: {}", query);

        let mut prepared = self
            .connection
            .prepare(query)
            .map_err(|e| ConnectionError::QueryFailed(e.to_string()))?;

        let mut cursor = prepared
            .execute(())
            .map_err(|e| ConnectionError::QueryFailed(e.to_string()))?
            .ok_or_else(|| ConnectionError::QueryFailed("Query returned no cursor".to_string()))?;

        let mut buffer = TextRowSet::for_cursor(1, &mut cursor, Some(4096))?;
        let mut row_set_cursor = cursor.bind_buffer(&mut buffer)?;

        if let Some(row_set) = row_set_cursor.fetch()? {
            if row_set.num_rows() > 0 {
                if let Some(value) = row_set.at(0, 0) {
                    let result = String::from_utf8_lossy(value).to_string();
                    debug!("Query returned single value: {}", result);
                    return Ok(Some(result));
                }
            }
        }

        debug!("Query returned no value");
        Ok(None)
    }

    pub fn query_rows(&mut self, query: &str) -> Result<Vec<Vec<String>>, ConnectionError> {
        debug!("Querying multiple rows: {}", query);

        let mut prepared = self
            .connection
            .prepare(query)
            .map_err(|e| ConnectionError::QueryFailed(e.to_string()))?;

        let mut cursor = prepared
            .execute(())
            .map_err(|e| ConnectionError::QueryFailed(e.to_string()))?
            .ok_or_else(|| ConnectionError::QueryFailed("Query returned no cursor".to_string()))?;

        let mut buffer = TextRowSet::for_cursor(100, &mut cursor, Some(4096))?;
        let mut row_set_cursor = cursor.bind_buffer(&mut buffer)?;
        let mut results = Vec::new();

        while let Some(row_set) = row_set_cursor.fetch()? {
            for row_index in 0..row_set.num_rows() {
                let mut row = Vec::new();
                for col_index in 0..row_set.num_cols() {
                    let value = row_set
                        .at(col_index, row_index)
                        .map(|v| String::from_utf8_lossy(v).to_string())
                        .unwrap_or_else(|| "NULL".to_string());
                    row.push(value);
                }
                results.push(row);
            }
        }

        debug!("Query returned {} rows", results.len());
        Ok(results)
    }
}

