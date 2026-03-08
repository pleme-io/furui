//! Furui (篩) — fuzzy matching, scoring, and result ranking engine.
//!
//! A zero-dependency fuzzy matching library inspired by fzf's scoring
//! algorithm. Designed for use in Neovim plugins but usable anywhere.
//!
//! # Quick Start
//!
//! ```
//! use furui::{FuzzyMatcher, MatchResult};
//!
//! let matcher = FuzzyMatcher::new();
//! let result = matcher.score("src/main.rs", "smr");
//! assert!(result.is_some());
//!
//! // Rank a list of candidates
//! let candidates = vec!["src/main.rs", "src/lib.rs", "README.md"];
//! let ranked = matcher.rank("sr", &candidates);
//! assert_eq!(ranked[0].candidate, "src/lib.rs");
//! ```

mod matcher;
mod score;

pub use matcher::{FuzzyMatcher, MatchResult, RankedResult};
pub use score::Score;
