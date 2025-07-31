use crate::executor::ConnectionError;
use crate::model::migration::{Migration, MigrationType};
use crate::tracker::version_store::{AppliedMigration, VersionStore};
use crate::orchestrator::migration_loader::MigrationLoader;
use log::{debug, error, info, warn};
use std::io::{self, Write};

/// Error types for rollback operations
#[derive(Debug, thiserror::Error)]
pub enum RollbackError {
    #[error("Connection error: {0}")]
    Connection(#[from] ConnectionError),
    
    #[error("Migration error: {0}")]
    Migration(String),
    
    #[error("No migrations to roll back")]
    NoMigrationsToRollback,
    
    #[error("Migration {0} cannot be rolled back: no rollback SQL found")]
    NoRollbackSql(String),
    
    #[error("Cannot rollback to version {0}: migration not found or not applied")]
    InvalidTargetVersion(u32),
    
    #[error("Rollback cancelled by user")]
    Cancelled,
    
    #[error("Repeatable migration {0} cannot be rolled back")]
    RepeatableMigrationRollback(String),
}

/// Rollback strategy
#[derive(Debug, Clone)]
pub enum RollbackStrategy {
    /// Roll back N migrations
    Steps(u32),
    /// Roll back to specific version (inclusive)
    ToVersion(u32),
}

/// Information about a migration rollback operation
#[derive(Debug, Clone)]
pub struct RollbackPlan {
    pub migrations_to_rollback: Vec<AppliedMigration>,
    pub strategy: RollbackStrategy,
    pub total_migrations: usize,
}

/// Run migration rollback with the specified strategy
pub fn run_rollback(
    connection_string: &str,
    migrations_path: &str,
    steps: u32,
    to_version: Option<u32>,
    dry_run: bool,
    require_confirmation: bool,
) -> Result<(), RollbackError> {
    info!("Starting rollback operation");
    debug!("Connection string length: {}", connection_string.len());
    debug!("Migrations path: {}", migrations_path);
    debug!("Dry run: {}", dry_run);

    let strategy = match to_version {
        Some(version) => RollbackStrategy::ToVersion(version),
        None => RollbackStrategy::Steps(steps),
    };
    
    // Create version store
    let mut version_store = VersionStore::new(connection_string)?;

    // Load migrations from filesystem
    let mut migrations = MigrationLoader::load_migrations(migrations_path)
        .map_err(|e| RollbackError::Migration(e.to_string()))?;

    // Get applied migrations from database
    let applied_migrations = version_store.get_applied_migrations()?;
    
    // Create rollback plan
    let plan = create_rollback_plan(&applied_migrations, &strategy)?;
    
    if plan.migrations_to_rollback.is_empty() {
        info!("✅ No migrations to roll back.");
        return Ok(());
    }

    // Display rollback plan
    display_rollback_plan(&plan, dry_run);

    // Get confirmation if required
    if require_confirmation && !dry_run {
        if !get_user_confirmation(&plan)? {
            return Err(RollbackError::Cancelled);
        }
    }

    // Load migration files and validate rollback SQL exists
    let migration_map = create_migration_map(&mut migrations);
    
    if dry_run {
        info!("🔍 Dry run mode - no changes will be applied");
        validate_rollback_plan(&plan, &migration_map)?;
        info!("✅ Rollback plan is valid");
        return Ok(());
    }

    // Execute rollbacks
    execute_rollbacks(&mut version_store, &plan, &migration_map)?;
    
    info!("✅ Rollback completed successfully");
    Ok(())
}

/// Create a rollback plan based on the strategy
pub fn create_rollback_plan(
    applied_migrations: &[AppliedMigration],
    strategy: &RollbackStrategy,
) -> Result<RollbackPlan, RollbackError> {
    // Filter to only versioned migrations (can't rollback repeatables)
    let mut versioned_migrations: Vec<_> = applied_migrations
        .iter()
        .filter(|m| m.migration_type == MigrationType::Versioned && m.success)
        .collect();
    
    // Sort by version descending (newest first)
    versioned_migrations.sort_by(|a, b| {
        b.version.unwrap_or(0).cmp(&a.version.unwrap_or(0))
    });

    let migrations_to_rollback = match strategy {
        RollbackStrategy::Steps(steps) => {
            let steps = *steps as usize;
            if steps > versioned_migrations.len() {
                warn!("Requested {} steps but only {} applied migrations", steps, versioned_migrations.len());
            }
            versioned_migrations.into_iter().take(steps).cloned().collect()
        }
        RollbackStrategy::ToVersion(target_version) => {
            let mut rollback_migrations = Vec::new();
            
            for migration in versioned_migrations {
                if let Some(version) = migration.version {
                    if version > *target_version {
                        rollback_migrations.push(migration.clone());
                    } else {
                        break;
                    }
                }
            }
            
            if rollback_migrations.is_empty() {
                return Err(RollbackError::InvalidTargetVersion(*target_version));
            }
            
            rollback_migrations
        }
    };

    Ok(RollbackPlan {
        total_migrations: migrations_to_rollback.len(),
        migrations_to_rollback,
        strategy: strategy.clone(),
    })
}

/// Display the rollback plan to the user
fn display_rollback_plan(plan: &RollbackPlan, dry_run: bool) {
    let action = if dry_run { "Would roll back" } else { "Will roll back" };
    
    match &plan.strategy {
        RollbackStrategy::Steps(steps) => {
            info!("{} {} migration(s):", action, steps);
        }
        RollbackStrategy::ToVersion(version) => {
            info!("{} migrations back to version {}:", action, version);
        }
    }

    println!();
    for migration in &plan.migrations_to_rollback {
        let version_str = migration.version.map_or("N/A".to_string(), |v| v.to_string());
        println!("  📦 V{:0>4} {} (applied: {})", 
                 version_str, migration.filename, migration.applied_at.format("%Y-%m-%d %H:%M:%S"));
    }
    println!();
}

/// Get user confirmation for rollback
fn get_user_confirmation(plan: &RollbackPlan) -> Result<bool, RollbackError> {
    warn!("⚠️  DESTRUCTIVE OPERATION");
    warn!("Rolling back {} migration(s) will permanently modify your database!", plan.total_migrations);
    print!("Do you want to continue? (y/N): ");
    io::stdout().flush().map_err(|_| RollbackError::Migration("Failed to flush stdout".to_string()))?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)
        .map_err(|_| RollbackError::Migration("Failed to read user input".to_string()))?;

