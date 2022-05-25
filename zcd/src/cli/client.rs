use crate::config::config_file;
use crate::db::dir::{Dir, OpsDelegate};
use crate::db::Database;
use crate::server::{check_server_alive, DbClient, ServerOps};

use anyhow::{Context, Result};
pub struct OnceClient<'a> {
    db: Option<Box<Database<'a>>>,
}

pub enum Client<'a> {
    Cli(Box<Database<'a>>),
    ServerCli(DbClient),
}

impl Client<'_> {
    pub fn new() -> Result<Self> {
        if check_server_alive() {
            Ok(Client::ServerCli(
                DbClient::new().context("failed to init server client")?,
            ))
        } else {
            let config_path = config_file().context("failed to find config file")?;
            let database = Database::new(&config_path).context("failed to init database")?;
            Ok(Client::Cli(Box::new(database)))
        }
    }
    pub fn insert(&mut self, s: &str) -> Result<()> {
        match self {
            Client::Cli(db) => {
                let database = db.as_mut();
                database.insert_or_update(s.into());
                database.save()
            }
            Client::ServerCli(cli) => cli.insert(s).context("failed to send message"),
        }
    }

    pub fn delete(&mut self, s: &str) -> Result<()> {
        match self {
            Client::Cli(db) => {
                let database = db.as_mut();
                database.delete(s);
                database.save()
            }
            Client::ServerCli(cli) => cli.delete(s).context("failed to send message"),
        }
    }

    pub fn query(&self, pattern: &str) -> Result<Option<Dir>> {
        let res = match self {
            Client::Cli(db) => {
                let database = db.as_ref();
                database.query(pattern)
            }
            Client::ServerCli(cli) => cli.query(pattern).context("failed to send message")?,
        };
        if let Some(list) = res {
            if !list.is_empty() {
                return Ok(Some(list[0].clone()));
            }
            Ok(None)
        } else {
            Ok(None)
        }
    }

    pub fn list(&self) -> Result<Option<Vec<Dir>>> {
        match self {
            Client::Cli(db) => {
                let database = db.as_ref();
                Ok(database.list())
            }
            Client::ServerCli(cli) => cli.list(),
        }
    }
}
