use log::{info, warn, error, debug};
use std::process::Command;
use std::path::Path;
use std::fs;

#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub name: String,
    pub status: HealthStatus,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Pass,
    Warn,
    Fail,
}

pub fn run_health(path: &str, dialect: &str) {
    info!("Running system health check");
    debug!("Migrations path: {}", path);
    debug!("SQL dialect: {}", dialect);

    let mut checks = Vec::new();
    let mut overall_status = HealthStatus::Pass;

    // Check Python installation
    checks.push(check_python());
    
    // Check SQLGlot availability
    checks.push(check_sqlglot(dialect));
    
    // Check migrations directory
    checks.push(check_migrations_directory(path));
    
    // Check migration file permissions
    checks.push(check_file_permissions(path));
    
    // Check for migration sequence issues
    if let Ok(migrations) = crate::orchestrator::MigrationLoader::load_migrations(path) {
        checks.push(check_migration_sequence(&migrations));
    } else {
        checks.push(HealthCheckResult {
            name: "Migration Loading".to_string(),
            status: HealthStatus::Fail,
            message: "Failed to load migrations".to_string(),
        });
    }

    // Display results
    info!("Health Check Results:");
    info!("===================");
    
    for check in &checks {
        match check.status {
            HealthStatus::Pass => {
                info!("✅ {}: {}", check.name, check.message);
            }
            HealthStatus::Warn => {
                warn!("⚠️  {}: {}", check.name, check.message);
                if overall_status == HealthStatus::Pass {
                    overall_status = HealthStatus::Warn;
                }
            }
            HealthStatus::Fail => {
                error!("❌ {}: {}", check.name, check.message);
                overall_status = HealthStatus::Fail;
            }
        }
    }
    
    info!("===================");
    match overall_status {
        HealthStatus::Pass => info!("🎉 All checks passed! System is ready."),
        HealthStatus::Warn => warn!("⚠️  System has warnings but should work."),
        HealthStatus::Fail => error!("❌ System has critical issues that need fixing."),
    }
}

fn check_python() -> HealthCheckResult {
    match Command::new("python").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            HealthCheckResult {
                name: "Python".to_string(),
                status: HealthStatus::Pass,
                message: format!("Found: {}", version.trim()),
            }
        }
        _ => match Command::new("python3").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout);
                HealthCheckResult {
                    name: "Python".to_string(),
                    status: HealthStatus::Pass,
                    message: format!("Found: {}", version.trim()),
                }
            }
            _ => HealthCheckResult {
                name: "Python".to_string(),
                status: HealthStatus::Fail,
                message: "Python not found. Install Python 3.x".to_string(),
            }
        }
    }
}

fn check_sqlglot(dialect: &str) -> HealthCheckResult {
    // Check if SQLGlot is available
    match Command::new("python").arg("-m").arg("sqlglot").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            
            // Test with the specified dialect
            let test_sql = "SELECT 1";
            match Command::new("python")
                .arg("-m")
                .arg("sqlglot")
                .arg("--parse")
                .arg("--read")
                .arg(dialect)
                .arg(test_sql)
                .output() {
                Ok(dialect_output) if dialect_output.status.success() => {
                    HealthCheckResult {
                        name: "SQLGlot".to_string(),
                        status: HealthStatus::Pass,
                        message: format!("Version {} - {} dialect supported", version.trim(), dialect),
                    }
                }
                _ => HealthCheckResult {
                    name: "SQLGlot".to_string(),
                    status: HealthStatus::Warn,
                    message: format!("Version {} found but {} dialect not supported", version.trim(), dialect),
                }
            }
        }
        _ => HealthCheckResult {
            name: "SQLGlot".to_string(),
            status: HealthStatus::Warn,
            message: "SQLGlot not found. Install with: pip install sqlglot".to_string(),
        }
    }
}

fn check_migrations_directory(path: &str) -> HealthCheckResult {
    let migrations_path = Path::new(path);
    
    if !migrations_path.exists() {
        return HealthCheckResult {
            name: "Migrations Directory".to_string(),
            status: HealthStatus::Warn,
            message: format!("Directory '{}' does not exist", path),
        };
    }
    
    if !migrations_path.is_dir() {
        return HealthCheckResult {
            name: "Migrations Directory".to_string(),
            status: HealthStatus::Fail,
            message: format!("'{}' exists but is not a directory", path),
        };
    }
    
    // Count SQL files
    match fs::read_dir(migrations_path) {
        Ok(entries) => {
            let sql_count = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext == "sql")
                        .unwrap_or(false)
                })
                .count();
                
            HealthCheckResult {
                name: "Migrations Directory".to_string(),
                status: HealthStatus::Pass,
                message: format!("Found {} SQL migration files in '{}'", sql_count, path),
            }
        }
        Err(e) => HealthCheckResult {
            name: "Migrations Directory".to_string(),
            status: HealthStatus::Fail,
            message: format!("Cannot read directory '{}': {}", path, e),
        }
    }
}

fn check_file_permissions(path: &str) -> HealthCheckResult {
    let migrations_path = Path::new(path);
    
    if !migrations_path.exists() {
        return HealthCheckResult {
            name: "File Permissions".to_string(),
            status: HealthStatus::Warn,
            message: "Migrations directory does not exist".to_string(),
        };
    }
    
    // Check if we can read the directory
    match fs::read_dir(migrations_path) {
        Ok(_) => HealthCheckResult {
            name: "File Permissions".to_string(),
            status: HealthStatus::Pass,
            message: "Can read migrations directory".to_string(),
        },
        Err(e) => HealthCheckResult {
            name: "File Permissions".to_string(),
            status: HealthStatus::Fail,
            message: format!("Cannot read migrations directory: {}", e),
        }
    }
}

fn check_migration_sequence(migrations: &[crate::model::Migration]) -> HealthCheckResult {
    let issues = crate::orchestrator::Validator::validate_migration_sequence(migrations);
    
    if issues.is_empty() {
        HealthCheckResult {
            name: "Migration Sequence".to_string(),
            status: HealthStatus::Pass,
            message: format!("{} migrations in correct sequence", migrations.len()),
        }
    } else {
        HealthCheckResult {
            name: "Migration Sequence".to_string(),
            status: HealthStatus::Warn,
            message: format!("{} sequence issues found: {}", issues.len(), issues.join(", ")),
        }
    }
}