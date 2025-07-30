use log::{info, debug, error, warn};
use crate::orchestrator::{MigrationLoader, Validator};

pub fn run_status(conn: &str, path: &str) {
    info!("Checking migration status");
    debug!("Connection: {}", conn);
    debug!("Migrations path: {}", path);

    match MigrationLoader::load_migrations(path) {
        Ok(migrations) => {
            info!("Found {} migration files:", migrations.len());
            
            // Validate migration sequence
            let sequence_issues = Validator::validate_migration_sequence(&migrations);
            if !sequence_issues.is_empty() {
                warn!("Migration sequence issues found:");
                for issue in sequence_issues {
                    warn!("  {}", issue);
                }
            }
            
            // Display migration info with basic validation
            for migration in &migrations {
                let line_count = migration.sql_content.lines().count();
                info!("  {} - {} lines", migration.filename(), line_count);
                
                // Optional: validate SQL if sqlglot is available
                let validation = Validator::validate_sql(&migration.sql_content, "postgres");
                if !validation.is_valid {
                    if let Some(error) = validation.error_message {
                        warn!("    SQL validation failed: {}", error);
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to load migrations: {}", e);
        }
    }

    // TODO: Compare applied vs available migrations
}
