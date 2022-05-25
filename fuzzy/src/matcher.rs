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
    pub fn has_match<S: AsRef<str>>(needle: S, haystack: S) -> bool {
        let needle = needle.as_ref().to_ascii_lowercase();
        let haystack = haystack.as_ref().to_ascii_lowercase();
        let mut haystack_pt = 0;
        for ch in needle.chars() {
            if haystack_pt < haystack.len() {
                if let Some(next) = haystack[haystack_pt..].find(ch) {
                    haystack_pt += next;
                } else {
                    return false;
                }
            } else {
                return false;
            }
            haystack_pt += 1;
        }
        true
    }

    pub fn match_score<S: AsRef<str>>(&self, needle: S, haystack: S) -> f64 {
        let needle = needle.as_ref();
        let haystack = haystack.as_ref();
        if Matcher::has_match(needle, haystack) {
            return match self {
                Matcher::Naive => {
                    if haystack.contains(needle) {
                        SCORE_MAX
                    } else {
                        SCORE_MIN
                    }
                }
                Matcher::Fzy => FzyMatcher::match_score(needle, haystack),
            };
        }
        SCORE_MIN
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
        assert!(Matcher::has_match("", ""));
        assert!(Matcher::has_match("", "d"));
    }
}
