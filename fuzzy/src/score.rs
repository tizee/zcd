#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum FzyMatchType {
    Leading,
    Trailing,
    Inner,
    Consecutive,
    Slash,
    Word,
    Captial,
    Dot,
}

pub static FZY_SCORE_GAP_LEADING: f64 = -0.005;
pub static FZY_SCORE_GAP_TRAILING: f64 = -0.005;
pub static FZY_SCORE_GAP_INNER: f64 = -0.01;
pub static FZY_SCORE_MATCH_CONSECUTIVE: f64 = 1.0;
pub static FZY_SCORE_MATCH_SLASH: f64 = 0.9;
pub static FZY_SCORE_MATCH_WORD: f64 = 0.8;
pub static FZY_SCORE_MATCH_CAPITAL: f64 = 0.7;
pub static FZY_SCORE_MATCH_DOT: f64 = 0.6;
pub static SCORE_MIN: f64 = f64::MIN;
pub static SCORE_MAX: f64 = f64::MAX;

#[derive(Debug, Clone)]
pub enum FzyCharType {
    Upper,
    // ascii lower case and other Unicode characters
    Lower,
    Digit,
    Slash,
    Dot,
    // other seperator like -,_,/ etc
    Sep,
}

impl FzyCharType {
    pub fn get_type(ch: char) -> FzyCharType {
        match ch {
            '0'..='9' => FzyCharType::Digit,
            'A'..='Z' => FzyCharType::Upper,
            ' ' | '-' | '_' => FzyCharType::Sep,
            '.' => FzyCharType::Dot,
            '/' => FzyCharType::Slash,
            'a'..='z' => FzyCharType::Lower,
            _ => FzyCharType::Lower,
        }
    }

    pub fn get_bonus(&self, last_ch: FzyCharType) -> f64 {
        match self {
            FzyCharType::Upper => match last_ch {
                FzyCharType::Lower => FZY_SCORE_MATCH_CAPITAL,
                FzyCharType::Dot => FZY_SCORE_MATCH_DOT,
                FzyCharType::Sep => FZY_SCORE_MATCH_WORD,
                FzyCharType::Slash => FZY_SCORE_MATCH_SLASH,
                _ => 0.0,
            },
            FzyCharType::Lower | FzyCharType::Digit => match last_ch {
                FzyCharType::Sep => FZY_SCORE_MATCH_WORD,
                FzyCharType::Slash => FZY_SCORE_MATCH_SLASH,
                FzyCharType::Dot => FZY_SCORE_MATCH_DOT,
                _ => 0.0,
            },
            _ => 0.0,
        }
    }
}
