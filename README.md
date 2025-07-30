# deriDDL

**A fast, deterministic, Rust-native ODBC schema migration tool.**  
Run versioned SQL migrations against ODBC-only databases with safety, auditability, and zero JVM overhead.

---

## 🔧 Why deriDDL?

Most enterprise data platforms (e.g. Databricks, Snowflake, Synapse) expose **ODBC-only interfaces**, yet migration tooling is dominated by **JDBC-based Java tools** like Flyway and Liquibase.  

**deriDDL** eliminates the bloat:
- No JVM
- No XML/DSL
- No per-connection licensing
- Just SQL + Rust

---

## 🚀 Features

- ✅ ODBC-based execution via [`odbc-api`](https://crates.io/crates/odbc-api)
- ✅ Versioned `.sql` file migrations
- ✅ `schema_migrations` tracking table
- ✅ Dry-run mode for CI/CD verification
- ✅ **TOML configuration system** with environment support
- ✅ **SQLGlot integration** for SQL validation
- ✅ Health checks and system readiness verification
- ✅ Modular architecture for extension
- ✅ Single static binary (no runtime deps)

---

## ⚙️ Configuration

deriDDL supports flexible TOML-based configuration with environment-specific overrides.

### Quick Start

Generate a default configuration file:
```bash
cargo run -- config
```

This creates `config.toml` with all available settings and sensible defaults.

### Configuration Structure

```toml
[database]
# Connection string (can be overridden with --conn)
connection_string = "Driver={PostgreSQL};Server=localhost;..."
timeout = 30
max_retries = 3

[migrations]
path = "./migrations"          # Directory containing .sql files
dialect = "postgres"           # SQL dialect for validation
validate_sql = true            # Enable SQLGlot validation
file_pattern = '^\d{4}_.*\.sql$'  # Migration file naming pattern

[logging]
level = "info"                 # error, warn, info, debug, trace
colored = true
format = "pretty"              # compact, pretty, json

[behavior]
auto_create_migrations_dir = false
require_confirmation = true
default_dry_run = false

[validation]
enable_sqlglot = true          # Requires: pip install sqlglot
strict_validation = false      # Fail on warnings, not just errors
max_file_size_mb = 10
```

### Environment-Specific Configuration

Create environment overrides using the `--env` flag:

```bash
# Generate environment-specific config
cargo run -- config --env dev
cargo run -- config --env staging  
cargo run -- config --env prod
```

This creates `config/{env}.toml` files that override the base configuration.

**Example `config/dev.toml`:**
```toml
[database]
connection_string = "Driver={PostgreSQL};Server=dev-db;..."

[migrations]
dialect = "postgres"

[logging]
level = "debug"
```

### Configuration Loading Priority

1. **Base config**: `config.toml` or `config/default.toml`
2. **Environment override**: `config/{env}.toml` (if `--env` specified)
3. **Local overrides**: `config/local.toml` (git-ignored, always applied last)
4. **CLI flags**: Override everything

### Usage Examples

```bash
# Use default config
cargo run -- health

# Use specific environment
cargo run -- --env dev health

# Use custom config file
cargo run -- --config my-config.toml status

# CLI flags override everything
cargo run -- --env prod --conn "Driver=..." apply
```

---

## 📁 Migration File Format

```text
migrations/
├── 0001_init_schema.sql
├── 0002_add_users_table.sql
└── 0003_add_index.sql
```

Files must follow the `{version}_{description}.sql` pattern where:
- **Version**: 4-digit zero-padded number (0001, 0002, etc.)
- **Description**: Snake_case description
- **Extension**: `.sql`

---

## 🏃 Commands

### Health Check
Verify system readiness and dependencies:
```bash
# Check with default settings
cargo run -- health

# Check specific environment
cargo run -- --env prod health

# Check with custom path and dialect
cargo run -- health --path ./my-migrations --dialect mysql
```

**Health checks include:**
- ✅ Python installation
- ✅ SQLGlot availability and dialect support
- ✅ Migrations directory accessibility
- ✅ File permissions
- ✅ Migration sequence validation

### Configuration Management
```bash
# Generate default config
cargo run -- config

# Generate environment-specific configs
cargo run -- config --env dev
cargo run -- config --env staging
cargo run -- config --env prod

# Custom output location
cargo run -- config --output my-config.toml
```

### Migration Operations
```bash
# Initialize schema_migrations table
cargo run -- init --conn "Driver={PostgreSQL};..."

# Check migration status
cargo run -- status --conn "..." --path ./migrations

# Preview pending migrations
cargo run -- plan --conn "..." --path ./migrations

# Apply migrations (dry-run)
cargo run -- apply --conn "..." --path ./migrations --dry-run

# Apply migrations (live)
cargo run -- apply --conn "..." --path ./migrations
```

### Global Flags
All commands support these global configuration flags:
- `--config <path>`: Custom configuration file
- `--env <environment>`: Load environment-specific config

---

## 🧪 Development

### Prerequisites
- Rust 2024 edition
- Python 3.x (for SQLGlot validation)
- Virtual environment with `sqlglot` installed

### Setup
```bash
# Clone and build
git clone <repo>
cd deriDDL
cargo build

# Set up Python environment (optional, for SQL validation)
python -m venv venv
source venv/bin/activate  # or `venv\Scripts\activate` on Windows
pip install sqlglot

# Run health check
cargo run -- health
```

### Architecture
```text
src/
├── cli/          # Command-line interface and argument parsing
├── model/        # Data structures (Migration, Config)
├── orchestrator/ # Business logic (apply, plan, status, health)
├── executor/     # ODBC connection and query execution
└── tracker/      # Migration state tracking
```

---

## 📄 License

[Add your license here]

---

## 🤝 Contributing

[Add contribution guidelines here]
