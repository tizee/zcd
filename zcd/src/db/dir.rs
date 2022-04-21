use std::borrow::Cow;
use std::cmp::{Ord, Ordering, PartialEq, PartialOrd};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::time::SystemTime;

use itertools::Itertools;

pub type Ranking = f64;
pub type Epoch = u64;

#[derive(Debug, Clone, PartialOrd)]
pub struct Dir<'a> {
    pub path: Cow<'a, str>,
    pub rank: Ranking,
    pub last_accessed: Epoch,
}

impl Ord for Dir<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        let rank_self = self.rank.round() as u64;
        let rank_other = other.rank.round() as u64;
        let order = rank_self.cmp(&rank_other);
        if order == Ordering::Equal {
            return self.last_accessed.cmp(&other.last_accessed);
        }
        order
    }
}

impl PartialEq for Dir<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.rank == other.rank
    }
}

impl Display for Dir<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut rank = self.rank;
        let path = self.path.to_string();
        if rank > 9999.0 {
            rank = 9999.0;
        }
        write!(f, "{} {}", rank, path)
    }
}

impl Eq for Dir<'_> {}

#[derive(Debug, Default)]
pub struct DirList<'a>(HashMap<String, Dir<'a>>);

impl<'a, const N: usize> From<[(String, Dir<'a>); N]> for DirList<'a> {
    fn from(v: [(String, Dir<'a>); N]) -> Self {
        DirList(HashMap::from(v))
    }
}

impl<'a> DirList<'a> {
    pub fn new() -> Self {
        DirList(HashMap::new())
    }
}

impl<'a> Deref for DirList<'a> {
    type Target = HashMap<String, Dir<'a>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DerefMut for DirList<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub trait OpsDelegate {
    fn update_frecent(&mut self);
    fn insert_or_update(&mut self, p: Cow<str>);
    fn delete<P: AsRef<str>>(&mut self, p: P);
    fn query<S: AsRef<str>>(&self, pattern: S) -> Option<Vec<Dir>>;

    fn list(&self) -> Option<Vec<Dir>>;
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

impl<'a> OpsDelegate for DirList<'a> {
    /// it's an expensive operation
    fn update_frecent(&mut self) {
        let now = now();
        for (_, dir) in self.iter_mut() {
            let rank = frecent(now, dir);
            dir.rank = rank;
        }
    }

    fn insert_or_update(&mut self, p: Cow<'_, str>) {
        let key = p.to_string();
        let now = now();
        if let Entry::Vacant(e) = self.entry(key.clone()) {
            e.insert(Dir {
                path: Cow::Owned(p.into()),
                rank: 1.0,
                last_accessed: now,
            });
        } else {
            let mut dir = self.get_mut(&key).unwrap();
            let rank = frecent(now, dir);
            dir.rank = rank;
            dir.last_accessed = now;
        }
    }

    fn delete<P: AsRef<str>>(&mut self, path: P) {
        let path = path.as_ref();
        self.remove(&path.to_string());
    }

    /// O(N*M)
    fn query<S: AsRef<str>>(&self, pattern: S) -> Option<Vec<Dir>> {
        let pattern = pattern.as_ref();
        let mut candidates = Vec::new();
        for (path, dir) in self.iter() {
            if path.contains(pattern) {
                candidates.push(dir.clone());
            }
        }
        let list_desc_order = candidates
            .into_iter()
            .sorted_by(|a, b| Ord::cmp(&b, &a))
            .collect();
        Some(list_desc_order)
    }

    fn list(&self) -> Option<Vec<Dir>> {
        let mut candidates = Vec::new();
        for (_, dir) in self.iter() {
            candidates.push(dir.clone());
        }
        let list_desc_order = candidates
            .into_iter()
            .sorted_by(|a, b| Ord::cmp(&b, &a))
            .collect();
        Some(list_desc_order)
    }
}

// ranking algorithm: prefer higher rank
fn frecent(now: Epoch, dir: &Dir) -> Ranking {
    let dx = now - dir.last_accessed;
    let mut rank;
    const HOUR: u64 = 60 * 60;
    const DAY: u64 = 24 * HOUR;
    const WEEK: u64 = 7 * DAY;
    // in 1 hour
    if dx < HOUR {
        rank = dir.rank * 4.0;
    } else if dx < DAY {
        // in 24 hour
        rank = dir.rank * 2.0;
    } else if dx < WEEK {
        // in 7 days
        rank = dir.rank * 0.5;
    } else {
        rank = dir.rank * 0.25;
    }
    if rank > 9999.0 {
        rank = 9999.0
    } else if rank < 1.0 {
        rank = 1.0
    }
    rank
}

#[cfg(test)]
mod test_dir {
    use super::*;

    #[test]
    fn test_dir_list() {
        let foo = Dir {
            path: Cow::Owned("/usr/local/bin".into()),
            rank: 1.0,
            last_accessed: now(),
        };
        let foo1 = Dir {
            path: Cow::Owned("/usr/local/bin".into()),
            rank: 1.0,
            last_accessed: now(),
        };
        let foo2 = Dir {
            path: Cow::Owned("/usr/local/bin".into()),
            rank: 1.0,
            last_accessed: now(),
        };
        assert_eq!(foo, foo1);
        let mut dir_list = DirList::new();
        dir_list.insert(foo.path.to_string(), foo);
        dir_list.insert(foo1.path.to_string(), foo1);
        dir_list.insert(foo2.path.to_string(), foo2);
        assert_eq!(dir_list.len(), 2);
    }
}
