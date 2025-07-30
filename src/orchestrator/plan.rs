use crate::executor::ConnectionError;
use crate::orchestrator::MigrationLoader;
use crate::tracker::{schema_init, VersionStore};
use log::{debug, info};

pub fn run_plan(conn: &str, path: &str) -> Result<(), PlanError> {
    info!("Running migration plan");
    debug!("Connection string length: {}", conn.len());
    debug!("Migrations path: {}", path);

    // Load migrations from filesystem
    let migrations =
        MigrationLoader::load_migrations(path).map_err(|e| PlanError::LoadFailed(e.to_string()))?;

    if migrations.is_empty() {
        println!("📋 No migrations found in {}", path);
        return Ok(());
    }

    // Check if schema_migrations table exists
    let table_exists = schema_init::check_migration_table_exists(conn)?;

    if !table_exists {
        println!("📋 Migration Plan");
        println!("================");
        println!("⚠️  schema_migrations table does not exist. All migrations will be applied.");
        println!();
        println!("Migrations to apply ({}):", migrations.len());
        for (i, migration) in migrations.iter().enumerate() {
            println!(
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

    println!("📋 Migration Plan");
    println!("================");

    if pending_migrations.is_empty() {
        println!("✅ No pending migrations to apply. Database is up to date!");
        return Ok(());
    }

    println!("Pending migrations ({}):", pending_migrations.len());
    println!();

    for (i, migration) in pending_migrations.iter().enumerate() {
        println!("{}. 📄 {}", i + 1, migration.filename());
        println!("   Version: {}", migration.version);
        println!("   Lines: {}", migration.sql_content.lines().count());
        println!("   Checksum: {}...", &migration.checksum[..8]);

        // Show SQL preview (first few lines)
        let sql_lines: Vec<&str> = migration.sql_content.lines().take(3).collect();
        if !sql_lines.is_empty() {
            println!("   Preview:");
            for line in sql_lines {
                if !line.trim().is_empty() {
                    println!("     {}", line.chars().take(60).collect::<String>());
                }
            }
            if migration.sql_content.lines().count() > 3 {
                println!("     ...");
            }
        }
        println!();
    }

    println!("💡 Run with the 'apply' command to execute these migrations.");
    println!("💡 Use '--dry-run' flag to see what would be executed without applying changes.");

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("Failed to load migrations: {0}")]
    LoadFailed(String),

    #[error("Connection error: {0}")]
    Connection(#[from] ConnectionError),
}

