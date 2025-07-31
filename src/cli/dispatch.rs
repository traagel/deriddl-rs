use crate::cli::args::{Cli, Commands};
use crate::model::Config;
use crate::orchestrator;
use log::{debug, error, info};

pub fn handle(cli: Cli) {
    // Load configuration
    let config = match Config::load(cli.config.as_deref(), cli.env.as_deref()) {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    debug!("Loaded configuration: {:?}", config);

    match cli.command {
        Commands::Apply {
            conn,
            path,
            dry_run,
        } => {
            info!("Running APPLY command");
            let final_conn = conn
                .or(config.database.connection_string)
                .unwrap_or_else(|| {
                    error!("No connection string provided via --conn flag or config file");
                    std::process::exit(1);
                });
            let final_path = if path == "./migrations" {
                &config.migrations.path
            } else {
                &path
            };
            let final_dry_run = dry_run || config.behavior.default_dry_run;

            debug!("Connection: {}", final_conn);
            debug!("Migrations path: {}", final_path);
            debug!("Dry run mode: {}", final_dry_run);
            if let Err(e) = orchestrator::run_apply(&final_conn, final_path, final_dry_run) {
                error!("Apply command failed: {}", e);
                std::process::exit(1);
            }
        }

        Commands::Status { conn, path } => {
            info!("Running STATUS command");
            let final_conn = conn
                .or(config.database.connection_string)
                .unwrap_or_else(|| {
                    error!("No connection string provided via --conn flag or config file");
                    std::process::exit(1);
                });
            let final_path = if path == "./migrations" {
                &config.migrations.path
            } else {
                &path
            };

            debug!("Connection: {}", final_conn);
            debug!("Migrations path: {}", final_path);
            if let Err(e) = orchestrator::run_status(&final_conn, final_path) {
                error!("Status command failed: {}", e);
                std::process::exit(1);
            }
        }

        Commands::Plan { conn, path } => {
            info!("Running PLAN command");
            let final_conn = conn
                .or(config.database.connection_string)
                .unwrap_or_else(|| {
                    error!("No connection string provided via --conn flag or config file");
                    std::process::exit(1);
                });
            let final_path = if path == "./migrations" {
                &config.migrations.path
            } else {
                &path
            };

            debug!("Connection: {}", final_conn);
            debug!("Migrations path: {}", final_path);
            if let Err(e) = orchestrator::run_plan(&final_conn, final_path) {
                error!("Plan command failed: {}", e);
                std::process::exit(1);
            }
        }

        Commands::Health { path, dialect } => {
            info!("Running HEALTH command");
            let final_path = if path == "./migrations" {
                &config.migrations.path
            } else {
                &path
            };
            let final_dialect = if dialect == "postgres" {
                &config.migrations.dialect
            } else {
                &dialect
            };

            debug!("Migrations path: {}", final_path);
            debug!("SQL dialect: {}", final_dialect);

            if !std::path::Path::new(final_path).exists() {
                error!("Migrations path does not exist: {}", final_path);
                std::process::exit(1);
            }

            orchestrator::run_health(final_path, final_dialect);
        }

        Commands::Validate { conn, path } => {
            info!("Running VALIDATE command");
            let final_conn = conn
                .or(config.database.connection_string)
                .unwrap_or_else(|| {
                    error!("No connection string provided via --conn flag or config file");
                    std::process::exit(1);
                });
            let final_path = if path == "./migrations" {
                &config.migrations.path
            } else {
                &path
            };

            debug!("Connection: {}", final_conn);
            debug!("Migrations path: {}", final_path);
            if let Err(e) = orchestrator::run_validate(&final_conn, final_path) {
                error!("Validate command failed: {}", e);
                std::process::exit(1);
            }
        }

        Commands::Rollback { conn, path, steps, to_version, dry_run, force } => {
            info!("Running ROLLBACK command");
            let final_conn = conn
                .or(config.database.connection_string)
                .unwrap_or_else(|| {
                    error!("No connection string provided via --conn flag or config file");
                    std::process::exit(1);
                });
            let final_path = if path == "./migrations" {
                &config.migrations.path
            } else {
                &path
            };
            let final_dry_run = dry_run || config.behavior.default_dry_run;
            let require_confirmation = config.behavior.require_confirmation && !force;

            debug!("Connection: {}", final_conn);
            debug!("Migrations path: {}", final_path);
            debug!("Steps: {}", steps);
            debug!("To version: {:?}", to_version);
            debug!("Dry run mode: {}", final_dry_run);
            debug!("Force mode: {}", force);
            
            if let Err(e) = orchestrator::run_rollback(
                &final_conn,
                final_path,
                steps,
                to_version,
                final_dry_run,
                require_confirmation,
            ) {
                error!("Rollback command failed: {}", e);
                std::process::exit(1);
            }
        }

        Commands::Baseline { conn, version, description, from_schema, dry_run } => {
            info!("Running BASELINE command");
            let final_conn = conn
                .or(config.database.connection_string)
                .unwrap_or_else(|| {
                    error!("No connection string provided via --conn flag or config file");
                    std::process::exit(1);
                });
            
            // Use config defaults if not provided via CLI
            let final_description = if description.is_empty() {
                config.baseline.default_description.as_str()
            } else {
                description.as_str()
            };
            
            let require_confirmation = config.baseline.require_confirmation;
            let final_from_schema = from_schema || config.baseline.auto_generate_schema;

            debug!("Connection: {}", final_conn);
            debug!("Baseline version: {}", version);
            debug!("Description: {}", final_description);
            debug!("From schema: {}", final_from_schema);
            debug!("Dry run: {}", dry_run);
            
            if let Err(e) = orchestrator::run_baseline(
                &final_conn,
                version,
                final_description,
                final_from_schema,
                dry_run,
                require_confirmation,
            ) {
                error!("Baseline command failed: {}", e);
                std::process::exit(1);
            }
        }

        Commands::Init { conn } => {
            info!("Running INIT command");
            let final_conn = conn
                .or(config.database.connection_string)
                .unwrap_or_else(|| {
                    error!("No connection string provided via --conn flag or config file");
                    std::process::exit(1);
                });

            debug!("Connection: {}", final_conn);
            
            if let Err(e) = crate::tracker::schema_init::init_migration_table_with_config(
                &final_conn, 
                Some(&config.migrations.dialect)
            ) {
                error!("Init command failed: {}", e);
                std::process::exit(1);
            }
        }

        Commands::Config { output, env } => {
            info!("Running CONFIG command");
            debug!("Output path: {}", output);

            match Config::generate_default_config(&output) {
                Ok(()) => {
                    info!("Generated default configuration file: {}", output);
                    if let Some(env_name) = env {
                        let env_path = format!("config/{}.toml", env_name);
                        match std::fs::create_dir_all("config") {
                            Ok(()) => match Config::generate_default_config(&env_path) {
                                Ok(()) => {
                                    info!("Generated environment configuration file: {}", env_path)
                                }
                                Err(e) => error!("Failed to create environment config: {}", e),
                            },
                            Err(e) => error!("Failed to create config directory: {}", e),
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to generate configuration file: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
