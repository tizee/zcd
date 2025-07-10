use std::borrow::Cow;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

use super::dir::{Dir, DirList, Epoch, Ranking};
use anyhow::{anyhow, Context, Result};
use itertools::Itertools;

pub trait DataFileIO {
    fn to_bytes(&self, data: &DirList) -> Result<Vec<u8>>;
    #[allow(clippy::wrong_self_convention)]
    fn from_bytes<T: Read>(&self, f: T) -> Result<DirList>;
}

pub struct ZcdDataFile;
pub struct ZDataFile;
#[allow(dead_code)]
pub enum DataFile {
    Zcd(ZcdDataFile),
    Z(ZDataFile),
}

pub fn expand_path<P: AsRef<Path>>(p: P) -> Option<PathBuf> {
    let path = p.as_ref();
    if !path.starts_with("~") {
        return Some(path.to_path_buf());
    }
    // handle tilde symbol
    if path == Path::new("~") {
        return dirs::home_dir();
    }
    dirs::home_dir().map(|mut h| {
        // root user
        if path == Path::new("/") {
            path.strip_prefix("~").unwrap().to_path_buf()
        } else {
            h.push(path.strip_prefix("~").unwrap());
            h
        }
    })
}
pub fn open_file<P: AsRef<Path>>(p: P) -> Result<File> {
    // resolve symlink
    let path = expand_path(p.as_ref()).unwrap();
    let file = File::open(path.as_path()).context(anyhow!("Failed to load {}", path.display()))?;
    Ok(file)
}

pub fn write_file<P: AsRef<Path>, C: AsRef<[u8]>>(p: P, c: C) -> Result<()> {
    // resolve symlink
    let path = expand_path(p.as_ref()).unwrap();
    let contents = c.as_ref();
    fs::write(path.as_path(), contents)
        .context(anyhow!("failed to write into {}", path.display()))?;
    Ok(())
}

impl DataFileIO for ZcdDataFile {
    fn to_bytes(&self, data: &DirList) -> Result<Vec<u8>> {
        let mut buffer: Vec<u8> = Vec::new();
        for (path, dir) in data.iter().sorted_by(|a, b| Ord::cmp(&b, &a)) {
            let line: Vec<String> = vec![
                path.to_string(),
                format!("{:.1}", dir.rank),
                dir.last_accessed.to_string(),
            ];
            let mut line = line.into_iter().join("|");
            line.push('\n');
            let line_bytes = line.as_bytes();
            buffer.reserve(line.len());
            buffer.extend(line_bytes);
        }
        Ok(buffer)
    }

    fn from_bytes<T: Read>(&self, f: T) -> Result<DirList> {
        let mut dir_list = DirList::new();
        let reader = BufReader::new(f);
        for (line_num, line) in reader.lines().enumerate() {
            let line = line?;
            if line.is_empty() {
                return Err(anyhow!("Empty line at {}!", line_num));
            }
            let (path_str, rank, last_accessed) = (|| {
                let mut split_iter = line.rsplitn(3, '|');
                let last_accessed = split_iter.next()?;
                let rank = split_iter.next()?;
                let path_str = split_iter.next()?;
                Some((path_str, rank, last_accessed))
            })()
            .with_context(|| format!("invalid entry: {}", line))?;

            // conversion
            let rank = rank
                .parse::<Ranking>()
                .with_context(|| format!("invalid rank: {}", rank))?;

            let last_accessed = last_accessed
                .parse::<Epoch>()
                .with_context(|| format!("invalid last accessed: {}", rank))?;

            dir_list.insert(
                path_str.to_string(),
                Dir {
                    path: Cow::Owned(path_str.into()),
                    rank,
                    last_accessed,
                    visit_count: 1,
                },
            );
        }
        Ok(dir_list)
    }
}

// format: path|ranking|last_access_epoch
impl DataFileIO for ZDataFile {
    fn to_bytes(&self, data: &DirList) -> Result<Vec<u8>> {
        let mut buffer: Vec<u8> = Vec::new();
        for (path, dir) in data.iter().sorted_by(|a, b| Ord::cmp(&b, &a)) {
            let line: Vec<String> = vec![
                path.to_string(),
                format!("{:.1}", dir.rank),
                dir.last_accessed.to_string(),
            ];
            let mut line = line.into_iter().join("|");
            line.push('\n');
            let line_bytes = line.as_bytes();
            buffer.reserve(line.len());
            buffer.extend(line_bytes);
        }
        Ok(buffer)
    }

