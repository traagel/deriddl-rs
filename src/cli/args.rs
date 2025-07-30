use clap::{Parser, Subcommand};

/// CLI entry point for deriddl
#[derive(Parser, Debug)]
#[command(
    name = "deriddl",
    version,
    about = "Rust-based ODBC schema migration runner"
)]
pub struct Cli {
    /// Path to configuration file
    #[arg(long, global = true)]
    pub config: Option<String>,
    
    /// Environment (loads config/{env}.toml)
    #[arg(long, global = true)]
    pub env: Option<String>,
    
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Apply pending migrations
    Apply {
        /// ODBC connection string
        #[arg(long)]
        conn: Option<String>,

        /// Path to .sql migration files
        #[arg(long, default_value = "./migrations")]
        path: String,

        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,
    },

    /// Show applied and pending migrations
    Status {
        /// ODBC connection string
        #[arg(long)]
        conn: Option<String>,

        /// Path to .sql migration files
        #[arg(long, default_value = "./migrations")]
        path: String,
    },

    /// Initialize schema_migrations table
    Init {
        /// ODBC connection string
        #[arg(long)]
        conn: Option<String>,
    },

    /// Show which migrations would be applied
    Plan {
        /// ODBC connection string
        #[arg(long)]
        conn: Option<String>,

        /// Path to .sql migration files
        #[arg(long, default_value = "./migrations")]
        path: String,
    },

    /// Check system readiness and dependencies
    Health {
        /// Path to .sql migration files
        #[arg(long, default_value = "./migrations")]
        path: String,

        /// SQL dialect to validate against
        #[arg(long, default_value = "postgres")]
        dialect: String,
    },

    /// Generate configuration file
    Config {
        /// Output path for config file
        #[arg(long, default_value = "config.toml")]
        output: String,
        
        /// Create environment-specific config
        #[arg(long)]
        env: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_help() {
        let result = Cli::try_parse_from(["deriDDL", "--help"]);
        assert!(result.is_err()); // Help exits with error
    }

    #[test]
    fn test_cli_version() {
        let result = Cli::try_parse_from(["deriDDL", "--version"]);
        assert!(result.is_err()); // Version exits with error
    }

    #[test]
    fn test_apply_command_defaults() {
        let cli = Cli::try_parse_from(["deriDDL", "apply"]).unwrap();
        match cli.command {
            Commands::Apply { conn, path, dry_run } => {
                assert_eq!(conn, None);
                assert_eq!(path, "./migrations");
                assert!(!dry_run);
            }
            _ => panic!("Expected Apply command"),
        }
    }

    #[test]
    fn test_apply_command_with_flags() {
        let cli = Cli::try_parse_from([
            "deriDDL",
            "apply",
            "--conn",
            "Driver={SQLite3};Database=test.db;",
            "--path",
            "./custom-migrations",
            "--dry-run",
        ]).unwrap();
        
        match cli.command {
            Commands::Apply { conn, path, dry_run } => {
                assert_eq!(conn, Some("Driver={SQLite3};Database=test.db;".to_string()));
                assert_eq!(path, "./custom-migrations");
                assert!(dry_run);
            }
            _ => panic!("Expected Apply command"),
        }
    }

    #[test]
    fn test_status_command_defaults() {
        let cli = Cli::try_parse_from(["deriDDL", "status"]).unwrap();
        match cli.command {
            Commands::Status { conn, path } => {
                assert_eq!(conn, None);
                assert_eq!(path, "./migrations");
            }
            _ => panic!("Expected Status command"),
        }
    }

    #[test]
    fn test_init_command() {
        let cli = Cli::try_parse_from(["deriDDL", "init"]).unwrap();
        match cli.command {
            Commands::Init { conn } => {
                assert_eq!(conn, None);
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_plan_command() {
        let cli = Cli::try_parse_from(["deriDDL", "plan", "--conn", "test"]).unwrap();
        match cli.command {
            Commands::Plan { conn, path } => {
                assert_eq!(conn, Some("test".to_string()));
                assert_eq!(path, "./migrations");
            }
            _ => panic!("Expected Plan command"),
        }
    }

    #[test]
    fn test_health_command_defaults() {
        let cli = Cli::try_parse_from(["deriDDL", "health"]).unwrap();
        match cli.command {
            Commands::Health { path, dialect } => {
                assert_eq!(path, "./migrations");
                assert_eq!(dialect, "postgres");
            }
            _ => panic!("Expected Health command"),
        }
    }

    #[test]
    fn test_health_command_custom_dialect() {
        let cli = Cli::try_parse_from([
            "deriDDL",
            "health",
            "--dialect",
            "mysql",
            "--path",
            "./sql",
        ]).unwrap();
        
        match cli.command {
            Commands::Health { path, dialect } => {
                assert_eq!(path, "./sql");
                assert_eq!(dialect, "mysql");
            }
            _ => panic!("Expected Health command"),
        }
    }

    #[test]
    fn test_config_command_defaults() {
        let cli = Cli::try_parse_from(["deriDDL", "config"]).unwrap();
        match cli.command {
            Commands::Config { output, env } => {
                assert_eq!(output, "config.toml");
                assert_eq!(env, None);
            }
            _ => panic!("Expected Config command"),
        }
    }

    #[test]
    fn test_config_command_with_env() {
        let cli = Cli::try_parse_from([
            "deriDDL",
            "config",
            "--output",
            "custom.toml",
            "--env",
            "dev",
        ]).unwrap();
        
        match cli.command {
            Commands::Config { output, env } => {
                assert_eq!(output, "custom.toml");
                assert_eq!(env, Some("dev".to_string()));
            }
            _ => panic!("Expected Config command"),
        }
    }

    #[test]
    fn test_global_config_flag() {
        let cli = Cli::try_parse_from([
            "deriDDL",
            "--config",
            "custom-config.toml",
            "health",
        ]).unwrap();
        
        assert_eq!(cli.config, Some("custom-config.toml".to_string()));
        assert!(matches!(cli.command, Commands::Health { .. }));
    }

    #[test]
    fn test_global_env_flag() {
        let cli = Cli::try_parse_from([
            "deriDDL",
            "--env",
            "production",
            "status",
        ]).unwrap();
        
        assert_eq!(cli.env, Some("production".to_string()));
        assert!(matches!(cli.command, Commands::Status { .. }));
    }

    #[test]
    fn test_invalid_command() {
        let result = Cli::try_parse_from(["deriDDL", "invalid-command"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_required_subcommand() {
        let result = Cli::try_parse_from(["deriDDL"]);
        assert!(result.is_err());
    }
}
