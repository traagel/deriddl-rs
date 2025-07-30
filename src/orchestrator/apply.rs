use crate::orchestrator::{MigrationLoader, Validator};
use crate::tracker::{schema_init, VersionStore};
use crate::executor::{ConnectionManager, DatabaseExecutor, ConnectionError};
use log::{info, debug, error};
use std::time::Instant;

pub fn run_apply(conn: &str, path: &str, dry_run: bool) -> Result<(), ApplyError> {
    info!("Running migration apply");
    debug!("Connection string length: {}", conn.len());
    debug!("Migrations path: {}", path);
    debug!("Dry run mode: {}", dry_run);
    
    // Load migrations
    let migrations = MigrationLoader::load_migrations(path)
        .map_err(|e| ApplyError::LoadFailed(e.to_string()))?;
        
    if migrations.is_empty() {
        info!("No migrations found in {}", path);
        return Ok(());
    }
    
    info!("Loaded {} migrations", migrations.len());
    
    // Validate migration sequence
    let validation_issues = Validator::validate_migration_sequence(&migrations);
    if !validation_issues.is_empty() {
        error!("Migration validation failed:");
        for issue in &validation_issues {
            error!("  - {}", issue);
        }
        return Err(ApplyError::ValidationFailed(validation_issues));
    }
    
    // Test connection first
    let connection_manager = ConnectionManager::new()?;
    connection_manager.test_connection(conn)
        .map_err(ApplyError::Connection)?;
    info!("✅ Database connection verified");
    
    // Ensure schema_migrations table exists
    if !schema_init::check_migration_table_exists(conn)? {
        info!("schema_migrations table does not exist, creating it");
        schema_init::init_migration_table(conn)?;
    }
    
    // Get pending migrations
    let mut version_store = VersionStore::new(conn)?;
    let pending_migrations = version_store.get_pending_migrations(&migrations)?;
    
    if pending_migrations.is_empty() {
        info!("✅ No pending migrations to apply");
        return Ok(());
    }
    
    info!("Found {} pending migrations", pending_migrations.len());
    
    if dry_run {
        return run_dry_run(&pending_migrations);
    }
    
    // Apply migrations
    apply_migrations(conn, &pending_migrations)
}

fn run_dry_run(pending_migrations: &[crate::model::Migration]) -> Result<(), ApplyError> {
    info!("🔍 DRY RUN: Would apply {} migrations", pending_migrations.len());
    
    for migration in pending_migrations {
        info!("  📄 {} - {}", migration.filename(), migration.sql_content.lines().count());
        debug!("Migration SQL preview: {}", 
            migration.sql_content.chars().take(100).collect::<String>());
    }
    
    info!("✅ Dry run completed successfully");
    Ok(())
}

fn apply_migrations(conn: &str, migrations: &[crate::model::Migration]) -> Result<(), ApplyError> {
    info!("🚀 Applying {} migrations", migrations.len());
    
    let connection_manager = ConnectionManager::new()?;
    let connection = connection_manager.connect(conn)?;
    let mut executor = DatabaseExecutor::new(connection);
    let mut version_store = VersionStore::new(conn)?;
    
    for migration in migrations {
        info!("Applying migration: {}", migration.filename());
        
        let start_time = Instant::now();
        
        // Record migration start
        version_store.record_migration_start(migration)?;
        
        // Execute migration in a transaction
        let result = executor.execute_transaction(|exec| {
            exec.execute_query(&migration.sql_content)
                .map_err(|e| ConnectionError::QueryFailed(format!("Migration {}: {}", migration.filename(), e)))
        });
        
        let execution_time = start_time.elapsed().as_millis() as i32;
        
        match result {
            Ok(()) => {
                version_store.record_migration_success(migration, execution_time)?;
                info!("✅ Migration {} applied successfully in {}ms", 
                    migration.filename(), execution_time);
            }
            Err(e) => {
                version_store.record_migration_failure(migration, execution_time)?;
                error!("❌ Migration {} failed: {}", migration.filename(), e);
                return Err(ApplyError::MigrationFailed(migration.filename(), e.to_string()));
            }
        }
    }
    
    info!("🎉 All {} migrations applied successfully!", migrations.len());
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    #[error("Failed to load migrations: {0}")]
    LoadFailed(String),
    
    #[error("Migration validation failed: {0:?}")]
    ValidationFailed(Vec<String>),
    
    #[error("Connection error: {0}")]
    Connection(#[from] ConnectionError),
    
    #[error("Migration {0} failed: {1}")]
    MigrationFailed(String, String),
}