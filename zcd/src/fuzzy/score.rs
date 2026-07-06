/// Score sentinels. Infinities keep arithmetic sane (`-inf + gap == -inf`)
/// and match upstream fzy, which uses -INFINITY/INFINITY.
pub const SCORE_MIN: f64 = f64::NEG_INFINITY;
pub const SCORE_MAX: f64 = f64::INFINITY;

pub const SCORE_GAP_LEADING: f64 = -0.005;
pub const SCORE_GAP_TRAILING: f64 = -0.005;
pub const SCORE_GAP_INNER: f64 = -0.01;
pub const SCORE_MATCH_CONSECUTIVE: f64 = 1.0;
pub const SCORE_MATCH_SLASH: f64 = 0.9;
pub const SCORE_MATCH_WORD: f64 = 0.8;
pub const SCORE_MATCH_CAPITAL: f64 = 0.7;
pub const SCORE_MATCH_DOT: f64 = 0.6;
/// Penalty for a needle character with no counterpart in the haystack.
/// Must be costlier than any single-character match bonus so that
/// skipping is always a last resort.
pub const SCORE_SKIP_NEEDLE: f64 = -1.0;

#[derive(Debug, Clone, Copy)]
pub enum CharType {
    Upper,
    /// ASCII lowercase and any other Unicode character.
    Lower,
    Digit,
    Slash,
    Dot,
    /// Other separators: space, `-`, `_`.
    Sep,
}

impl CharType {
    pub fn of(ch: char) -> CharType {
        match ch {
            '0'..='9' => CharType::Digit,
            'A'..='Z' => CharType::Upper,
            ' ' | '-' | '_' => CharType::Sep,
            '.' => CharType::Dot,
            '/' => CharType::Slash,
            _ => CharType::Lower,
        }
    }

    /// Bonus awarded for matching a character of this type when the
    /// previous haystack character is `prev`.
    pub fn bonus(self, prev: CharType) -> f64 {
        match self {
            CharType::Upper => match prev {
                CharType::Lower => SCORE_MATCH_CAPITAL,
                CharType::Dot => SCORE_MATCH_DOT,
                CharType::Sep => SCORE_MATCH_WORD,
                CharType::Slash => SCORE_MATCH_SLASH,
                _ => 0.0,
            },
            CharType::Lower | CharType::Digit => match prev {
                CharType::Sep => SCORE_MATCH_WORD,
                CharType::Slash => SCORE_MATCH_SLASH,
                CharType::Dot => SCORE_MATCH_DOT,
                _ => 0.0,
            },
            _ => 0.0,
        }
    }
}
