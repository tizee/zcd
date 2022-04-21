use crate::config::config_file;
use crate::db::dir::{Dir, OpsDelegate};
use crate::db::Database;
use crate::server::check_server_alive;

use anyhow::{Context, Result};
pub struct Client<'a> {
    db: Option<Box<Database<'a>>>,
}

// client side
impl Client<'_> {
    pub fn new() -> Result<Self> {
        if check_server_alive() {
            Ok(Client { db: None })
        } else {
            let config_path = config_file().context("failed to find config file")?;
            let database = Database::new(&config_path).context("failed to init database")?;
            Ok(Client {
                db: Some(Box::new(database)),
            })
        }
    }
    pub fn insert(&mut self, s: &str) -> Result<()> {
        if self.db.is_some() {
            let database = self.db.as_mut().unwrap();
            database.insert_or_update(s.into());
            database.save()
        } else {
            Ok(())
        }
    }

    pub fn delete(&mut self, s: &str) -> Result<()> {
        if self.db.is_some() {
            let database = self.db.as_mut().unwrap();
            database.delete(s);
            database.save()
        } else {
            Ok(())
        }
    }

    pub fn query(&self, pattern: &str) -> Option<Dir> {
        if self.db.is_some() {
            let database = self.db.as_ref().unwrap();
            let res = database.query(pattern);
            if let Some(list) = res {
                if !list.is_empty() {
                    return Some(list[0].clone());
                }
                None
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn list(&self) -> Option<Vec<Dir>> {
        if self.db.is_some() {
            let database = self.db.as_ref().unwrap();
            database.list()
        } else {
            None
        }
    }
}