    Ok(input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes")
}

/// Create a map of migration versions to Migration objects
fn create_migration_map(migrations: &mut [Migration]) -> std::collections::HashMap<u32, &Migration> {
    migrations.iter()
        .filter_map(|m| m.version.map(|v| (v, m)))
        .collect()
}

/// Validate that all migrations in the rollback plan have rollback SQL
pub fn validate_rollback_plan(
    plan: &RollbackPlan,
    migration_map: &std::collections::HashMap<u32, &Migration>,
) -> Result<(), RollbackError> {
    for applied_migration in &plan.migrations_to_rollback {
        if let Some(version) = applied_migration.version {
            if let Some(migration) = migration_map.get(&version) {
                if !migration.has_rollback() {
                    return Err(RollbackError::NoRollbackSql(applied_migration.filename.clone()));
                }
            } else {
                error!("Migration file not found for version {}", version);
                return Err(RollbackError::Migration(
                    format!("Migration file not found for version {}", version)
                ));
            }
        }
    }
    Ok(())
}

/// Execute the rollback operations
fn execute_rollbacks(
    version_store: &mut VersionStore,
    plan: &RollbackPlan,
    migration_map: &std::collections::HashMap<u32, &Migration>,
) -> Result<(), RollbackError> {
    let total = plan.migrations_to_rollback.len();
    
    for (i, applied_migration) in plan.migrations_to_rollback.iter().enumerate() {
        info!("Rolling back migration {}/{}: {}", i + 1, total, applied_migration.filename);
        
        if let Some(version) = applied_migration.version {
            if let Some(migration) = migration_map.get(&version) {
                // Validate rollback SQL exists
                let rollback_sql = migration.get_rollback_sql()
                    .ok_or_else(|| RollbackError::NoRollbackSql(applied_migration.filename.clone()))?;

                debug!("Executing rollback SQL for migration {}", version);
                debug!("Rollback SQL: {}", rollback_sql);

                // Execute rollback SQL
                let start_time = std::time::Instant::now();
                let rollback_result = {
                    let mut executor = version_store.executor()?;
                    executor.execute_query(rollback_sql)
                };
                
                match rollback_result {
                    Ok(_) => {
                        let execution_time = start_time.elapsed().as_millis() as u32;
                        info!("✅ Successfully rolled back migration {} in {}ms", 
                              applied_migration.filename, execution_time);
                        
                        // Remove from schema_migrations table
                        version_store.remove_migration(version)?;
                        
                    }
                    Err(e) => {
                        error!("❌ Failed to rollback migration {}: {}", 
                               applied_migration.filename, e);
                        return Err(RollbackError::Connection(e));
                    }
                }
            } else {
                return Err(RollbackError::Migration(
                    format!("Migration file not found for version {}", version)
                ));
            }
        }
    }

    Ok(())
}