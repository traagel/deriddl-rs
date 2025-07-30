use crate::cli::args::{Cli, Commands};
use crate::orchestrator;
use crate::tracker::schema_init;
use crate::model::Config;
use log::{info, debug, error};

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
            let final_conn = conn.or(config.database.connection_string).unwrap_or_else(|| {
                error!("No connection string provided via --conn flag or config file");
                std::process::exit(1);
            });
            let final_path = if path == "./migrations" { &config.migrations.path } else { &path };
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
            let final_conn = conn.or(config.database.connection_string).unwrap_or_else(|| {
                error!("No connection string provided via --conn flag or config file");
                std::process::exit(1);
            });
            let final_path = if path == "./migrations" { &config.migrations.path } else { &path };
            
            debug!("Connection: {}", final_conn);
            debug!("Migrations path: {}", final_path);
            if let Err(e) = orchestrator::run_status(&final_conn, final_path) {
                error!("Status command failed: {}", e);
                std::process::exit(1);
            }
        }

        Commands::Init { conn } => {
            info!("Running INIT command");
            let final_conn = conn.or(config.database.connection_string).unwrap_or_else(|| {
                error!("No connection string provided via --conn flag or config file");
                std::process::exit(1);
            });
            
            debug!("Connection: {}", final_conn);
            if let Err(e) = schema_init::init_migration_table(&final_conn) {
                error!("Init command failed: {}", e);
                std::process::exit(1);
            }
        }

        Commands::Plan { conn, path } => {
            info!("Running PLAN command");
            let final_conn = conn.or(config.database.connection_string).unwrap_or_else(|| {
                error!("No connection string provided via --conn flag or config file");
                std::process::exit(1);
            });
            let final_path = if path == "./migrations" { &config.migrations.path } else { &path };
            
            debug!("Connection: {}", final_conn);
            debug!("Migrations path: {}", final_path);
            if let Err(e) = orchestrator::run_plan(&final_conn, final_path) {
                error!("Plan command failed: {}", e);
                std::process::exit(1);
            }
        }

        Commands::Health { path, dialect } => {
            info!("Running HEALTH command");
            let final_path = if path == "./migrations" { &config.migrations.path } else { &path };
            let final_dialect = if dialect == "postgres" { &config.migrations.dialect } else { &dialect };
            
            debug!("Migrations path: {}", final_path);
            debug!("SQL dialect: {}", final_dialect);
            orchestrator::run_health(final_path, final_dialect);
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
                                Ok(()) => info!("Generated environment configuration file: {}", env_path),
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
