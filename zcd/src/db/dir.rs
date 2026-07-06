use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::time::SystemTime;

use itertools::Itertools;

pub type Ranking = f64;
pub type Epoch = u64;

const HOUR: u64 = 3600;
const DAY: u64 = 24 * HOUR;
const WEEK: u64 = 7 * DAY;

/// When ranks age out (see [`DirList::age`]), every rank is multiplied by
/// this factor and entries falling below [`AGE_DROP_THRESHOLD`] are removed.
const AGE_DECAY: f64 = 0.9;
const AGE_DROP_THRESHOLD: f64 = 1.0;

#[derive(Debug, Clone)]
pub struct Dir<'a> {
    pub path: Cow<'a, str>,
    /// Accumulated visit weight (+1 per visit, decayed by aging).
    pub rank: Ranking,
    pub last_accessed: Epoch,
}

impl Ord for Dir<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank
            .total_cmp(&other.rank)
            .then_with(|| self.last_accessed.cmp(&other.last_accessed))
            .then_with(|| self.path.cmp(&other.path))
    }
}

impl PartialOrd for Dir<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Dir<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Dir<'_> {}

impl Display for Dir<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path)
    }
}

/// Frecency: the stored rank scaled by a recency bucket, following zoxide.
///
/// Rank grows without bound (aging keeps the total in check), so two
/// directories with different visit frequencies never converge to the same
/// score — unlike a logarithmic/capped curve.
pub fn frecency(rank: Ranking, now: Epoch, last_accessed: Epoch) -> f64 {
    let elapsed = now.saturating_sub(last_accessed);
    let multiplier = if elapsed < HOUR {
        4.0
    } else if elapsed < DAY {
        2.0
    } else if elapsed < WEEK {
        0.5
    } else {
        0.25
    };
    rank * multiplier
}

#[derive(Debug, Default)]
pub struct DirList<'a>(HashMap<String, Dir<'a>>);

impl<'a, const N: usize> From<[(String, Dir<'a>); N]> for DirList<'a> {
    fn from(v: [(String, Dir<'a>); N]) -> Self {
        DirList(HashMap::from(v))
    }
}

impl DirList<'_> {
    pub fn new() -> Self {
        DirList(HashMap::new())
    }

    /// Decay all ranks once their sum exceeds `max_total_rank`, dropping
    /// entries whose rank becomes negligible. Keeps ranks bounded over time
    /// while preserving their relative order.
    pub fn age(&mut self, max_total_rank: f64) {
        let total: f64 = self.values().map(|d| d.rank).sum();
        if total <= max_total_rank {
            return;
        }
        self.retain(|_, dir| {
            dir.rank *= AGE_DECAY;
            dir.rank >= AGE_DROP_THRESHOLD
        });
    }
}

impl<'a> Deref for DirList<'a> {
    type Target = HashMap<String, Dir<'a>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DirList<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub trait OpsDelegate {
    fn insert_or_update(&mut self, p: Cow<str>);
    fn delete<P: AsRef<str>>(&mut self, p: P);
    fn query<S: AsRef<str>>(&self, pattern: S) -> Vec<Dir<'_>>;
    fn list(&self) -> Vec<Dir<'_>>;
    fn clear_data(&mut self);
}

#[inline]
fn now() -> Epoch {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_secs()
}

/// Bucket a fuzzy score to one decimal so that near-equal match qualities
/// tie and let frecency decide the order.
fn score_bucket(score: f64) -> f64 {
    (score * 10.0).round()
}

