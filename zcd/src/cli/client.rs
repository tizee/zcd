use crate::config::config_file;
use crate::db::dir::{Dir, OpsDelegate};
use crate::db::Database;

use anyhow::{Context, Result};

pub struct Client {
    db: Database<'static>,
}

impl Client {
    pub fn new() -> Result<Self> {
        let config_path = config_file().context("failed to find config file")?;
        let database = Database::new(&config_path).context("failed to init database")?;
        Ok(Client { db: database })
    }

    pub fn insert(&mut self, s: &str) -> Result<()> {
        self.db.insert_or_update(s.into());
        self.db.save()
    }

    pub fn delete(&mut self, s: &str) -> Result<()> {
        self.db.delete(s);
        self.db.save()
    }

    pub fn query(&self, pattern: &str) -> Result<Option<Dir>> {
        let res = self.db.query(pattern);
        if let Some(list) = res {
            if !list.is_empty() {
                return Ok(Some(list[0].clone()));
            }
        }
        Ok(None)
    }

    pub fn list(&self) -> Result<Option<Vec<Dir>>> {
        Ok(self.db.list())
    }

    pub fn clear(&mut self) -> Result<()> {
        self.db.clear()?;
        self.db.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;
    // This test creates a temporary config file and data file,
    // then verifies that insert, query, and delete work as expected.
    #[test]
    fn test_client_insert_query_delete() {
        let temp_dir = tempdir().unwrap();
        let config_path: PathBuf = temp_dir.path().join("config");
        let datafile_path: PathBuf = temp_dir.path().join("zcddata");

        // Write a simple config file with the datafile path set appropriately.
        let config_contents = format!(
            r#"max_age=5000
datafile={}
exclude_dirs=[]
debug=false "#,
            datafile_path.display()
        );
        fs::write(&config_path, config_contents).unwrap();
        // Force the config lookup to use our temporary file.
        std::env::set_var("ZCD_CONFIG_FILE", config_path.to_str().unwrap());

        let mut client = Client::new().unwrap();
        let entry = "/tmp/test-entry";
        client.insert(entry).unwrap();

        let query_result = client.query("test").unwrap();
        assert!(query_result.is_some());
        assert_eq!(query_result.unwrap().path, entry);

        client.delete(entry).unwrap();
        let query_result = client.query("test").unwrap();
        assert!(query_result.is_none());
    }

}
