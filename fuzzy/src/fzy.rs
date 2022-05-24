use crate::{
    matcher::{MatchScore, Matcher},
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
    pub(crate) fn new<S: AsRef<str>>(needle: S, haystack: S) -> Self {
        let needle = needle
            .as_ref()
            .to_ascii_lowercase()
            .chars()
            .collect::<Vec<char>>();
        let haystack = haystack
            .as_ref()
            .to_ascii_lowercase()
            .chars()
            .collect::<Vec<char>>();
        let needle_len = needle.len();
        let haystack_len = haystack.len();
        let mut match_bonus = Vec::with_capacity(haystack_len);
        let mut last_ch = FzyCharType::get_type('/');
        for ch in haystack.iter() {
            let cur = FzyCharType::get_type(*ch);
            match_bonus.push(cur.get_bonus(last_ch));
            last_ch = cur;
        }

        Self {
            needle_len,
            haystack_len,
            lower_haystack: haystack,
            lower_needle: needle,
            match_bonus,
        }
    }
}

impl MatchScore for FzyMatcher {
    fn match_score(needle: &str, haystack: &str) -> f64 {
        let fzy = Self::new(needle, haystack);
        if needle.is_empty() {
            return SCORE_MAX;
        }
        let (n, m) = (fzy.needle_len, fzy.haystack_len);
        if n >= m {
            if n == m {
                return SCORE_MAX;
            } else {
                return SCORE_MIN;
            }
        } // 1-d dynamic programming
          // best score fot haystack prefix ending with a match
        let mut dp_match: Vec<Vec<f64>> = vec![vec![0.0; m + 1], vec![0.0; m + 1]];
        // best possible score fot haystack prefix ending at this position
        let mut dp_score: Vec<Vec<f64>> = vec![vec![0.0; m + 1], vec![0.0; m + 1]];
        for i in 0..n {
            let gap_score = if i == fzy.needle_len - 1 {
                FZY_SCORE_GAP_TRAILING
            } else {
                FZY_SCORE_GAP_INNER
            };
            let mut prev_score = SCORE_MIN;
            for j in 0..m {
                if fzy.lower_needle[i] == fzy.lower_haystack[j] {
                    let mut score = SCORE_MIN;
                    if i == 0 {
                        score = (j as f64) * FZY_SCORE_GAP_LEADING + fzy.match_bonus[j];
                    } else if j > 0 {
                        score = (dp_score[0][j - 1] + fzy.match_bonus[j])
                            .max(dp_match[0][j - 1] + FZY_SCORE_MATCH_CONSECUTIVE);
                    }
                    dp_match[1][j] = score;
                    prev_score = score.max(prev_score + gap_score);
                    dp_score[1][j] = prev_score;
                } else {
                    // gap
                    dp_match[1][j] = SCORE_MIN;
                    prev_score += gap_score;
                    dp_score[1][j] = prev_score;
                }
            }
            dp_score.swap(0, 1);
            dp_match.swap(0, 1);
        }
        dp_score[1][m - 1]
    }
}

#[cfg(test)]
mod test_fzy {
    use super::*;
    #[test]
    fn test_prefer_consecutive() {
        //1. prefer consecutive characters
        let s1 = FzyMatcher::match_score("file", "file");
        let s2 = FzyMatcher::match_score("file", "filter");
        assert!(s1 > s2);
    }

    #[test]
    fn test_prefer_beginning_of_words() {
        //2. prefer matching the beginning of words
        let s1 = FzyMatcher::match_score("amor", "app/models/order");
        let s2 = FzyMatcher::match_score("amor", "app/models/zrder");
        assert!(s1 > s2);
    }
    #[test]
    fn test_prefer_shorter_matches() {
        //3. prefer shorter matches
        let s1 = FzyMatcher::match_score("abce", "abcdef");
        let s2 = FzyMatcher::match_score("abce", "abc de");
        println!("{} {}", s1, s2);
        let s1 = FzyMatcher::match_score("abc", "    a b c ");
        let s2 = FzyMatcher::match_score("abc", " a  b  c ");
        println!("{} {}", s1, s2);
        assert!(s1 > s2);
        let s1 = FzyMatcher::match_score("abc", " a b c    ");
        let s2 = FzyMatcher::match_score("abc", " a  b    c ");
        println!("{} {}", s1, s2);
        assert!(s1 > s2);
    }
    #[test]
    fn test_prefer_shorter_candidates() {
        //4. prefer shorter candidates
        let s1 = FzyMatcher::match_score("test", "tests");
        let s2 = FzyMatcher::match_score("test", "testing");
        assert!(s1 > s2);
        assert!(s1 > s2);
    }
}
