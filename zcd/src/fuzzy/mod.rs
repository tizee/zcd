//! Typo-tolerant fuzzy matching for path-like strings.
//!
//! Extends the fzy scoring algorithm with a bounded skip-needle tolerance.
//! See [`match_score`] for the entry point.

mod fzy;
mod score;

pub use fzy::{has_match, match_score};
pub use score::{SCORE_MAX, SCORE_MIN};