    fn from_bytes<T: Read>(&self, f: T) -> Result<DirList> {
        let mut dir_list = DirList::new();
        let reader = BufReader::new(f);
        for (line_num, line) in reader.lines().enumerate() {
            let line = line?;
            if line.is_empty() {
                return Err(anyhow!("Empty line at {}!", line_num));
            }
            let (path_str, rank, last_accessed) = (|| {
                let mut split_iter = line.rsplitn(3, '|');
                let last_accessed = split_iter.next()?;
                let rank = split_iter.next()?;
                let path_str = split_iter.next()?;
                Some((path_str, rank, last_accessed))
            })()
            .with_context(|| format!("invalid entry: {}", line))?;

            // conversion
            let rank = rank
                .parse::<Ranking>()
                .with_context(|| format!("invalid rank: {}", rank))?;

            let last_accessed = last_accessed
                .parse::<Epoch>()
                .with_context(|| format!("invalid last accessed: {}", rank))?;

            dir_list.insert(
                path_str.to_string(),
                Dir {
                    path: Cow::Owned(path_str.into()),
                    rank,
                    last_accessed,
                    visit_count: 1,
                },
            );
        }
        Ok(dir_list)
    }
}

impl DataFileIO for DataFile {
    fn to_bytes(&self, data: &DirList) -> Result<Vec<u8>> {
        match self {
            DataFile::Z(inner) => inner.to_bytes(data),
            DataFile::Zcd(inner) => inner.to_bytes(data),
        }
    }
    fn from_bytes<T: Read>(&self, f: T) -> Result<DirList> {
        match self {
            DataFile::Z(inner) => inner.from_bytes(f),
            DataFile::Zcd(inner) => inner.from_bytes(f),
        }
    }
}

#[cfg(test)]
mod test_data {
    use super::{
        expand_path, open_file, DataFile, DataFileIO, Dir, DirList, ZDataFile,
    };
    use std::borrow::Cow;
    use std::path::Path;
    #[test]
    fn z_zero_copy() {
        let path = "/usr/bin";
        let dir = Dir {
            path: path.into(),
            rank: 1.0,
            last_accessed: 0,
            visit_count: 1,
        };
        let dirs = DirList::from([(path.to_string(), dir)]);

        let z_datafile = DataFile::Z(ZDataFile {});
        let bytes = z_datafile.to_bytes(&dirs).unwrap();
        let dirs = z_datafile.from_bytes(bytes.as_slice()).unwrap();
        for (_, dir) in dirs.iter() {
            assert!(matches!(dir.path, Cow::Owned(_)));
        }
    }

    #[test]
    fn test_load_data_from_file() {
        let z_data = r"/Users/tizee/dev/grepo_python/beancount|28|1626969287
/Users/tizee/dev/grepo_shell/tz-shell-packages/awk-scripts|30|1626954435
/Users/tizee/dev/playground/action-time|11|1626960591
/Users/tizee/dev/grepo_confs/dotfiles/tizee/nvim|6|1626966988
/Users/tizee/dev/grepo_rust|9|1626967474
/Users/tizee/dev/grepo_confs/dotfiles/tizee/zsh/vendor|9|1626956220
/Users/tizee/dev|1|1626960550
/Users/tizee/dev/grepo_rn/NativeBase-2.13.8|1|1626949060
/Users/tizee/dev/grepo_vim/tz-vim-packages|2|1626967076
/Users/tizee/dev/grepo_shell/z|24|1627435429
/usr/local/share|3|1627435829";
        let z_datafile = DataFile::Z(ZDataFile {});
        if let Ok(list) = z_datafile.from_bytes(z_data.as_bytes()) {
            assert_eq!(list.len(), 11);
            assert!(list.contains_key("/Users/tizee/dev/grepo_python/beancount"));
            assert!(list.contains_key("/usr/local/share"));
            assert!(list.contains_key("/Users/tizee/dev/grepo_confs/dotfiles/tizee/zsh/vendor"));
            let bytes = z_datafile.to_bytes(&list).unwrap();
            let data_str = String::from_utf8(bytes).unwrap();
            if let Ok(list2) = z_datafile.from_bytes(data_str.as_bytes()) {
                assert_eq!(list2.len(), 11);
                assert!(list2.contains_key("/Users/tizee/dev/grepo_python/beancount"));
                assert!(list2.contains_key("/usr/local/share"));
                assert!(
                    list2.contains_key("/Users/tizee/dev/grepo_confs/dotfiles/tizee/zsh/vendor")
                );
            }
        }
    }
    #[test]
    #[should_panic(expected = "Failed to load")]
    fn test_load_file() {
        match open_file(Path::new("/tmpaaasdfsdf/a_file_does_not_exist")) {
            Ok(_) => {}
            Err(_) => panic!("Failed to load"),
        }
    }

    #[test]
    fn test_expand_path() {
        if let Ok(home) = std::env::var("HOME") {
            let mut home_path = Path::new(&home).to_path_buf();
            assert_eq!(home_path, expand_path(Path::new("~")).unwrap());
            home_path.push(".config/zcd");
            assert_eq!(home_path, expand_path(Path::new("~/.config/zcd")).unwrap());
        }
    }
}
