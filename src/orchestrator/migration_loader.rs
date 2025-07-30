use crate::model::Migration;
use log::{info, debug, warn};
use std::fs;
use std::path::{Path, PathBuf};
use std::io;

pub struct MigrationLoader;

impl MigrationLoader {
    pub fn load_migrations(migrations_path: &str) -> io::Result<Vec<Migration>> {
        info!("Loading migrations from: {}", migrations_path);
        
        let path = Path::new(migrations_path);
        if !path.exists() {
            warn!("Migrations directory does not exist: {}", migrations_path);
            return Ok(Vec::new());
        }

        let mut migrations = Vec::new();
        let entries = fs::read_dir(path)?;

        for entry in entries {
            let entry = entry?;
            let file_path = entry.path();
            
            if let Some(extension) = file_path.extension() {
                if extension == "sql" {
                    if let Some(migration) = Self::parse_migration_file(&file_path)? {
                        debug!("Loaded migration: {} (version {})", migration.name, migration.version);
                        migrations.push(migration);
                    }
                }
            }
        }

        // Sort by version number
        migrations.sort_by_key(|m| m.version);
        info!("Loaded {} migrations", migrations.len());
        
        Ok(migrations)
    }

    fn parse_migration_file(file_path: &PathBuf) -> io::Result<Option<Migration>> {
        let filename = file_path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        // Parse filename like "0001_init_schema.sql"
        if let Some((version_str, name_part)) = filename.split_once('_') {
            if let Ok(version) = version_str.parse::<u32>() {
                let name = name_part.strip_suffix(".sql").unwrap_or(name_part).to_string();
                let sql_content = fs::read_to_string(file_path)?;
                
                return Ok(Some(Migration::new(
                    version,
                    name,
                    file_path.clone(),
                    sql_content,
                )));
            }
        }

        warn!("Skipping file with invalid name format: {}", filename);
        Ok(None)
    }
}