use crate::executor::{ConnectionError, ConnectionManager, DatabaseExecutor};
use crate::tracker::{schema_init, VersionStore};
use log::{debug, error, info, warn};
use std::io::{self, Write};

pub fn run_baseline(
    conn: &str, 
    version: u32, 
    description: &str,
    from_schema: bool,
    dry_run: bool,
    require_confirmation: bool,
) -> Result<(), BaselineError> {
    info!("Running baseline creation");
    debug!("Connection string length: {}", conn.len());
    debug!("Baseline version: {}", version);
    debug!("Description: {}", description);
    debug!("From schema: {}", from_schema);
    debug!("Dry run: {}", dry_run);

    // Test connection first
    let connection_manager = ConnectionManager::new()?;
    connection_manager.test_connection(conn)?;
    info!("✅ Database connection verified");

    // Ensure schema_migrations table exists
    if !schema_init::check_migration_table_exists(conn)? {
        if dry_run {
            info!("🔍 DRY RUN: Would create schema_migrations table");
        } else {
            info!("Creating schema_migrations table");
            schema_init::init_migration_table(conn)?;
        }
    }

    // Check for existing migrations
    let mut version_store = VersionStore::new(conn)?;
    let applied_migrations = version_store.get_applied_migrations()?;
    
    if !applied_migrations.is_empty() {
        warn!("⚠️  Database already has {} applied migrations", applied_migrations.len());
        for migration in &applied_migrations {
            let migration_version = migration.version.map_or("R".to_string(), |v| v.to_string());
            warn!("    - {} ({})", migration.filename, migration_version);
        }
        
        // Check if any migrations are at or above the baseline version
        let conflicting_migrations: Vec<_> = applied_migrations
            .iter()
            .filter(|m| {
                if let Some(v) = m.version {
                    v >= version
                } else {
                    false // Repeatable migrations don't conflict with version baselines
                }
            })
            .collect();
            
        if !conflicting_migrations.is_empty() {
            error!("❌ Cannot create baseline at version {} - {} existing migrations at or above this version:", 
                version, conflicting_migrations.len());
            for migration in conflicting_migrations {
                error!("    - {} (version {})", migration.filename, migration.version.unwrap());
            }
            return Err(BaselineError::ConflictingMigrations(version));
        }
    }

    // Show what will be done
    info!("📋 Baseline Plan");
    info!("================");
    info!("Baseline version: {}", version);
    info!("Description: {}", description);
    
    if from_schema {
        info!("Schema dump: Will be generated from current database state");
    }
    
    info!("Existing migrations: {}", applied_migrations.len());
    
    if dry_run {
        info!("🔍 DRY RUN: Baseline would be created successfully");
        return Ok(());
    }

    // Require confirmation if configured
    if require_confirmation {
        print!("Are you sure you want to create baseline version {} (y/N)? ", version);
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            info!("Baseline creation cancelled");
            return Ok(());
        }
    }

    // Create the baseline
    create_baseline(&mut version_store, version, description, from_schema, conn)?;
    
    info!("🎉 Baseline version {} created successfully!", version);
    info!("Future migrations with version > {} will be applied", version);
    
    Ok(())
}

fn create_baseline(
    version_store: &mut VersionStore,
    version: u32,
    description: &str,
    from_schema: bool,
    conn: &str,
) -> Result<(), BaselineError> {
    debug!("Creating baseline record in database");
    
    // Create baseline record
    version_store.create_baseline(version, description)?;
    
    // Generate schema dump if requested
    if from_schema {
        match generate_schema_dump(conn, version) {
            Ok(schema_file) => {
                info!("📄 Schema dump generated: {}", schema_file);
            }
            Err(e) => {
                warn!("⚠️  Failed to generate schema dump: {}", e);
                // Don't fail the baseline creation for this
            }
        }
    }
    
    Ok(())
}

fn generate_schema_dump(conn: &str, version: u32) -> Result<String, BaselineError> {
    debug!("Generating schema dump for baseline version {}", version);
    
    let connection_manager = ConnectionManager::new()?;
    let connection = connection_manager.connect(conn)?;
    let mut executor = DatabaseExecutor::new(connection);
    
    // Try to get schema information (this is database-specific)
    // For now, we'll create a simple placeholder - in a real implementation,
    // this would extract DDL statements from the database
    let schema_queries = vec![
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != 'schema_migrations'",
    ];
    
    use chrono::Utc;
    let mut schema_content = format!(
        "-- Schema dump for baseline version {}\n-- Generated at: {}\n\n",
        version,
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );
    
    for query in schema_queries {
        match executor.query_rows(query) {
            Ok(rows) => {
                if !rows.is_empty() {
                    schema_content.push_str("-- Tables found:\n");
                    for row in rows {
                        if let Some(table_name) = row.first() {
                            schema_content.push_str(&format!("-- Table: {}\n", table_name));
                        }
                    }
                }
            }
            Err(_) => {
                // Ignore errors for schema introspection - different databases have different system tables
            }
        }
    }
    
    schema_content.push_str(&format!(
        "\n-- This is a baseline marker - no actual DDL to execute\n-- Database was baselined at version {}\n",
        version
    ));
    
    let schema_file = format!("baseline_{:04}_schema_dump.sql", version);
    std::fs::write(&schema_file, schema_content)
        .map_err(|e| BaselineError::SchemaGeneration(e.to_string()))?;
    
    Ok(schema_file)
}

#[derive(Debug, thiserror::Error)]
pub enum BaselineError {
    #[error("Connection error: {0}")]
    Connection(#[from] ConnectionError),

    #[error("Cannot create baseline version {0} - conflicting migrations exist at or above this version")]
    ConflictingMigrations(u32),

    #[error("Baseline version {0} already exists")]
    BaselineExists(u32),

    #[error("Failed to generate schema dump: {0}")]
    SchemaGeneration(String),

    #[error("Invalid baseline version: {0}")]
    InvalidVersion(String),
}