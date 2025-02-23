use crate::fzy::FzyMatcher;
use crate::score::*;

pub trait MatchScore {
    fn match_score(needle: &str, haystack: &str) -> f64;
}

pub enum Matcher {
    Naive,
    Fzy,
}

impl Matcher {
    /// Checks if all characters in the needle appear in the haystack in order.
    /// Uses Unicode‑aware lowercasing for consistency.
    pub fn has_match<S: AsRef<str>>(needle: S, haystack: S) -> bool {
        let needle = needle.as_ref().to_lowercase();
        let haystack = haystack.as_ref().to_lowercase();
        let mut haystack_pt = 0;

        for ch in needle.chars() {
            if haystack_pt < haystack.len() {
                if let Some((next_index, _)) = haystack[haystack_pt..].char_indices().find(|(_, c)| *c == ch) {
                    haystack_pt += next_index;
                } else {
                    return false;
                }
            } else {
                return false;
            }
            haystack_pt += ch.len_utf8();
        }
        true
    }

    /// Computes a match score between the needle and haystack.
    /// For a successful match, the score is determined either by a naive substring
    /// check (for Matcher::Naive) or the FzyMatcher algorithm.
    pub fn match_score<S: AsRef<str>>(&self, needle: S, haystack: S) -> f64 {
        let needle = needle.as_ref();
        let haystack = haystack.as_ref();
        if Matcher::has_match(needle, haystack) {
            match self {
                Matcher::Naive => {
                    if haystack.to_lowercase().contains(&needle.to_lowercase()) {
                        SCORE_MAX
                    } else {
                        SCORE_MIN
                    }
                }
                Matcher::Fzy => FzyMatcher::match_score(needle, haystack),
            }
        } else {
            SCORE_MIN
        }
    }
}

#[cfg(test)]
mod test_matcher {
    use super::*;

    #[test]
    fn test_exact_match() {
        assert!(Matcher::has_match("a", "a"));
    }

    #[test]
    fn test_partial_match() {
        assert!(Matcher::has_match("a", "ab"));
        assert!(Matcher::has_match("a", "ba"));
    }

    #[test]
    fn test_match_with_delimiters() {
        assert!(Matcher::has_match("abc", "/a/b/c"));
    }

    #[test]
    fn test_non_match() {
        assert!(!Matcher::has_match("abc", ""));
        assert!(!Matcher::has_match("abc", "d"));
        assert!(!Matcher::has_match("ass", "tags"));
    }

    #[test]
    fn test_empty() {
        // An empty needle always matches.
        assert!(Matcher::has_match("", ""));
        assert!(Matcher::has_match("", "d"));
    }

    #[test]
    fn test_unicode_naive_match() {
        // Verify that the naive matcher works with Unicode.
        assert!(Matcher::has_match("路径", "/用户/路径/文档"));
    }
}