impl OpsDelegate for DirList<'_> {
    fn insert_or_update(&mut self, p: Cow<'_, str>) {
        let key = p.to_string();
        let now = now();
        match self.entry(key) {
            Entry::Vacant(e) => {
                e.insert(Dir {
                    path: Cow::Owned(p.into()),
                    rank: 1.0,
                    last_accessed: now,
                });
            }
            Entry::Occupied(mut e) => {
                let dir = e.get_mut();
                dir.rank += 1.0;
                dir.last_accessed = now;
            }
        }
    }

    fn delete<P: AsRef<str>>(&mut self, path: P) {
        self.remove(path.as_ref());
    }

    /// Rank matching directories: primary key is the bucketed fuzzy score,
    /// frecency breaks ties. Returned `Dir.rank` carries the frecency value
    /// so callers can display the effective score.
    fn query<S: AsRef<str>>(&self, pattern: S) -> Vec<Dir<'_>> {
        let pattern = pattern.as_ref();
        let now = now();
        self.values()
            .filter(|dir| Path::new(dir.path.as_ref()).exists())
            .filter_map(|dir| {
                let score = fuzzy::match_score(pattern, &dir.path);
                (score > fuzzy::SCORE_MIN).then(|| {
                    let mut dir = dir.clone();
                    dir.rank = frecency(dir.rank, now, dir.last_accessed);
                    (score_bucket(score), dir)
                })
            })
            .sorted_by(|a, b| {
                b.0.total_cmp(&a.0)
                    .then_with(|| b.1.rank.total_cmp(&a.1.rank))
            })
            .map(|(_, dir)| dir)
            .collect()
    }

    fn list(&self) -> Vec<Dir<'_>> {
        let now = now();
        self.values()
            .filter(|dir| Path::new(dir.path.as_ref()).exists())
            .map(|dir| {
                let mut dir = dir.clone();
                dir.rank = frecency(dir.rank, now, dir.last_accessed);
                dir
            })
            .sorted_by(|a, b| b.rank.total_cmp(&a.rank))
            .collect()
    }

    fn clear_data(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod test_frecency {
    use super::*;

    const NOW: Epoch = 1_600_000_000;

    #[test]
    fn fresher_access_scores_higher() {
        let score_now = frecency(1.0, NOW, NOW);
        let score_hour = frecency(1.0, NOW, NOW - 2 * HOUR);
        let score_day = frecency(1.0, NOW, NOW - 2 * DAY);
        let score_month = frecency(1.0, NOW, NOW - 5 * WEEK);
        assert!(score_now > score_hour);
        assert!(score_hour > score_day);
        assert!(score_day > score_month);
    }

    #[test]
    fn frequency_never_saturates() {
        // The old ln(count)+1 curve converged: heavily visited dirs became
        // indistinguishable. Rank is linear, so 2x visits => 2x frecency.
        let a = frecency(100.0, NOW, NOW);
        let b = frecency(200.0, NOW, NOW);
        assert!(
            (b / a - 2.0).abs() < 1e-9,
            "200-visit dir should stay exactly 2x a 100-visit dir"
        );
    }

    #[test]
    fn recency_multiplier_dominates_small_rank_differences() {
        // A dir visited slightly less often but just now beats a slightly
        // higher-ranked dir untouched for a month.
        let fresh = frecency(10.0, NOW, NOW);
        let stale = frecency(12.0, NOW, NOW - 5 * WEEK);
        assert!(fresh > stale);
    }
}

#[cfg(test)]
mod test_dir_list {
    use super::*;

    fn dir(path: &str, rank: f64, last_accessed: Epoch) -> Dir<'static> {
        Dir {
            path: Cow::Owned(path.to_string()),
            rank,
            last_accessed,
        }
    }

    #[test]
    fn insert_accumulates_rank() {
        let mut list = DirList::new();
        list.insert_or_update("/tmp".into());
        list.insert_or_update("/tmp".into());
        list.insert_or_update("/tmp".into());
        assert_eq!(list.len(), 1);
        let entry = list.get("/tmp").unwrap();
        assert!(
            (entry.rank - 3.0).abs() < 1e-9,
            "rank should be 3, got {}",
            entry.rank
        );
    }

    #[test]
    fn aging_only_triggers_above_threshold() {
        let mut list = DirList::new();
        list.insert("/a".to_string(), dir("/a", 10.0, 0));
        list.age(100.0);
        assert!((list.get("/a").unwrap().rank - 10.0).abs() < 1e-9);
    }

    #[test]
    fn aging_decays_ranks_and_drops_negligible_entries() {
        let mut list = DirList::new();
        list.insert("/hot".to_string(), dir("/hot", 90.0, 0));
        list.insert("/cold".to_string(), dir("/cold", 1.0, 0));
        list.age(50.0);
        assert!((list.get("/hot").unwrap().rank - 81.0).abs() < 1e-9);
        assert!(
            !list.contains_key("/cold"),
            "entries decayed below 1.0 should be pruned"
        );
    }

    #[test]
    fn dir_eq_is_consistent_with_ord() {
        let a = dir("/a", 1.0, 100);
        let b = dir("/b", 1.0, 100);
        assert_ne!(a, b, "different paths must not compare equal");
        assert_eq!(a, a.clone());
        assert_eq!(a.cmp(&a.clone()), Ordering::Equal);
    }

    #[test]
    fn delete_removes_entry() {
        let mut list = DirList::new();
        list.insert_or_update("/tmp".into());
        list.delete("/tmp");
        assert!(list.is_empty());
    }
}

#[cfg(test)]
mod test_query {
    use super::*;
    use tempfile::tempdir;

    fn insert(list: &mut DirList<'static>, path: &std::path::Path, rank: f64, last: Epoch) {
        let p = path.to_str().unwrap().to_string();
        list.insert(
            p.clone(),
            Dir {
                path: Cow::Owned(p),
                rank,
                last_accessed: last,
            },
        );
    }

    #[test]
    fn query_finds_dir_despite_transposed_characters() {
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("lab/exmaple");
        std::fs::create_dir_all(&target).unwrap();

        let mut list = DirList::new();
        insert(&mut list, &target, 1.0, now());
        let res = list.query("labexample");
        assert_eq!(res.len(), 1, "typo query should still find the target");
        assert_eq!(res[0].path, target.to_str().unwrap());
    }

    #[test]
    fn query_skips_nonexistent_paths() {
        let mut list = DirList::new();
        insert(
            &mut list,
            std::path::Path::new("/definitely/not/a/real/dir"),
            1.0,
            now(),
        );
        assert!(list.query("real").is_empty());
    }

    #[test]
    fn equal_match_quality_falls_back_to_frecency() {
        let tmp = tempdir().unwrap();
        let hot = tmp.path().join("work/proj-hot");
        let cold = tmp.path().join("work/proj-cold");
        std::fs::create_dir_all(&hot).unwrap();
        std::fs::create_dir_all(&cold).unwrap();

        let mut list = DirList::new();
        insert(&mut list, &hot, 50.0, now());
        insert(&mut list, &cold, 1.0, now() - 10 * WEEK);
        let res = list.query("work");
        assert_eq!(res.len(), 2);
        assert_eq!(
            res[0].path,
            hot.to_str().unwrap(),
            "frecency should break the tie between equal-quality matches"
        );
    }

    #[test]
    fn better_match_quality_beats_higher_frecency() {
        let tmp = tempdir().unwrap();
        let exact = tmp.path().join("zcd");
        let sloppy = tmp.path().join("dotfiles/z-config-data");
        std::fs::create_dir_all(&exact).unwrap();
        std::fs::create_dir_all(&sloppy).unwrap();

        let mut list = DirList::new();
        insert(&mut list, &exact, 1.0, now() - 10 * WEEK);
        insert(&mut list, &sloppy, 500.0, now());
        let res = list.query("zcd");
        assert_eq!(
            res[0].path,
            exact.to_str().unwrap(),
            "a clearly better match should not be buried by frecency"
        );
    }
}
