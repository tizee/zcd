//! Typo-tolerant fuzzy scoring.
//!
//! The core is the fzy dynamic-programming scorer (ported from
//! <https://github.com/jhawthorn/fzy>) extended with a *skip-needle*
//! transition: a needle character that has no counterpart in the haystack
//! costs [`SCORE_SKIP_NEEDLE`] instead of failing the whole match. This
//! makes queries with transposed or mistyped characters (e.g. `labexample`
//! for `lab/exmaple`) still find their target.

use super::score::*;

/// Number of needle characters allowed to go unmatched.
///
/// Short needles stay strict: with 3 or fewer characters, dropping even one
/// makes almost everything match. Longer needles tolerate 25% misses.
fn allowed_skips(needle_len: usize) -> usize {
    if needle_len <= 3 {
        0
    } else {
        needle_len / 4
    }
}

/// Length of the longest common subsequence between needle and haystack.
/// Used as the match gate: enough needle characters must appear in order.
fn lcs_len(needle: &[char], haystack: &[char]) -> usize {
    let m = haystack.len();
    let mut row = vec![0usize; m + 1];
    for &nc in needle {
        let mut prev_diag = 0; // row[j-1] from the previous row
        for j in 1..=m {
            let prev_row_j = row[j];
            row[j] = if nc == haystack[j - 1] {
                prev_diag + 1
            } else {
                row[j].max(row[j - 1])
            };
            prev_diag = prev_row_j;
        }
    }
    row[m]
}

/// Precompute the positional bonus for every haystack character based on
/// its immediate predecessor (directory separator at the start).
fn compute_match_bonus(haystack: &[char]) -> Vec<f64> {
    let mut bonuses = Vec::with_capacity(haystack.len());
    let mut prev = CharType::of('/');
    for &ch in haystack {
        let cur = CharType::of(ch);
        bonuses.push(cur.bonus(prev));
        prev = cur;
    }
    bonuses
}

/// Returns true when `needle` matches `haystack` within the skip tolerance.
pub fn has_match(needle: &str, haystack: &str) -> bool {
    let needle: Vec<char> = needle.to_lowercase().chars().collect();
    let haystack: Vec<char> = haystack.to_lowercase().chars().collect();
    matches_within_tolerance(&needle, &haystack)
}

fn matches_within_tolerance(needle: &[char], haystack: &[char]) -> bool {
    let required = needle.len() - allowed_skips(needle.len());
    haystack.len() >= required && lcs_len(needle, haystack) >= required
}

/// Score `needle` against `haystack`.
///
/// Returns [`SCORE_MAX`] for an exact (case-insensitive) match,
/// [`SCORE_MIN`] when too few needle characters appear in order,
/// and a finite score otherwise (higher is better).
pub fn match_score(needle: &str, haystack: &str) -> f64 {
    if needle.is_empty() {
        return SCORE_MAX;
    }
    let needle: Vec<char> = needle.to_lowercase().chars().collect();
    let haystack: Vec<char> = haystack.to_lowercase().chars().collect();
    if !matches_within_tolerance(&needle, &haystack) {
        return SCORE_MIN;
    }
    if needle == haystack {
        return SCORE_MAX;
    }
    compute_score(&needle, &haystack)
}

/// Dynamic program over (needle prefix, haystack prefix).
///
/// `best[i][j]` is the best score using the first `i` needle chars against
/// the first `j` haystack chars; `matched[i][j]` additionally requires
/// needle char `i` to match haystack char `j`. Transitions:
///
/// - match: `best[i-1][j-1] + bonus` or `matched[i-1][j-1] + consecutive`
/// - gap:   `best[i][j-1] + gap` (trailing gap once all needle chars used)
/// - skip:  `best[i-1][j] + SCORE_SKIP_NEEDLE` (the tolerance extension)
fn compute_score(needle: &[char], haystack: &[char]) -> f64 {
    let n = needle.len();
    let m = haystack.len();
    let match_bonus = compute_match_bonus(haystack);

    // Rolling rows over the needle dimension.
    let mut best_prev = vec![0.0f64; m + 1];
    let mut matched_prev = vec![SCORE_MIN; m + 1];
    // Row 0: no needle chars consumed; gaps before the first match are
    // charged at the leading rate, as in fzy.
    for (j, cell) in best_prev.iter_mut().enumerate().skip(1) {
        *cell = j as f64 * SCORE_GAP_LEADING;
    }

    let mut best_cur = vec![0.0f64; m + 1];
    let mut matched_cur = vec![SCORE_MIN; m + 1];

    for i in 1..=n {
        let gap = if i == n {
            SCORE_GAP_TRAILING
        } else {
            SCORE_GAP_INNER
        };
        best_cur[0] = best_prev[0] + SCORE_SKIP_NEEDLE;
        matched_cur[0] = SCORE_MIN;
        for j in 1..=m {
            matched_cur[j] = if needle[i - 1] == haystack[j - 1] {
                let start = best_prev[j - 1] + match_bonus[j - 1];
                let extend = matched_prev[j - 1] + SCORE_MATCH_CONSECUTIVE;
                start.max(extend)
            } else {
                SCORE_MIN
            };
            best_cur[j] = matched_cur[j]
                .max(best_cur[j - 1] + gap)
                .max(best_prev[j] + SCORE_SKIP_NEEDLE);
        }
        std::mem::swap(&mut best_prev, &mut best_cur);
        std::mem::swap(&mut matched_prev, &mut matched_cur);
    }
    best_prev[m]
}

