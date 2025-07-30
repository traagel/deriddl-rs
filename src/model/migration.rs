use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq)]
pub enum MigrationType {
    /// Versioned migrations (V001__description.sql) - run once in order
    Versioned,
    /// Repeatable migrations (R__description.sql) - re-run when checksum changes
    Repeatable,
}

#[derive(Debug, Clone)]
pub struct Migration {
    pub migration_type: MigrationType,
    pub version: Option<u32>, // None for repeatable migrations
    pub name: String,
    pub file_path: PathBuf,
    pub sql_content: String,
    pub checksum: String,
    pub applied_at: Option<DateTime<Utc>>,
    pub execution_time_ms: Option<u32>,
    pub success: bool,
}

impl Migration {
    /// Constructs a new versioned `Migration` with computed checksum and default metadata.
    pub fn new(version: u32, name: String, file_path: PathBuf, sql_content: String) -> Self {
        let checksum = Self::compute_checksum(&sql_content);

        Self {
            migration_type: MigrationType::Versioned,
            version: Some(version),
            name,
            file_path,
            sql_content,
            checksum,
            applied_at: None,
            execution_time_ms: None,
            success: true,
        }
    }

    /// Creates a Migration from database applied migration data, useful for reconstructing
    /// migration state from database records
    pub fn from_applied(
        applied: &crate::tracker::version_store::AppliedMigration,
        file_path: PathBuf,
        sql_content: String,
    ) -> Self {
        Self {
            migration_type: applied.migration_type.clone(),
            version: applied.version,
            name: extract_name_from_filename(&applied.filename),
            file_path,
            sql_content,
            checksum: applied.checksum.clone(),
            applied_at: Some(applied.applied_at),
            execution_time_ms: Some(applied.execution_time_ms as u32),
            success: applied.success,
        }
    }

    /// Returns true if this migration has been applied to the database
    pub fn is_applied(&self) -> bool {
        self.applied_at.is_some() && self.success
    }

    /// Returns the execution time in milliseconds if the migration has been applied
    pub fn execution_time(&self) -> Option<u32> {
        self.execution_time_ms
    }

    /// Returns the timestamp when this migration was applied, if applicable
    pub fn applied_timestamp(&self) -> Option<DateTime<Utc>> {
        self.applied_at
    }
    
    /// Constructs a new repeatable `Migration` with computed checksum and default metadata.
    pub fn new_repeatable(name: String, file_path: PathBuf, sql_content: String) -> Self {
        let checksum = Self::compute_checksum(&sql_content);

        Self {
            migration_type: MigrationType::Repeatable,
            version: None,
            name,
            file_path,
            sql_content,
            checksum,
            applied_at: None,
            execution_time_ms: None,
            success: true,
        }
    }

    /// Returns the expected canonical filename for the migration.
    pub fn filename(&self) -> String {
        match &self.migration_type {
            MigrationType::Versioned => {
                format!("{:04}_{}.sql", self.version.unwrap_or(0), self.name)
            }
            MigrationType::Repeatable => {
                format!("R__{}.sql", self.name)
            }
        }
    }
    
    /// Returns a unique identifier for this migration in the database.
    /// For versioned migrations, this is the version number.
    /// For repeatable migrations, this is the name with R__ prefix.
    pub fn identifier(&self) -> String {
        match &self.migration_type {
            MigrationType::Versioned => self.version.unwrap_or(0).to_string(),
            MigrationType::Repeatable => format!("R__{}", self.name),
        }
    }
    
    /// Returns true if this migration is repeatable.
    pub fn is_repeatable(&self) -> bool {
        self.migration_type == MigrationType::Repeatable
    }

    /// Computes a stable checksum based on the SQL content.
    fn compute_checksum(content: &str) -> String {
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

/// Extracts the migration name from a filename (e.g., "0001_create_users.sql" -> "create_users")
fn extract_name_from_filename(filename: &str) -> String {
    let stem = filename.strip_suffix(".sql").unwrap_or(filename);
    
    if stem.starts_with("R__") {
        // Repeatable migration: R__create_view.sql -> create_view
        stem.strip_prefix("R__").unwrap_or(stem).to_string()
    } else if let Some(underscore_pos) = stem.find('_') {
        // Versioned migration: 0001_create_users.sql -> create_users
        stem[underscore_pos + 1..].to_string()
    } else {
        // Fallback
        stem.to_string()
    }
}

