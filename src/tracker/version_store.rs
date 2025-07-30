use crate::executor::{ConnectionError, ConnectionManager, DatabaseExecutor};
use crate::model::{Migration, MigrationType};
use chrono::{DateTime, Utc};
use log::{debug, info};

#[derive(Debug, Clone)]
pub struct AppliedMigration {
    pub migration_id: String,
    pub migration_type: MigrationType,
    pub version: Option<u32>,
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
            SELECT migration_id, migration_type, version, filename, checksum, applied_at, execution_time_ms, success
            FROM schema_migrations 
            ORDER BY 
                CASE WHEN migration_type = 'versioned' THEN 0 ELSE 1 END,
                CASE WHEN migration_type = 'versioned' THEN version ELSE 0 END,
                filename
        "#;

        let mut executor = self.get_executor()?;
        let rows = executor.query_rows(query)?;
        let mut migrations = Vec::new();

        for row in rows {
            if row.len() >= 8 {
                let migration_type = match row[1].as_str() {
                    "repeatable" => MigrationType::Repeatable,
                    _ => MigrationType::Versioned,
                };
                
                let version = if migration_type == MigrationType::Versioned {
                    Some(row[2].parse().unwrap_or(0))
                } else {
                    None
                };

                let migration = AppliedMigration {
                    migration_id: row[0].clone(),
                    migration_type,
                    version,
                    filename: row[3].clone(),
                    checksum: row[4].clone(),
                    applied_at: parse_timestamp(&row[5]),
                    execution_time_ms: row[6].parse().unwrap_or(0),
                    success: parse_boolean(&row[7]),
                };
                migrations.push(migration);
            }
        }

