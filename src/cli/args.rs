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
