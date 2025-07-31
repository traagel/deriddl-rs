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
    pub rollback_sql: Option<String>, // SQL for rolling back this migration
    pub checksum: String,
    pub applied_at: Option<DateTime<Utc>>,
    pub execution_time_ms: Option<u32>,
    pub success: bool,
}

impl Migration {
    /// Constructs a new versioned `Migration` with computed checksum and default metadata.
    pub fn new(version: u32, name: String, file_path: PathBuf, sql_content: String) -> Self {
        let (up_sql, down_sql) = Self::parse_migration_content(&sql_content);
        let checksum = Self::compute_checksum(&up_sql);

        Self {
            migration_type: MigrationType::Versioned,
            version: Some(version),
            name,
            file_path,
            sql_content: up_sql,
            rollback_sql: down_sql,
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
        let (up_sql, down_sql) = Self::parse_migration_content(&sql_content);
        
        Self {
            migration_type: applied.migration_type.clone(),
            version: applied.version,
            name: extract_name_from_filename(&applied.filename),
            file_path,
            sql_content: up_sql,
            rollback_sql: down_sql,
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
        let (up_sql, down_sql) = Self::parse_migration_content(&sql_content);
        let checksum = Self::compute_checksum(&up_sql);

        Self {
            migration_type: MigrationType::Repeatable,
            version: None,
            name,
            file_path,
            sql_content: up_sql,
            rollback_sql: down_sql,
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

    /// Parses migration content to separate up/down SQL sections
    /// Supports two formats:
    /// 1. Separator-based: -- +migrate Up / -- +migrate Down
    /// 2. Section-based: -- UP / -- DOWN
    fn parse_migration_content(content: &str) -> (String, Option<String>) {
        let content = content.trim();
        
        // Try different separator patterns
        let separators = [
            ("-- +migrate Up", "-- +migrate Down"),
            ("-- UP", "-- DOWN"),
            ("-- +goose Up", "-- +goose Down"), // Compatible with goose migrations
            ("-- @@UP@@", "-- @@DOWN@@"),
        ];
        
        for (up_marker, down_marker) in &separators {
            if let Some((up_sql, down_sql)) = Self::split_by_markers(content, up_marker, down_marker) {
                return (up_sql.trim().to_string(), Some(down_sql.trim().to_string()));
            }
        }
        
        // If no separators found, treat entire content as up migration
        (content.to_string(), None)
    }
    
    /// Helper function to split content by up/down markers
    fn split_by_markers(content: &str, up_marker: &str, down_marker: &str) -> Option<(String, String)> {
        // Find the up marker (case insensitive)
        let up_pos = content.to_lowercase().find(&up_marker.to_lowercase())?;
        
        // Find the down marker after the up marker
        let search_start = up_pos + up_marker.len();
        let remaining_content = &content[search_start..];
        let down_pos = remaining_content.to_lowercase().find(&down_marker.to_lowercase())?;
        
        // Extract up SQL (everything after up marker until down marker)
        let up_end = search_start + down_pos;
        let up_sql = &content[up_pos + up_marker.len()..up_end];
        
        // Extract down SQL (everything after down marker)
        let down_start = search_start + down_pos + down_marker.len();
        let down_sql = &content[down_start..];
        
        Some((up_sql.to_string(), down_sql.to_string()))
    }
    
    /// Returns true if this migration has rollback SQL available
    pub fn has_rollback(&self) -> bool {
        self.rollback_sql.is_some() && !self.rollback_sql.as_ref().unwrap().trim().is_empty()
    }
    
    /// Gets the rollback SQL for this migration
    pub fn get_rollback_sql(&self) -> Option<&str> {
        self.rollback_sql.as_deref()
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

