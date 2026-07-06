//! Datafile I/O.
//!
//! One plain-text format, shared with the original `z` tool so the data
//! stays portable across z-like tools: one entry per line,
//! `path|rank|last_accessed_epoch`, sorted by rank on write.

use std::borrow::Cow;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

use super::dir::{Dir, DirList, Epoch, Ranking};
use anyhow::{anyhow, Context, Result};
use itertools::Itertools;

pub fn expand_path<P: AsRef<Path>>(p: P) -> Option<PathBuf> {
    let path = p.as_ref();
    if !path.starts_with("~") {
        return Some(path.to_path_buf());
    }
    if path == Path::new("~") {
        return dirs::home_dir();
    }
    dirs::home_dir().map(|mut home| {
        home.push(path.strip_prefix("~").unwrap());
        home
    })
}

fn resolve_path<P: AsRef<Path>>(p: P) -> Result<PathBuf> {
    expand_path(p.as_ref())
        .with_context(|| format!("cannot resolve home directory for {}", p.as_ref().display()))
}

pub fn open_file<P: AsRef<Path>>(p: P) -> Result<File> {
    let path = resolve_path(p)?;
    File::open(&path).with_context(|| format!("Failed to load {}", path.display()))
}

pub fn write_file<P: AsRef<Path>, C: AsRef<[u8]>>(p: P, c: C) -> Result<()> {
    let path = resolve_path(p)?;
    fs::write(&path, c.as_ref()).with_context(|| format!("failed to write into {}", path.display()))
}

/// Serialize entries in the z-compatible pipe format, best rank first.
pub fn to_bytes(data: &DirList) -> Vec<u8> {
    let mut buffer = String::new();
    for dir in data.values().sorted_by(|a, b| Ord::cmp(&b, &a)) {
        buffer.push_str(&format!(
            "{}|{:.1}|{}\n",
            dir.path, dir.rank, dir.last_accessed
        ));
    }
    buffer.into_bytes()
}

/// Parse a z-compatible datafile. Paths may contain `|`, so fields are
/// split from the right.
pub fn from_bytes<T: Read>(f: T) -> Result<DirList<'static>> {
    let mut dir_list = DirList::new();
    let reader = BufReader::new(f);
    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let (path_str, rank, last_accessed) = (|| {
            let mut fields = line.rsplitn(3, '|');
            let last_accessed = fields.next()?;
            let rank = fields.next()?;
            let path_str = fields.next()?;
            Some((path_str, rank, last_accessed))
        })()
        .ok_or_else(|| anyhow!("invalid entry at line {}: {}", line_num + 1, line))?;

        let rank = rank
            .parse::<Ranking>()
            .with_context(|| format!("invalid rank at line {}: {}", line_num + 1, rank))?;
        let last_accessed = last_accessed.parse::<Epoch>().with_context(|| {
            format!(
                "invalid last accessed at line {}: {}",
                line_num + 1,
                last_accessed
            )
        })?;

        dir_list.insert(
            path_str.to_string(),
            Dir {
                path: Cow::Owned(path_str.into()),
                rank,
                last_accessed,
            },
        );
    }
    Ok(dir_list)
}

#[cfg(test)]
mod test_data {
    use super::*;

    #[test]
    fn roundtrip_preserves_entries() {
        let z_data = "/home/user/dev/python/beancount|28|1626969287\n\
/home/user/dev/sandbox/action-timer|11.5|1626960591\n\
/usr/local/share|3|1627435829\n";
        let list = from_bytes(z_data.as_bytes()).unwrap();
        assert_eq!(list.len(), 3);
        assert!((list.get("/usr/local/share").unwrap().rank - 3.0).abs() < 1e-9);

        let bytes = to_bytes(&list);
        let list2 = from_bytes(bytes.as_slice()).unwrap();
        assert_eq!(list2.len(), 3);
        assert!(list2.contains_key("/home/user/dev/python/beancount"));
        assert!(list2.contains_key("/home/user/dev/sandbox/action-timer"));
    }

    #[test]
    fn serialization_orders_by_rank_descending() {
        let data = "/low|1|100\n/high|50|100\n";
        let list = from_bytes(data.as_bytes()).unwrap();
        let text = String::from_utf8(to_bytes(&list)).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[0].starts_with("/high|"));
        assert!(lines[1].starts_with("/low|"));
    }

    #[test]
    fn path_containing_pipe_survives() {
        let data = "/weird|dir|2|100\n";
        let list = from_bytes(data.as_bytes()).unwrap();
        assert!(list.contains_key("/weird|dir"));
    }

    #[test]
    fn blank_lines_are_ignored() {
        let data = "/a|1|100\n\n/b|2|100\n";
        let list = from_bytes(data.as_bytes()).unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn malformed_line_is_reported_with_line_number() {
        let err = from_bytes("not-a-valid-line\n".as_bytes()).unwrap_err();
        assert!(err.to_string().contains("line 1"), "got: {err}");
    }

    #[test]
    fn missing_file_fails_loudly() {
        assert!(open_file(Path::new("/tmpaaasdfsdf/a_file_does_not_exist")).is_err());
    }

    #[test]
    fn expand_tilde_paths() {
        if let Ok(home) = std::env::var("HOME") {
            let mut home_path = Path::new(&home).to_path_buf();
            assert_eq!(home_path, expand_path(Path::new("~")).unwrap());
            home_path.push(".config/zcd");
            assert_eq!(home_path, expand_path(Path::new("~/.config/zcd")).unwrap());
        }
    }
}
