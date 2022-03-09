mod data;
pub mod dir;

use anyhow::{Context, Result};
use std::borrow::Cow;
use std::path::Path;

use data::{open_file, DataFile, DataFileIO, ZDataFile, ZcdDataFile};
use dir::{Dir, DirList, OpsDelegate};

use crate::config::{self, config_file, load_config_from_path, load_default_config, ConfigFile};

pub struct Database<'a> {
    delegate: DirList<'a>,
    pub dirty: bool,
    pub config_file: ConfigFile,
}

impl OpsDelegate for Database<'_> {
    fn update_frecent(&mut self) -> &mut Self {
        self.delegate.update_frecent();
        self
    }

    fn insert_or_update(&mut self, path: Cow<str>) {
        self.delegate.insert_or_update(path);
    }

    fn delete<P: AsRef<str>>(&mut self, path: P) {
        self.delegate.delete(path);
    }

    fn query<S: AsRef<str>>(&self, pattern: S) -> Option<Vec<Dir>> {
        self.delegate.query(pattern)
    }

    fn list(&self) -> Option<Vec<Dir>> {
        self.delegate.list()
    }
}

fn load_from_zcd_data_impl(p: &String) -> Result<DirList<'static>> {
    let file = open_file(p).context("failed to read from z data")?;
    let zcd_datafile = &DataFile::Zcd(ZcdDataFile {});
    let dir_list = zcd_datafile
        .from_bytes(file)
        .context(format!("failed to load from z data file {}", p))?;
    Ok(dir_list)
}

pub fn load_from_z_data_impl(p: &String) -> Result<DirList<'static>> {
    let file = open_file(p).context("failed to read from z data")?;
    let z_datafile = &DataFile::Z(ZDataFile {});
    let dir_list = z_datafile
        .from_bytes(file)
        .context("failed to load from z data file")?;
    Ok(dir_list)
}

impl Database<'_> {
    pub fn new(config_path: &Path) -> Result<Self> {
        let config = load_config_from_path(config_path).context("failed to load config")?;
        let config_file = ConfigFile {
            config,
            config_path: config_path.display().to_string(),
        };
        let data_file = config_file.config.datafile.to_string();
        let dir_list = load_from_zcd_data_impl(&data_file).context("failed to load data")?;
        Ok(Database {
            config_file,
            delegate: dir_list,
            dirty: false,
        })
    }

    pub fn load_from_zcd(&mut self, p: &Path) -> Result<()> {
        let dir_list =
            load_from_zcd_data_impl(&p.display().to_string()).context("failed to load data")?;
        self.delegate = dir_list;
        Ok(())
    }

    pub fn load_from_z(&mut self, p: &Path) -> Result<()> {
        let dir_list =
            load_from_z_data_impl(&p.display().to_string()).context("failed to load data")?;
        self.delegate = dir_list;
        Ok(())
    }

    pub fn write_when_dirty() {}
    pub fn save() {}
}

#[cfg(test)]
mod test_db {
    #[test]
    fn test_frecent() {}
}
