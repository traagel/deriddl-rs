use crate::executor::ConnectionError;
use crate::orchestrator::{MigrationLoader, Validator};
use crate::tracker::{schema_init, VersionStore};
use log::{debug, error, info, warn};
use std::collections::HashMap;

pub fn run_status(conn: &str, path: &str) -> Result<(), StatusError> {
    info!("Running migration status check");
    debug!("Connection string length: {}", conn.len());
    debug!("Migrations path: {}", path);

    // Load migrations from filesystem
    let migrations = MigrationLoader::load_migrations(path)
        .map_err(|e| StatusError::LoadFailed(e.to_string()))?;

    if migrations.is_empty() {
        info!("📊 No migrations found in {}", path);
        return Ok(());
    }

    info!("Loaded {} migrations from {}", migrations.len(), path);

    // Validate migration sequence
    let sequence_issues = Validator::validate_migration_sequence(&migrations);
    if !sequence_issues.is_empty() {
        warn!("Migration sequence issues found:");
        for issue in &sequence_issues {
            warn!("⚠️  {}", issue);
        }
    }

    // Check if schema_migrations table exists
    let table_exists = schema_init::check_migration_table_exists(conn)?;

    if !table_exists {
        info!("📊 Migration Status");
        info!("==================");
        warn!("⚠️  schema_migrations table does not exist. Run 'init' command first.");
        info!("");
        info!("Available migrations ({}): ", migrations.len());
        for migration in migrations {
            info!("  📄 {} (PENDING)", migration.filename());
        }
        return Ok(());
    }

    // Get applied migrations and baseline info
    let mut version_store = VersionStore::new(conn)?;
    let applied_migrations = version_store.get_applied_migrations()?;
    let applied_versions = version_store.get_applied_versions()?;
    let baseline_version = version_store.get_baseline_version()?;
    let applied_map: HashMap<String, _> =
        applied_migrations.iter().map(|m| (m.migration_id.clone(), m)).collect();

    // Display status
    info!("📊 Migration Status");
    info!("==================");
    info!("Database: Connected ✅");
    info!("Total migrations: {}", migrations.len());
    info!("Applied: {}", applied_migrations.len());
    info!("Pending: {}", migrations.len() - applied_migrations.len());
    
    // Show baseline information
    if let Some(baseline) = baseline_version {
        info!("Baseline version: {} 🏁", baseline);
        let skipped_count = migrations.iter()
            .filter(|m| if let Some(v) = m.version { v <= baseline } else { false })
            .count();
        if skipped_count > 0 {
            info!("Migrations below baseline: {} (skipped)", skipped_count);
        }
    } else {
        info!("Baseline: Not set");
    }
    
    // Show version statistics for versioned migrations
    if !applied_versions.is_empty() {
        info!("Latest applied version: {}", applied_versions.iter().max().unwrap());
        debug!("Applied versions: {:?}", applied_versions);
    }
    info!("");

    // Show each migration status
    for migration in &migrations {
        match applied_map.get(&migration.identifier()) {
            Some(applied) => {
                // Create a full Migration object with applied data for richer information
                let migration_with_applied = crate::model::Migration::from_applied(
                    applied, 
                    migration.file_path.clone(), 
                    migration.sql_content.clone()
                );
                
                let status_icon = if migration_with_applied.is_applied() { "✅" } else { "❌" };
                let migration_type_display = match applied.migration_type {
                    crate::model::MigrationType::Versioned => "V",
                    crate::model::MigrationType::Repeatable => "R",
                };
                
                let timing_info = if let Some(exec_time) = migration_with_applied.execution_time() {
                    format!("{}ms", exec_time)
                } else {
                    "unknown".to_string()
                };
                
                info!(
                    "  {} [{}] {} (applied: {}, {})", 
                    status_icon,
                    migration_type_display,
                    migration.filename(),
                    applied.applied_at.format("%Y-%m-%d %H:%M:%S"),
                    timing_info
                );
                
                // Show file path for detailed info
                debug!("      File: {}", migration.file_path.display());
                
                // Show applied timestamp if available  
                if let Some(applied_time) = migration_with_applied.applied_timestamp() {
                    debug!("      Applied at: {}", applied_time.format("%Y-%m-%d %H:%M:%S UTC"));
                }

                // Check for checksum mismatch using the applied migration data
                if applied.checksum != migration.checksum {
                    warn!("      ⚠️  Checksum mismatch! File may have been modified after application.");
                    debug!("         Stored: {}, Current: {}", applied.checksum, migration.checksum);
                }
            }
            None => {
                let migration_type_display = match migration.migration_type {
                    crate::model::MigrationType::Versioned => "V",
                    crate::model::MigrationType::Repeatable => "R",
                };
                info!("  ⏳ [{}] {} (PENDING)", migration_type_display, migration.filename());
                debug!("      File: {}", migration.file_path.display());
            }
        }
    }

    // Show any failed migrations
    let failed_migrations: Vec<_> = applied_migrations.iter().filter(|m| !m.success).collect();

    if !failed_migrations.is_empty() {
        info!("");
        warn!("❌ Failed Migrations:");
        for failed in failed_migrations {
            warn!(
                "  {} (version {}, failed at: {})",
                failed.filename,
                failed.version.map_or("R__".to_string(), |v| v.to_string()),
                failed.applied_at.format("%Y-%m-%d %H:%M:%S")
            );
        }
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum StatusError {
    #[error("Failed to load migrations: {0}")]
    LoadFailed(String),

    #[error("Connection error: {0}")]
    Connection(#[from] ConnectionError),
}