#[cfg(test)]
mod test_fzy {
    use super::*;

    #[test]
    fn empty_needle_matches_everything() {
        assert_eq!(match_score("", "/a/b/c"), SCORE_MAX);
    }

    #[test]
    fn exact_match_scores_max() {
        assert_eq!(match_score("test", "test"), SCORE_MAX);
    }

    #[test]
    fn case_insensitive_unicode_exact_match() {
        assert_eq!(match_score("über", "ÜBER"), SCORE_MAX);
    }

    #[test]
    fn disjoint_strings_do_not_match() {
        assert_eq!(match_score("xyz", "abc"), SCORE_MIN);
    }

    #[test]
    fn empty_haystack_does_not_match() {
        assert_eq!(match_score("test", ""), SCORE_MIN);
    }

    #[test]
    fn needle_much_longer_than_haystack_does_not_match() {
        assert_eq!(match_score("abcdef", "abc"), SCORE_MIN);
    }

    #[test]
    fn short_needles_stay_strict() {
        // 3 chars or fewer: no skip allowance, otherwise everything matches.
        assert_eq!(match_score("ass", "tags"), SCORE_MIN);
        assert_eq!(match_score("abc", "acb"), SCORE_MIN);
    }

    #[test]
    fn transposed_characters_still_match() {
        // The motivating regression: the user types the intended spelling
        // ("example") but the directory name carries a transposition
        // ("exmaple"), so a strict in-order subsequence never matches.
        let score = match_score("labexample", "/home/user/projects/lab/exmaple");
        assert!(
            score > SCORE_MIN,
            "typo-tolerant matcher should accept a single transposition, got {score}"
        );
    }

    #[test]
    fn correct_spelling_beats_typo() {
        let haystack = "/home/user/projects/lab/exmaple";
        let exact = match_score("labexmaple", haystack);
        let typo = match_score("labexample", haystack);
        assert!(
            exact > typo,
            "exact subsequence {exact} should outrank typo {typo}"
        );
    }

    #[test]
    fn typo_still_prefers_intended_target_over_unrelated_dirs() {
        let target = "/home/user/projects/lab/exmaple";
        let other = "/home/user/packages/zcd-tool";
        let target_score = match_score("labexample", target);
        let other_score = match_score("labexample", other);
        assert!(
            target_score > other_score,
            "target {target_score} should outrank unrelated {other_score}"
        );
    }

    #[test]
    fn too_many_typos_do_not_match() {
        // 8 chars, allowance 2, but 4 chars miss.
        assert_eq!(match_score("abcdwxyz", "abcd"), SCORE_MIN);
    }

    #[test]
    fn prefers_consecutive_matches() {
        assert!(match_score("file", "file") > match_score("file", "filter"));
    }

    #[test]
    fn prefers_beginning_of_words() {
        let word_start = match_score("amor", "app/models/order");
        let mid_word = match_score("amor", "app/models/zrder");
        assert!(word_start > mid_word);
    }

    #[test]
    fn prefers_shorter_candidates() {
        assert!(match_score("test", "tests") > match_score("test", "testing"));
    }

    #[test]
    fn unicode_subsequence_matches() {
        assert!(match_score("路径", "/用户/路径/文档") > SCORE_MIN);
    }

    #[test]
    fn ordinary_subsequence_scores_positive() {
        // A sane subsequence match in a long path must not be filtered out
        // by score-sign checks (regression for the old `> 0.0` filter).
        let score = match_score("zcd", "/home/user/projects/terminal/packages/zcd");
        assert!(score > SCORE_MIN);
    }

    #[test]
    fn lcs_len_basics() {
        let a: Vec<char> = "labexample".chars().collect();
        let b: Vec<char> = "lab/exmaple".chars().collect();
        assert_eq!(lcs_len(&a, &b), 9); // only one of 10 chars misses
    }

    #[test]
    fn allowed_skips_thresholds() {
        assert_eq!(allowed_skips(1), 0);
        assert_eq!(allowed_skips(3), 0);
        assert_eq!(allowed_skips(4), 1);
        assert_eq!(allowed_skips(11), 2);
    }
}
