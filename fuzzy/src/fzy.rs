use crate::{
    matcher::{MatchScore},
    score::*,
};

pub struct FzyMatcher {
    pub needle_len: usize,
    pub haystack_len: usize,
    pub lower_needle: Vec<char>,
    pub lower_haystack: Vec<char>,
    pub match_bonus: Vec<f64>,
}

impl FzyMatcher {
    /// Constructs a new FzyMatcher with Unicode‑aware lowercasing.
    pub(crate) fn new<S: AsRef<str>>(needle: S, haystack: S) -> Self {
        let lower_needle: Vec<char> = needle.as_ref().to_lowercase().chars().collect();
        let lower_haystack: Vec<char> = haystack.as_ref().to_lowercase().chars().collect();
        let needle_len = lower_needle.len();
        let haystack_len = lower_haystack.len();
        // Precompute bonus scores as per fzy's algorithm, based solely on the immediate predecessor.
        let match_bonus = compute_match_bonus(&lower_haystack);
        Self {
            needle_len,
            haystack_len,
            lower_needle,
            lower_haystack,
            match_bonus,
        }
    }

    /// Computes the final DP score using two rolling rows.
    /// This method encapsulates the fuzzy matching logic.
    fn compute_score(&self) -> f64 {
        let n = self.needle_len;
        let m = self.haystack_len;
        let mut dp_match = vec![vec![SCORE_MIN; m], vec![SCORE_MIN; m]]; // Best score ending with a match.
        let mut dp_score = vec![vec![SCORE_MIN; m], vec![SCORE_MIN; m]]; // Overall best score.

        for i in 0..n {
            let gap_score = if i == n - 1 { FZY_SCORE_GAP_TRAILING } else { FZY_SCORE_GAP_INNER };
            let mut prev_score = SCORE_MIN;
            for j in 0..m {
                if self.lower_needle[i] == self.lower_haystack[j] {
                    let score = if i == 0 {
                        // For the first needle character, add the leading gap penalty.
                        (j as f64) * FZY_SCORE_GAP_LEADING + self.match_bonus[j]
                    } else if j > 0 {
                        // For subsequent characters, choose the best between starting a new match or continuing.
                        (dp_score[(i - 1) % 2][j - 1] + self.match_bonus[j])
                            .max(dp_match[(i - 1) % 2][j - 1] + FZY_SCORE_MATCH_CONSECUTIVE)
                    } else {
                        SCORE_MIN
                    };
                    dp_match[i % 2][j] = score;
                    prev_score = score.max(prev_score + gap_score);
                    dp_score[i % 2][j] = prev_score;
                } else {
                    dp_match[i % 2][j] = SCORE_MIN;
                    prev_score += gap_score;
                    dp_score[i % 2][j] = prev_score;
                }
            }
            if i < n - 1 {
                dp_match[(i + 1) % 2].iter_mut().for_each(|v| *v = SCORE_MIN);
                dp_score[(i + 1) % 2].iter_mut().for_each(|v| *v = SCORE_MIN);
            }
        }
        dp_score[(n - 1) % 2][m - 1]
    }
}

impl MatchScore for FzyMatcher {
    fn match_score(needle: &str, haystack: &str) -> f64 {
        if needle.is_empty() {
            return SCORE_MAX;
        }
        let matcher = Self::new(needle, haystack);
        if matcher.needle_len > matcher.haystack_len {
            return SCORE_MIN;
        }
        // Handle exact match directly to avoid unnecessary DP processing
        if needle.to_lowercase() == haystack.to_lowercase() {
            return SCORE_MAX;
        }
        matcher.compute_score()
    }
}

/// Precompute bonus scores for each character in the haystack based on its immediate predecessor.
/// This follows the original approach in fzy's algorithm.
fn compute_match_bonus(haystack: &[char]) -> Vec<f64> {
    let mut bonuses = Vec::with_capacity(haystack.len());
    let mut last_ch = FzyCharType::get_type('/'); // Start with the directory separator.
    for &ch in haystack.iter() {
        let cur = FzyCharType::get_type(ch);
        let bonus = cur.get_bonus(last_ch);
        bonuses.push(bonus);
        last_ch = cur;
    }
    bonuses
}

#[cfg(test)]
mod test_fzy {
    use super::*;

    #[test]
    fn test_empty_needle() {
        let score = FzyMatcher::match_score("", "/a/b/c");
        assert_eq!(score, SCORE_MAX);
    }

    #[test]
    fn test_needle_longer_than_haystack() {
        let score = FzyMatcher::match_score("abcdef", "abc");
        assert_eq!(score, SCORE_MIN);
    }

    #[test]
    fn test_exact_match() {
        let score = FzyMatcher::match_score("test", "test");
        assert_eq!(score, SCORE_MAX);
    }

    #[test]
    fn test_non_match() {
        let score = FzyMatcher::match_score("xyz", "abc");
        assert_eq!(score, SCORE_MIN);
    }

    #[test]
    fn test_prefer_consecutive() {
        let score1 = FzyMatcher::match_score("file", "file");
        let score2 = FzyMatcher::match_score("file", "filter");
        assert!(score1 > score2, "Consecutive match should score higher");
    }

    #[test]
    fn test_prefer_beginning_of_words() {
        let score1 = FzyMatcher::match_score("amor", "app/models/order");
        let score2 = FzyMatcher::match_score("amor", "app/models/zrder");
        assert!(score1 > score2, "Match at beginning of word should score higher");
    }

    #[test]
    fn test_prefer_shorter_candidates() {
        let score1 = FzyMatcher::match_score("test", "tests");
        let score2 = FzyMatcher::match_score("test", "testing");
        assert!(score1 > score2, "Shorter candidate should score higher");
    }

    #[test]
    fn test_empty_haystack() {
        let score = FzyMatcher::match_score("test", "");
        assert_eq!(score, SCORE_MIN);
    }

    #[test]
    fn test_unicode_match() {
        // Ensure fuzzy matching works correctly with Chinese characters.
        let score = FzyMatcher::match_score("路径", "/用户/路径/文档");
        assert!(score > SCORE_MIN);
    }

    #[test]
    fn test_case_insensitive_unicode() {
        // Verify that case-insensitive matching works with Unicode accented characters.
        let score = FzyMatcher::match_score("über", "ÜBER");
        assert_eq!(score, SCORE_MAX);
    }
}

