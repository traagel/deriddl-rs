use crate::orchestrator::{MigrationLoader, Validator};
use crate::tracker::{schema_init, VersionStore};
use crate::executor::ConnectionError;
use log::{info, debug, warn, error};
use std::collections::HashMap;

pub fn run_status(conn: &str, path: &str) -> Result<(), StatusError> {
    info!("Running migration status check");
    debug!("Connection string length: {}", conn.len());
    debug!("Migrations path: {}", path);
    
    // Load migrations from filesystem
    let migrations = MigrationLoader::load_migrations(path)
        .map_err(|e| StatusError::LoadFailed(e.to_string()))?;
        
    if migrations.is_empty() {
        println!("📊 No migrations found in {}", path);
        return Ok(());
    }
    
    info!("Loaded {} migrations from {}", migrations.len(), path);
    
    // Validate migration sequence
    let sequence_issues = Validator::validate_migration_sequence(&migrations);
    if !sequence_issues.is_empty() {
        warn!("Migration sequence issues found:");
        for issue in &sequence_issues {
            println!("⚠️  {}", issue);
        }
    }
    
    // Check if schema_migrations table exists
    let table_exists = schema_init::check_migration_table_exists(conn)?;
    
    if !table_exists {
        println!("📊 Migration Status");
        println!("==================");
        println!("⚠️  schema_migrations table does not exist. Run 'init' command first.");
        println!();
        println!("Available migrations ({}): ", migrations.len());
        for migration in migrations {
            println!("  📄 {} (PENDING)", migration.filename());
        }
        return Ok(());
    }
    
    // Get applied migrations
    let mut version_store = VersionStore::new(conn)?;
    let applied_migrations = version_store.get_applied_migrations()?;
    let applied_versions: HashMap<u32, _> = applied_migrations
        .iter()
        .map(|m| (m.version, m))
        .collect();
    
    // Display status
    println!("📊 Migration Status");
    println!("==================");
    println!("Database: Connected ✅");
    println!("Total migrations: {}", migrations.len());
    println!("Applied: {}", applied_migrations.len());
    println!("Pending: {}", migrations.len() - applied_migrations.len());
    println!();
    
    // Show each migration status
    for migration in &migrations {
        match applied_versions.get(&migration.version) {
            Some(applied) => {
                let status_icon = if applied.success { "✅" } else { "❌" };
                println!("  {} {} (applied: {}, {}ms)", 
                    status_icon,
                    migration.filename(),
                    applied.applied_at.format("%Y-%m-%d %H:%M:%S"),
                    applied.execution_time_ms
                );
                
                // Check for checksum mismatch
                if applied.checksum != migration.checksum() {
                    println!("      ⚠️  Checksum mismatch! File may have been modified after application.");
                }
            }
            None => {
                println!("  ⏳ {} (PENDING)", migration.filename());
            }
        }
    }
    
    // Show any failed migrations
    let failed_migrations: Vec<_> = applied_migrations
        .iter()
        .filter(|m| !m.success)
        .collect();
        
    if !failed_migrations.is_empty() {
        println!();
        println!("❌ Failed Migrations:");
        for failed in failed_migrations {
            println!("  {} (version {}, failed at: {})", 
                failed.filename,
                failed.version,
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