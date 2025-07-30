use crate::executor::{ConnectionError, ConnectionManager};
use crate::orchestrator::{MigrationLoader, Validator};
use crate::tracker::{schema_init, VersionStore};
use log::{debug, error, info, warn};
use std::collections::HashMap;

pub fn run_validate(conn: &str, path: &str) -> Result<(), ValidateError> {
    info!("Running migration validation");
    debug!("Connection string length: {}", conn.len());
    debug!("Migrations path: {}", path);

    // Test connection first
    let connection_manager = ConnectionManager::new()?;
    connection_manager.test_connection(conn)?;
    info!("✅ Database connection verified");

    // Load migrations from filesystem
    let migrations = MigrationLoader::load_migrations(path)
        .map_err(|e| ValidateError::LoadFailed(e.to_string()))?;

    if migrations.is_empty() {
        info!("🔍 No migrations found in {}", path);
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
        info!("🔍 Migration Validation Results");
        info!("==============================");
        warn!("⚠️  schema_migrations table does not exist. Cannot validate against database.");
        info!("");
        info!("File-based validation:");
        info!("  📊 Total migrations: {}", migrations.len());
        
        let versioned_count = migrations.iter().filter(|m| !m.is_repeatable()).count();
        let repeatable_count = migrations.iter().filter(|m| m.is_repeatable()).count();
        info!("  📊 Versioned migrations: {}", versioned_count);
        info!("  📊 Repeatable migrations: {}", repeatable_count);
        
        for migration in &migrations {
            let migration_type_display = if migration.is_repeatable() { "R" } else { "V" };
            info!("  📄 [{}] {} - {} lines, checksum: {}...", 
                migration_type_display,
                migration.filename(), 
                migration.sql_content.lines().count(),
                &migration.checksum[..8]
            );
            debug!("      File: {}", migration.file_path.display());
        }
        return Ok(());
    }

    // Get applied migrations and versions
    let mut version_store = VersionStore::new(conn)?;
    let applied_migrations = version_store.get_applied_migrations()?;
    let applied_versions = version_store.get_applied_versions()?;
    
    // Create lookup maps
    let applied_map: HashMap<String, _> = applied_migrations
        .iter()
        .map(|m| (m.migration_id.clone(), m))
        .collect();

    info!("🔍 Migration Validation Results");
    info!("==============================");
    info!("Database: Connected ✅");
    info!("Total file migrations: {}", migrations.len());
    info!("Applied in database: {}", applied_migrations.len());
    info!("Applied versions: {:?}", applied_versions);
    info!("");

    let mut validation_errors = Vec::new();
    let mut checksum_mismatches = 0;
    let mut orphaned_db_migrations = 0;

    // Validate each file migration
    for migration in &migrations {
        let migration_type_display = if migration.is_repeatable() { "R" } else { "V" };
        
        match applied_map.get(&migration.identifier()) {
            Some(applied) => {
                let status_icon = if applied.success { "✅" } else { "❌" };
                info!(
                    "  {} [{}] {} (applied: {}, {}ms)",
                    status_icon,
                    migration_type_display,
                    migration.filename(),
                    applied.applied_at.format("%Y-%m-%d %H:%M:%S"),
                    applied.execution_time_ms
                );

                // Show detailed file information
                debug!("      File: {}", migration.file_path.display());
                debug!("      Lines: {}", migration.sql_content.lines().count());

                // Validate checksum integrity - compare both stored and applied data
                let stored_checksum = version_store.get_migration_checksum(&migration.identifier())?
                    .unwrap_or_else(|| applied.checksum.clone());
                
                if applied.checksum != migration.checksum || stored_checksum != migration.checksum {
                    checksum_mismatches += 1;
                    warn!(
                        "      ⚠️  CHECKSUM MISMATCH! File may have been modified after application."
                    );
                    warn!("         Applied record: {}", applied.checksum);
                    warn!("         Stored checksum: {}", stored_checksum);
                    warn!("         Current file: {}", migration.checksum);
                    validation_errors.push(format!(
                        "Checksum mismatch for {}: stored={}, current={}",
                        migration.filename(),
                        stored_checksum,
                        migration.checksum
                    ));
                } else {
                    debug!("      ✅ Checksum valid: {}", migration.checksum);
                }

                // Check for failed migrations
                if !applied.success {
                    validation_errors.push(format!(
                        "Migration {} failed during application",
                        migration.filename()
                    ));
                }
            }
            None => {
                info!("  ⏳ [{}] {} (PENDING)", migration_type_display, migration.filename());
                debug!("      File: {}", migration.file_path.display());
                debug!("      Lines: {}", migration.sql_content.lines().count());
                debug!("      Checksum: {}", migration.checksum);
            }
        }
    }

    // Check for orphaned database migrations (migrations in DB but not in files)
    for applied in &applied_migrations {
        let file_exists = migrations
            .iter()
            .any(|m| m.identifier() == applied.migration_id);
        
        if !file_exists {
            orphaned_db_migrations += 1;
            warn!(
                "  🚨 ORPHANED: {} exists in database but not in files", 
                applied.filename
            );
            validation_errors.push(format!(
                "Migration {} exists in database but corresponding file not found",
                applied.filename
            ));
        }
    }

    // Summary
    info!("");
    info!("📊 Validation Summary");
    info!("====================");
    info!("Total validation errors: {}", validation_errors.len());
    info!("Checksum mismatches: {}", checksum_mismatches);
    info!("Orphaned DB migrations: {}", orphaned_db_migrations);

    if validation_errors.is_empty() {
        info!("✅ All migrations validated successfully!");
    } else {
        error!("❌ Validation failed with {} errors:", validation_errors.len());
        for error in &validation_errors {
            error!("  - {}", error);
        }
        return Err(ValidateError::ValidationFailed(validation_errors));
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum ValidateError {
    #[error("Failed to load migrations: {0}")]
    LoadFailed(String),

    #[error("Connection error: {0}")]
    Connection(#[from] ConnectionError),

    #[error("Validation failed with {} errors", .0.len())]
    ValidationFailed(Vec<String>),
}