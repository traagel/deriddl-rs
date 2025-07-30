use crate::executor::{ConnectionError, ConnectionManager, DatabaseExecutor};
use crate::model::Migration;
use chrono::{DateTime, Utc};
use log::{debug, info};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AppliedMigration {
    pub version: u32,
    pub filename: String,
    pub checksum: String,
    pub applied_at: DateTime<Utc>,
    pub execution_time_ms: i32,
    pub success: bool,
}

pub struct VersionStore {
    connection_string: String,
    connection_manager: ConnectionManager,
}

impl VersionStore {
    pub fn new(conn_string: &str) -> Result<Self, ConnectionError> {
        let connection_manager = ConnectionManager::new()?;
        Ok(Self {
            connection_string: conn_string.to_string(),
            connection_manager,
        })
    }

    fn get_executor(&self) -> Result<DatabaseExecutor, ConnectionError> {
        let connection = self.connection_manager.connect(&self.connection_string)?;
        Ok(DatabaseExecutor::new(connection))
    }

    pub fn get_applied_migrations(&mut self) -> Result<Vec<AppliedMigration>, ConnectionError> {
        debug!("Fetching applied migrations from database");

        let query = r#"
            SELECT version, filename, checksum, applied_at, execution_time_ms, success
            FROM schema_migrations 
            ORDER BY version ASC
        "#;

        let mut executor = self.get_executor()?;
        let rows = executor.query_rows(query)?;
        let mut migrations = Vec::new();

        for row in rows {
            if row.len() >= 6 {
                let migration = AppliedMigration {
                    version: row[0].parse().unwrap_or(0),
                    filename: row[1].clone(),
                    checksum: row[2].clone(),
                    applied_at: parse_timestamp(&row[3]),
                    execution_time_ms: row[4].parse().unwrap_or(0),
                    success: parse_boolean(&row[5]),
                };
                migrations.push(migration);
            }
        }

        debug!("Found {} applied migrations", migrations.len());
        Ok(migrations)
    }

    pub fn get_applied_versions(&mut self) -> Result<Vec<u32>, ConnectionError> {
        debug!("Fetching applied migration versions");

        let query = "SELECT version FROM schema_migrations WHERE success = 1 ORDER BY version ASC";
        let mut executor = self.get_executor()?;
        let rows = executor.query_rows(query)?;

        let versions: Vec<u32> = rows
            .into_iter()
            .filter_map(|row| row.get(0)?.parse().ok())
            .collect();

        debug!("Found {} applied versions", versions.len());
        Ok(versions)
    }

    pub fn is_migration_applied(&mut self, version: u32) -> Result<bool, ConnectionError> {
        debug!("Checking if migration version {} is applied", version);

        let query_with_param = format!(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = {} AND success = 1",
            version
        );

        let mut executor = self.get_executor()?;
        match executor.query_single_value(&query_with_param)? {
            Some(count) => {
                let is_applied = count.parse::<i32>().unwrap_or(0) > 0;
                debug!("Migration {} is applied: {}", version, is_applied);
                Ok(is_applied)
            }
            None => Ok(false),
        }
    }

    pub fn record_migration_start(&mut self, migration: &Migration) -> Result<(), ConnectionError> {
        debug!(
            "Recording migration start for version {}",
            migration.version
        );

        let query = format!(
            "INSERT INTO schema_migrations (version, filename, checksum, applied_at, execution_time_ms, success) VALUES ({}, '{}', '{}', CURRENT_TIMESTAMP, 0, 0)",
            migration.version,
            migration.filename().replace("'", "''"), // Basic SQL injection protection
            migration.checksum.replace("'", "''")
        );

        let mut executor = self.get_executor()?;
        executor.execute_query(&query)?;
        debug!("Migration start recorded for version {}", migration.version);
        Ok(())
    }

    pub fn record_migration_success(
        &mut self,
        migration: &Migration,
        execution_time_ms: i32,
    ) -> Result<(), ConnectionError> {
        debug!(
            "Recording migration success for version {} ({}ms)",
            migration.version, execution_time_ms
        );

        let query = format!(
            "UPDATE schema_migrations SET execution_time_ms = {}, success = 1, applied_at = CURRENT_TIMESTAMP WHERE version = {}",
            execution_time_ms,
            migration.version
        );

        let mut executor = self.get_executor()?;
        executor.execute_query(&query)?;
        info!(
            "✅ Migration {} completed successfully in {}ms",
            migration.version, execution_time_ms
        );
        Ok(())
    }

    pub fn record_migration_failure(
        &mut self,
        migration: &Migration,
        execution_time_ms: i32,
    ) -> Result<(), ConnectionError> {
        debug!(
            "Recording migration failure for version {} ({}ms)",
            migration.version, execution_time_ms
        );

        let query = format!(
            "UPDATE schema_migrations SET execution_time_ms = {}, success = 0 WHERE version = {}",
            execution_time_ms, migration.version
        );

        let mut executor = self.get_executor()?;
        executor.execute_query(&query)?;
        debug!("Migration {} failure recorded", migration.version);
        Ok(())
    }

    pub fn get_migration_checksum(
        &mut self,
        version: u32,
    ) -> Result<Option<String>, ConnectionError> {
        debug!("Getting checksum for migration version {}", version);

        let query = format!(
            "SELECT checksum FROM schema_migrations WHERE version = {}",
            version
        );
        let mut executor = self.get_executor()?;
        executor.query_single_value(&query)
    }

    pub fn get_pending_migrations(
        &mut self,
        all_migrations: &[Migration],
    ) -> Result<Vec<Migration>, ConnectionError> {
        let applied_versions = self.get_applied_versions()?;
        let applied_set: HashMap<u32, bool> =
            applied_versions.into_iter().map(|v| (v, true)).collect();

        let pending: Vec<Migration> = all_migrations
            .iter()
            .filter(|migration| !applied_set.contains_key(&migration.version))
            .cloned()
            .collect();

        debug!("Found {} pending migrations", pending.len());
        Ok(pending)
    }
}

fn parse_timestamp(timestamp_str: &str) -> DateTime<Utc> {
    // Try to parse various timestamp formats
    DateTime::parse_from_rfc3339(timestamp_str)
        .or_else(|_| DateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S"))
        .or_else(|_| DateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S%.f"))
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn parse_boolean(bool_str: &str) -> bool {
    match bool_str.to_lowercase().as_str() {
        "true" | "1" | "t" | "yes" | "y" => true,
        _ => false,
    }
}

