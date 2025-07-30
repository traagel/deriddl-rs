use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Migration {
    pub version: u32,
    pub name: String,
    pub file_path: PathBuf,
    pub sql_content: String,
    pub checksum: String,
    pub applied_at: Option<DateTime<Utc>>,
    pub execution_time_ms: Option<u32>,
    pub success: bool,
}

impl Migration {
    /// Constructs a new `Migration` with computed checksum and default metadata.
    pub fn new(version: u32, name: String, file_path: PathBuf, sql_content: String) -> Self {
        let checksum = Self::compute_checksum(&sql_content);

        Self {
            version,
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
        format!("{:04}_{}.sql", self.version, self.name)
    }

    /// Computes a stable checksum based on the SQL content.
    fn compute_checksum(content: &str) -> String {
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

