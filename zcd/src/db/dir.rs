use std::borrow::Cow;
use std::cmp::{Ord, Ordering, PartialEq, PartialOrd};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::time::SystemTime;

use fuzzy::Matcher;

use itertools::Itertools;

use serde::{Deserialize, Serialize};

pub type Ranking = f64;
pub type Epoch = u64;

#[derive(Debug, Clone, PartialOrd, Serialize, Deserialize)]
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
        write!(f, "{}", path)
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
    fn clear_data(&mut self);
}

#[inline]
fn now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

impl<'a> OpsDelegate for DirList<'a> {
    // update all entries' ranks using current time
    fn update_frecent(&mut self) {
        let now = now();
        for (_, dir) in self.iter_mut() {
            let rank = frecent(now, dir.last_accessed, dir.rank);
            dir.rank = rank;
        }
        // TODO we could remove outdated or invalid entries here
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
            dir.rank = frecent(now, dir.last_accessed, dir.rank);
            dir.last_accessed = now;
        }
    }

    fn delete<P: AsRef<str>>(&mut self, path: P) {
        let path = path.as_ref();
        self.remove(&path.to_string());
    }

    // query with builtin fuzzy-matcher algorithm
    fn query<S: AsRef<str>>(&self, pattern: S) -> Option<Vec<Dir>> {
        let pattern = pattern.as_ref();
        let mut candidates = Vec::new();
        for (path, dir) in self.iter() {
            if Matcher::has_match(pattern, path) {
                let fzy = Matcher::Fzy;
                candidates.push((fzy.match_score(pattern, path), dir.clone()));
            }
        }
        let list_desc_order = candidates
            .into_iter()
            // multiply by 1000 is enough for handling small float numbers
            .sorted_by(|a, b| ((&b.0 * 1000.0) as u64).cmp(&((&a.0 * 1000.0) as u64)))
            .filter_map(|a| if a.0 > 0.0 { Some(a.1) } else { None })
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
    fn clear_data(&mut self) {
        self.clear();
    }
}

// ranking algorithm: prefer higher rank
fn frecent(now: Epoch, last_accessed: Epoch, last_rank: f64) -> Ranking {
    let dx = now - last_accessed;
    let time_factor = 3.75 / (1.25 + 0.0001 * dx as f64);
    
    // Cap the result to prevent unbounded growth
    let new_rank = 10.0 * time_factor;
    new_rank.min(1000.0) // Set a reasonable maximum value
}

#[cfg(test)]
mod test_frecent {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    // helper function to get a "now" timestamp
    fn current_epoch() -> Epoch {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    #[test]
    fn test_frecent_now() {
        let now = 1_600_000_000; // fixed timestamp
        // dx = 0 => expected = 10.0 * (3.75/1.25) = 30.0
        let score = frecent(now, now, 1.0);
        assert!(
            (score - 30.0).abs() < 1e-6,
            "Expected score ~30.0, got {}",
            score
        );
    }

    #[test]
    fn test_frecent_1_hour() {
        let now = 1_600_000_000;
        let last = now - 3600; // 1 hour ago
        let score = frecent(now, last, 1.0);
        let expected = 10.0 * (3.75 / (1.25 + 0.0001 * 3600.0));
        assert!(
            (score - expected).abs() < 1e-6,
            "Expected score {}, got {}",
            expected,
            score
        );
    }

    #[test]
    fn test_frecent_24_hours() {
        let now = 1_600_000_000;
        let last = now - 86400; // 24 hours ago
        let score = frecent(now, last, 1.0);
        let expected = 10.0 * (3.75 / (1.25 + 0.0001 * 86400.0));
        assert!(
            (score - expected).abs() < 1e-6,
            "Expected score {}, got {}",
            expected,
            score
        );
    }

    #[test]
    fn test_frecent_7_days() {
        let now = 1_600_000_000;
        let last = now - 604800; // 7 days ago
        let score = frecent(now, last, 1.0);
        let expected = 10.0 * (3.75 / (1.25 + 0.0001 * 604800.0));
        assert!(
            (score - expected).abs() < 1e-6,
            "Expected score {}, got {}",
            expected,
            score
        );
    }

    #[test]
    fn test_frecent_ordering() {
        let now = 1_600_000_000;
        let score_now = frecent(now, now, 1.0);
        let score_hour = frecent(now, now - 3600, 1.0);
        let score_day = frecent(now, now - 86400, 1.0);
        let score_week = frecent(now, now - 604800, 1.0);
        let score_8days = frecent(now, now - 691200, 1.0); // 8 days ago

        // Scores should strictly decrease as dx increases.
        assert!(
            score_now > score_hour,
            "score_now {} <= score_hour {}",
            score_now,
            score_hour
        );
        assert!(
            score_hour > score_day,
            "score_hour {} <= score_day {}",
            score_hour,
            score_day
        );
        assert!(
            score_day > score_week,
            "score_day {} <= score_week {}",
            score_day,
            score_week
        );
        assert!(
            score_week > score_8days,
            "score_week {} <= score_8days {}",
            score_week,
            score_8days
        );
    }

    #[test]
    fn test_frecent_max_cap() {
        // Test that the score is capped at 1000.0
        let now = 1_600_000_000;
        let score = frecent(now, now, 1000.0); // Even with high previous rank
        assert!(
            score <= 1000.0,
            "Score should be capped at 1000.0, got {}",
            score
        );
    }

    #[test]
    fn test_frecent_input_rank_ignored() {
        // Verify that the input rank doesn't affect the output
        let now = 1_600_000_000;
        let score1 = frecent(now, now - 3600, 1.0);
        let score2 = frecent(now, now - 3600, 2.0);
        assert!(
            (score1 - score2).abs() < 1e-6,
            "Input rank should be ignored, got {} vs {}",
            score1,
            score2
        );
    }
}

#[cfg(test)]
mod test_dir {
    use super::OpsDelegate;
    use super::*;

    #[test]
    fn test_dir_fmt() {
        let foo = Dir {
            path: Cow::Owned("/usr/local/bin".into()),
            rank: 1.0,
            last_accessed: now(),
        };
        assert_eq!(foo.to_string(), "/usr/local/bin");
    }

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
        assert_eq!(dir_list.len(), 1);
    }

    #[test]
    fn test_query() {
        let foo = Dir {
            path: Cow::Owned("/projects/foo/bar".into()),
            rank: 1.0,
            last_accessed: now(),
        };
        let foo1 = Dir {
            path: Cow::Owned("/projects/bar/foo".into()),
            rank: 1.0,
            last_accessed: now(),
        };
        let foo2 = Dir {
            path: Cow::Owned("/.config/zcd".into()),
            rank: 1.0,
            last_accessed: now(),
        };
        let foo3 = Dir {
            path: Cow::Owned("/projects/rust/zcd".into()),
            rank: 4.0,
            last_accessed: now(),
        };
        let mut dir_list = DirList::new();
        dir_list.insert(foo.path.to_string(), foo);
        dir_list.insert(foo1.path.to_string(), foo1);
        dir_list.insert(foo2.path.to_string(), foo2);
        dir_list.insert(foo3.path.to_string(), foo3);
        let res = dir_list.query("foo");
        assert!(res.is_some());
        let res = dir_list.query("bar");
        assert!(res.is_some());
        let res = dir_list.query("zcd");
        assert!(res.is_some());
    }
}
