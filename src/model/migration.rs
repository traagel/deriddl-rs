use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct Migration {
    pub version: u32,
    pub name: String,
    pub file_path: PathBuf,
    pub sql_content: String,
}

impl Migration {
    pub fn new(version: u32, name: String, file_path: PathBuf, sql_content: String) -> Self {
        Self {
            version,
            name,
            file_path,
            sql_content,
        }
    }
    
    pub fn filename(&self) -> String {
        format!("{:04}_{}.sql", self.version, self.name)
    }
}