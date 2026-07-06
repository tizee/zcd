mod data;
pub mod dir;

use anyhow::{Context, Result};
use std::borrow::Cow;
use std::path::Path;

use data::{expand_path, open_file, write_file};
pub use dir::{Dir, DirList, OpsDelegate};

use crate::config::{load_config_from_path, Config};

pub struct Database<'a> {
    delegate: DirList<'a>,
    dirty: bool,
    config: Config,
}

impl OpsDelegate for Database<'_> {
    fn insert_or_update(&mut self, path: Cow<str>) {
        self.delegate.insert_or_update(path);
        self.delegate.age(self.config.max_age as f64);
        self.dirty = true;
    }

    fn delete<P: AsRef<str>>(&mut self, path: P) {
        self.delegate.delete(path);
        self.dirty = true;
    }

    fn query<S: AsRef<str>>(&self, pattern: S) -> Vec<Dir<'_>> {
        self.delegate.query(pattern)
    }

    fn list(&self) -> Vec<Dir<'_>> {
        self.delegate.list()
    }

    fn clear_data(&mut self) {
        self.delegate.clear_data();
    }
}

fn load_datafile(p: &str) -> Result<DirList<'static>> {
    let path = expand_path(p).context("failed to resolve datafile path")?;
    if !path.exists() {
        return Ok(DirList::new());
    }
    let file = open_file(path.as_path()).context("failed to open datafile")?;
    data::from_bytes(file).with_context(|| format!("failed to parse datafile {}", p))
}

impl Database<'_> {
    pub fn new(config_path: &Path) -> Result<Self> {
        let config = load_config_from_path(config_path).context("failed to load config")?;
        let delegate = load_datafile(&config.datafile).context("failed to load data")?;
        Ok(Database {
            config,
            delegate,
            dirty: false,
        })
    }

    pub fn save(&self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }
        let bytes = data::to_bytes(&self.delegate);
        write_file(Path::new(&self.config.datafile), bytes).context("failed to write datafile")
    }

    /// Merge entries from another z-compatible datafile. Existing entries
    /// keep the higher rank and the most recent access time.
    pub fn import(&mut self, path: &Path) -> Result<usize> {
        let incoming = load_datafile(&path.display().to_string())
            .with_context(|| format!("failed to import from {}", path.display()))?;
        let count = incoming.len();
        for (key, dir) in incoming.iter() {
            match self.delegate.get_mut(key) {
                Some(existing) => {
                    existing.rank = existing.rank.max(dir.rank);
                    existing.last_accessed = existing.last_accessed.max(dir.last_accessed);
                }
                None => {
                    self.delegate.insert(key.clone(), dir.clone());
                }
            }
        }
        self.dirty = true;
        Ok(count)
    }

    /// Write all entries to `path` in the z-compatible pipe format.
    pub fn export(&self, path: &Path) -> Result<usize> {
        let bytes = data::to_bytes(&self.delegate);
        write_file(path, bytes)
            .with_context(|| format!("failed to export to {}", path.display()))?;
        Ok(self.delegate.len())
    }

    pub fn clear(&mut self) -> Result<()> {
        self.delegate.clear_data();
        self.dirty = true;

        let datafile = Path::new(&self.config.datafile);
        if datafile.exists() {
            std::fs::remove_file(datafile)
                .with_context(|| format!("failed to remove datafile: {}", datafile.display()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test_db {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_config(dir: &Path, datafile: &Path) -> std::path::PathBuf {
        let config_path = dir.join("config");
        let contents = format!(
            "max_age=5000\ndatafile={}\nexclude_dirs=[]\ndebug=false",
            datafile.to_string_lossy()
        );
        fs::write(&config_path, contents).unwrap();
        config_path
    }

    #[test]
    fn clear_empties_database_and_removes_datafile() {
        let temp_dir = tempdir().unwrap();
        let datafile_path = temp_dir.path().join("zcddata");
        let config_path = write_config(temp_dir.path(), &datafile_path);
        fs::write(&datafile_path, "/dummy/path|1.0|1626969287\n").unwrap();

        let mut db = Database::new(&config_path).unwrap();
        db.insert_or_update("dummy".into());
        db.clear().unwrap();

        assert!(db.list().is_empty());
        assert!(!datafile_path.exists());
    }

    #[test]
    fn clear_succeeds_without_existing_datafile() {
        let temp_dir = tempdir().unwrap();
        let datafile_path = temp_dir.path().join("zcddata");
        let config_path = write_config(temp_dir.path(), &datafile_path);

        let mut db = Database::new(&config_path).unwrap();
        db.insert_or_update("dummy".into());
        db.clear().unwrap();
        assert!(db.list().is_empty());
    }

    #[test]
    fn export_then_import_roundtrips() {
        let temp_dir = tempdir().unwrap();
        let datafile_path = temp_dir.path().join("zcddata");
        let config_path = write_config(temp_dir.path(), &datafile_path);
        let export_path = temp_dir.path().join("exported");

        let mut db = Database::new(&config_path).unwrap();
        db.insert_or_update(temp_dir.path().to_string_lossy().into_owned().into());
        assert_eq!(db.export(&export_path).unwrap(), 1);

        let mut db2 = Database::new(&config_path).unwrap();
        db2.clear_data();
        assert_eq!(db2.import(&export_path).unwrap(), 1);
        assert_eq!(db2.list().len(), 1);
    }

    #[test]
    fn import_merges_keeping_higher_rank_and_newer_access() {
        let temp_dir = tempdir().unwrap();
        let datafile_path = temp_dir.path().join("zcddata");
        let config_path = write_config(temp_dir.path(), &datafile_path);
        fs::write(&datafile_path, "/shared|10.0|200\n/mine|5.0|100\n").unwrap();

        let other = temp_dir.path().join("other-tool-data");
        fs::write(&other, "/shared|3.0|900\n/theirs|7.0|300\n").unwrap();

        let mut db = Database::new(&config_path).unwrap();
        db.import(&other).unwrap();
        db.save().unwrap();

        let text = fs::read_to_string(&datafile_path).unwrap();
        assert!(
            text.contains("/shared|10.0|900"),
            "merge should keep max rank and newest epoch, got:\n{text}"
        );
        assert!(text.contains("/mine|5.0|100"));
        assert!(text.contains("/theirs|7.0|300"));
    }
}