        debug!("Found {} applied migrations", migrations.len());
        Ok(migrations)
    }

    pub fn get_applied_versions(&mut self) -> Result<Vec<u32>, ConnectionError> {
        debug!("Fetching applied migration versions");

        let query = "SELECT version FROM schema_migrations WHERE migration_type = 'versioned' AND success = 1 ORDER BY version ASC";
        let mut executor = self.get_executor()?;
        let rows = executor.query_rows(query)?;

        let versions: Vec<u32> = rows
            .into_iter()
            .filter_map(|row| row.first()?.parse().ok())
            .collect();

        debug!("Found {} applied versions", versions.len());
        Ok(versions)
    }

    pub fn is_migration_applied(&mut self, version: u32) -> Result<bool, ConnectionError> {
        debug!("Checking if migration version {} is applied", version);

        let query_with_param = format!(
            "SELECT COUNT(*) FROM schema_migrations WHERE migration_type = 'versioned' AND version = {} AND success = 1",
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
    
    /// Check if a repeatable migration needs to be re-run (checksum has changed or never run)
    pub fn should_run_repeatable(&mut self, migration: &Migration) -> Result<bool, ConnectionError> {
        debug!("Checking if repeatable migration '{}' needs to run", migration.name);
        
        let query = format!(
            "SELECT checksum FROM schema_migrations WHERE migration_id = '{}' AND success = 1",
            migration.identifier().replace("'", "''")
        );

        let mut executor = self.get_executor()?;
        match executor.query_single_value(&query)? {
            Some(stored_checksum) => {
                let should_run = stored_checksum != migration.checksum;
                debug!("Repeatable migration '{}' checksum changed: {}", migration.name, should_run);
                Ok(should_run)
            }
            None => {
                debug!("Repeatable migration '{}' never run before", migration.name);
                Ok(true) // Never run before, should run
            }
        }
    }

    pub fn record_migration_start(&mut self, migration: &Migration) -> Result<(), ConnectionError> {
        debug!(
            "Recording migration start for '{}'", 
            migration.identifier()
        );

        let migration_type_str = match migration.migration_type {
            MigrationType::Versioned => "versioned",
            MigrationType::Repeatable => "repeatable",
        };

        let version_value = match migration.version {
            Some(v) => v.to_string(),
            None => "NULL".to_string(),
        };

        // For repeatable migrations, delete any existing record first
        if migration.is_repeatable() {
            let delete_query = format!(
                "DELETE FROM schema_migrations WHERE migration_id = '{}'",
                migration.identifier().replace("'", "''")
            );
            let mut executor = self.get_executor()?;
            let _ = executor.execute_query(&delete_query); // Ignore errors if record doesn't exist
        }

        let query = format!(
            "INSERT INTO schema_migrations (migration_id, migration_type, version, filename, checksum, applied_at, execution_time_ms, success) VALUES ('{}', '{}', {}, '{}', '{}', CURRENT_TIMESTAMP, 0, 0)",
            migration.identifier().replace("'", "''"),
            migration_type_str,
            version_value,
            migration.filename().replace("'", "''"),
            migration.checksum.replace("'", "''")
        );

        let mut executor = self.get_executor()?;
        executor.execute_query(&query)?;
        debug!("Migration start recorded for '{}'", migration.identifier());
        Ok(())
    }

    pub fn record_migration_success(
        &mut self,
        migration: &Migration,
        execution_time_ms: i32,
    ) -> Result<(), ConnectionError> {
        debug!(
            "Recording migration success for '{}' ({}ms)",
            migration.identifier(), execution_time_ms
        );

        let query = format!(
            "UPDATE schema_migrations SET execution_time_ms = {}, success = 1, applied_at = CURRENT_TIMESTAMP WHERE migration_id = '{}'",
            execution_time_ms,
            migration.identifier().replace("'", "''")
        );

        let mut executor = self.get_executor()?;
        executor.execute_query(&query)?;
        info!(
            "✅ Migration '{}' completed successfully in {}ms",
            migration.identifier(), execution_time_ms
        );
        Ok(())
    }

    pub fn record_migration_failure(
        &mut self,
        migration: &Migration,
        execution_time_ms: i32,
    ) -> Result<(), ConnectionError> {
        debug!(
            "Recording migration failure for '{}' ({}ms)",
            migration.identifier(), execution_time_ms
        );

        let query = format!(
            "UPDATE schema_migrations SET execution_time_ms = {}, success = 0 WHERE migration_id = '{}'",
            execution_time_ms, migration.identifier().replace("'", "''")
        );

        let mut executor = self.get_executor()?;
        executor.execute_query(&query)?;
        debug!("Migration '{}' failure recorded", migration.identifier());
        Ok(())
    }

    pub fn get_migration_checksum(
        &mut self,
        migration_id: &str,
    ) -> Result<Option<String>, ConnectionError> {
        debug!("Getting checksum for migration '{}'", migration_id);

        let query = format!(
            "SELECT checksum FROM schema_migrations WHERE migration_id = '{}'",
            migration_id.replace("'", "''")
        );
        let mut executor = self.get_executor()?;
        executor.query_single_value(&query)
    }

    pub fn get_pending_migrations(
        &mut self,
        all_migrations: &[Migration],
    ) -> Result<Vec<Migration>, ConnectionError> {
        let mut pending = Vec::new();
        
        for migration in all_migrations {
            match migration.migration_type {
                MigrationType::Versioned => {
                    // For versioned migrations, check if already applied
                    if let Some(version) = migration.version {
                        if !self.is_migration_applied(version)? {
                            pending.push(migration.clone());
                        }
                    }
                }
                MigrationType::Repeatable => {
                    // For repeatable migrations, check if checksum changed or never run
                    if self.should_run_repeatable(migration)? {
                        pending.push(migration.clone());
                    }
                }
            }
        }

        let versioned_pending = pending.iter().filter(|m| !m.is_repeatable()).count();
        let repeatable_pending = pending.iter().filter(|m| m.is_repeatable()).count();
        debug!(
            "Found {} pending migrations ({} versioned, {} repeatable)", 
            pending.len(), versioned_pending, repeatable_pending
        );
        
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
    matches!(bool_str.to_lowercase().as_str(), "true" | "1" | "t" | "yes" | "y")
}

