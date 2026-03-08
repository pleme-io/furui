//! Scoring constants and types for fuzzy matching.

/// A match score. Higher is better.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Score(pub i64);

impl Score {
    pub const ZERO: Self = Self(0);

    #[must_use]
    pub const fn value(self) -> i64 {
        self.0
    }
}

// Bonus/penalty weights inspired by fzf's scoring model.

/// Bonus for an exact character match.
pub(crate) const SCORE_MATCH: i64 = 16;

/// Penalty per gap (non-matching character between matches).
pub(crate) const SCORE_GAP_START: i64 = -3;

/// Penalty for each additional consecutive gap character.
pub(crate) const SCORE_GAP_EXTENSION: i64 = -1;

/// Bonus when a match occurs at a word boundary (after `/`, `_`, `-`, `.`, or uppercase transition).
pub(crate) const BONUS_BOUNDARY: i64 = SCORE_MATCH / 2;

/// Bonus when the match is the first character of the candidate.
pub(crate) const BONUS_FIRST_CHAR: i64 = SCORE_MATCH / 2 + 2;

/// Bonus for consecutive character matches.
pub(crate) const BONUS_CONSECUTIVE: i64 = SCORE_MATCH / 2 + 1;

/// Bonus for camelCase boundary (lowercase → uppercase transition).
pub(crate) const BONUS_CAMEL: i64 = BONUS_BOUNDARY - 1;
