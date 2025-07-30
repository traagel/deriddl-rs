use crate::executor::ConnectionError;
use crate::orchestrator::MigrationLoader;
use crate::tracker::{schema_init, VersionStore};
use log::{debug, info, warn};

pub fn run_plan(conn: &str, path: &str) -> Result<(), PlanError> {
    info!("Running migration plan");
    debug!("Connection string length: {}", conn.len());
    debug!("Migrations path: {}", path);

    // Load migrations from filesystem
    let migrations =
        MigrationLoader::load_migrations(path).map_err(|e| PlanError::LoadFailed(e.to_string()))?;

    if migrations.is_empty() {
        info!("📋 No migrations found in {}", path);
        return Ok(());
    }

    // Test connection first
    let connection_manager = crate::executor::ConnectionManager::new()?;
    connection_manager.test_connection(conn)?;
    debug!("Database connection verified");
    
    // Check if schema_migrations table exists
    let table_exists = schema_init::check_migration_table_exists(conn)?;

    if !table_exists {
        info!("📋 Migration Plan");
        info!("================");
        warn!("⚠️  schema_migrations table does not exist. All migrations will be applied.");
        info!("");
        info!("Migrations to apply ({}):", migrations.len());
        for (i, migration) in migrations.iter().enumerate() {
            info!(
                "{}. 📄 {} ({} lines)",
                i + 1,
                migration.filename(),
                migration.sql_content.lines().count()
            );
        }
        return Ok(());
    }

    // Get pending migrations
    let mut version_store = VersionStore::new(conn)?;
    let pending_migrations = version_store.get_pending_migrations(&migrations)?;

    info!("📋 Migration Plan");
    info!("================");

    if pending_migrations.is_empty() {
        info!("✅ No pending migrations to apply. Database is up to date!");
        return Ok(());
    }

    info!("Pending migrations ({}):", pending_migrations.len());
    info!("");

    for (i, migration) in pending_migrations.iter().enumerate() {
        info!("{}. 📄 {}", i + 1, migration.filename());
        match migration.version {
            Some(v) => info!("   Version: {}", v),
            None => info!("   Type: Repeatable"),
        }
        info!("   File: {}", migration.file_path.display());
        info!("   Lines: {}", migration.sql_content.lines().count());
        info!("   Checksum: {}...", &migration.checksum[..8]);

        // Show SQL preview (first few lines)
        let sql_lines: Vec<&str> = migration.sql_content.lines().take(3).collect();
        if !sql_lines.is_empty() {
            info!("   Preview:");
            for line in sql_lines {
                if !line.trim().is_empty() {
                    info!("     {}", line.chars().take(60).collect::<String>());
                }
            }
            if migration.sql_content.lines().count() > 3 {
                info!("     ...");
            }
        }
        info!("");
    }

    info!("💡 Run with the 'apply' command to execute these migrations.");
    info!("💡 Use '--dry-run' flag to see what would be executed without applying changes.");

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("Failed to load migrations: {0}")]
    LoadFailed(String),

    #[error("Connection error: {0}")]
    Connection(#[from] ConnectionError),
}

