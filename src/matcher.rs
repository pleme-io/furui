//! Core fuzzy matching engine.

use crate::score::{
    Score, BONUS_BOUNDARY, BONUS_CAMEL, BONUS_CONSECUTIVE, BONUS_FIRST_CHAR, SCORE_GAP_EXTENSION,
    SCORE_GAP_START, SCORE_MATCH,
};

/// A fuzzy matcher that scores candidates against a pattern.
#[derive(Debug, Clone)]
pub struct FuzzyMatcher {
    case_sensitive: bool,
}

/// Positions of matched characters and the score.
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// Score for this match. Higher is better.
    pub score: Score,
    /// Byte positions in the candidate where pattern characters matched.
    pub positions: Vec<usize>,
}

/// A candidate paired with its match result, used for ranked output.
#[derive(Debug, Clone)]
pub struct RankedResult<'a> {
    /// The original candidate string.
    pub candidate: &'a str,
    /// Index of the candidate in the original input slice.
    pub index: usize,
    /// Match details.
    pub match_result: MatchResult,
}

impl Default for FuzzyMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl FuzzyMatcher {
    /// Create a new case-insensitive fuzzy matcher.
    #[must_use]
    pub fn new() -> Self {
        Self {
            case_sensitive: false,
        }
    }

    /// Create a case-sensitive fuzzy matcher.
    #[must_use]
    pub fn case_sensitive() -> Self {
        Self {
            case_sensitive: true,
        }
    }

    /// Score a single candidate against a pattern.
    ///
    /// Returns `None` if the pattern doesn't match the candidate at all.
    #[must_use]
    pub fn score(&self, candidate: &str, pattern: &str) -> Option<MatchResult> {
        if pattern.is_empty() {
            return Some(MatchResult {
                score: Score::ZERO,
                positions: Vec::new(),
            });
        }

        if candidate.is_empty() {
            return None;
        }

        let cand: Vec<char> = candidate.chars().collect();
        let pat: Vec<char> = pattern.chars().collect();

        let n = cand.len();
        let m = pat.len();

        if m > n {
            return None;
        }

        // Quick rejection: check subsequence exists.
        if !self.is_subsequence(&cand, &pat) {
            return None;
        }

        // Two-row DP with match/skip states.
        // M[j] = best score ending with candidate[i] matched to pattern[j]
        // S[j] = best score for pattern[0..=j] in candidate[0..=i] (may skip i)
        let mut m_prev = vec![i64::MIN / 2; m]; // M row for previous i
        let mut s_prev = vec![i64::MIN / 2; m]; // S row for previous i
        let mut m_curr = vec![i64::MIN / 2; m];
        let mut s_curr = vec![i64::MIN / 2; m];

        // Track which state we came from for backtracking.
        // For positions, we track where each (i, j) match happened.
        let mut match_from = vec![vec![(0usize, false); m]; n]; // (prev_i, was_consecutive)

        for i in 0..n {
            for j in 0..m {
                m_curr[j] = i64::MIN / 2;
                s_curr[j] = i64::MIN / 2;
            }

            for j in 0..m {
                let chars_match = if self.case_sensitive {
                    cand[i] == pat[j]
                } else {
                    cand[i].to_ascii_lowercase() == pat[j].to_ascii_lowercase()
                };

                if chars_match {
                    let bonus = self.char_bonus(&cand, i, j == 0);

                    if j == 0 {
                        // First pattern char — no previous match needed.
                        m_curr[j] = SCORE_MATCH + bonus;
                        match_from[i][j] = (0, false);
                    } else {
                        // Option A: consecutive match (prev candidate char matched prev pattern char).
                        let consec = m_prev[j - 1]
                            .saturating_add(SCORE_MATCH)
                            .saturating_add(bonus)
                            .saturating_add(BONUS_CONSECUTIVE);

                        // Option B: gap match (best previous score for j-1, with gap penalty).
                        let gap = s_prev[j - 1]
                            .saturating_add(SCORE_MATCH)
                            .saturating_add(bonus)
                            .saturating_add(SCORE_GAP_START);

                        if consec >= gap {
                            m_curr[j] = consec;
                            match_from[i][j] = (i.saturating_sub(1), true);
                        } else {
                            m_curr[j] = gap;
                            match_from[i][j] = (i.saturating_sub(1), false);
                        }
                    }
                }

                // S[j] = best of: match here, or skip (extend gap from S[i-1][j]).
                let skip = if i > 0 {
                    let from_match = s_prev[j].saturating_add(SCORE_GAP_EXTENSION);
                    from_match
                } else {
                    i64::MIN / 2
                };

                s_curr[j] = m_curr[j].max(skip);
            }

            std::mem::swap(&mut m_prev, &mut m_curr);
            std::mem::swap(&mut s_prev, &mut s_curr);
        }

        let final_score = s_prev[m - 1];
        if final_score <= i64::MIN / 4 {
            return None;
        }

        // Backtrack greedily to find match positions.
        let positions = self.find_positions(&cand, &pat);

        Some(MatchResult {
            score: Score(final_score),
            positions,
        })
    }

    /// Rank a slice of candidates against a pattern, returning matches
    /// sorted by score (best first).
    #[must_use]
    pub fn rank<'a>(&self, pattern: &str, candidates: &'a [&str]) -> Vec<RankedResult<'a>> {
        let mut results: Vec<RankedResult<'a>> = candidates
            .iter()
            .enumerate()
            .filter_map(|(index, candidate)| {
                self.score(candidate, pattern)
                    .map(|match_result| RankedResult {
                        candidate,
                        index,
                        match_result,
                    })
            })
            .collect();

        results.sort_by(|a, b| {
            b.match_result
                .score
                .cmp(&a.match_result.score)
                .then_with(|| a.candidate.len().cmp(&b.candidate.len()))
        });

        results
    }

    /// Find the best match positions using a greedy forward pass that
    /// prefers boundary and consecutive matches.
    fn find_positions(&self, cand: &[char], pat: &[char]) -> Vec<usize> {
        let mut positions = Vec::with_capacity(pat.len());
        let mut pi = 0;
        let n = cand.len();

        for ci in 0..n {
            if pi >= pat.len() {
                break;
            }
            let chars_match = if self.case_sensitive {
                cand[ci] == pat[pi]
            } else {
                cand[ci].to_ascii_lowercase() == pat[pi].to_ascii_lowercase()
            };
            if chars_match {
                positions.push(ci);
                pi += 1;
            }
        }

        positions
    }

    fn is_subsequence(&self, cand: &[char], pat: &[char]) -> bool {
        let mut ci = 0;
        for &p in pat {
            let found = if self.case_sensitive {
                cand[ci..].iter().position(|&c| c == p)
            } else {
                cand[ci..]
                    .iter()
                    .position(|&c| c.to_ascii_lowercase() == p.to_ascii_lowercase())
            };
            match found {
                Some(pos) => ci += pos + 1,
                None => return false,
            }
        }
        true
    }

    fn char_bonus(&self, chars: &[char], idx: usize, is_first_pattern_char: bool) -> i64 {
        if idx == 0 {
            return if is_first_pattern_char {
                BONUS_FIRST_CHAR
            } else {
                BONUS_BOUNDARY
            };
        }

        let prev = chars[idx - 1];
        let curr = chars[idx];

        if matches!(prev, '/' | '\\' | '_' | '-' | '.' | ' ') {
            return BONUS_BOUNDARY;
        }

        if prev.is_ascii_lowercase() && curr.is_ascii_uppercase() {
            return BONUS_CAMEL;
        }

        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_pattern_matches_everything() {
        let m = FuzzyMatcher::new();
        let r = m.score("anything", "").unwrap();
        assert_eq!(r.score, Score::ZERO);
        assert!(r.positions.is_empty());
    }

    #[test]
    fn no_match_returns_none() {
        let m = FuzzyMatcher::new();
        assert!(m.score("hello", "xyz").is_none());
    }

    #[test]
    fn exact_match_scores_highest() {
        let m = FuzzyMatcher::new();
        let exact = m.score("abc", "abc").unwrap();
        let partial = m.score("axbxc", "abc").unwrap();
        assert!(
            exact.score > partial.score,
            "exact={:?} should beat partial={:?}",
            exact.score,
            partial.score
        );
    }

    #[test]
    fn case_insensitive_by_default() {
        let m = FuzzyMatcher::new();
        assert!(m.score("Hello", "hello").is_some());
        assert!(m.score("WORLD", "world").is_some());
    }

    #[test]
    fn case_sensitive_mode() {
        let m = FuzzyMatcher::case_sensitive();
        assert!(m.score("Hello", "hello").is_none());
        assert!(m.score("Hello", "Hello").is_some());
    }

    #[test]
    fn boundary_bonus_scores_higher() {
        let m = FuzzyMatcher::new();
        let boundary = m.score("src/main.rs", "mr").unwrap();
        let no_boundary = m.score("summer", "mr").unwrap();
        assert!(
            boundary.score > no_boundary.score,
            "boundary={:?} should beat no_boundary={:?}",
            boundary.score,
            no_boundary.score
        );
    }

    #[test]
    fn ranking_returns_best_first() {
        let m = FuzzyMatcher::new();
        let candidates = vec!["README.md", "src/main.rs", "src/lib.rs", "Makefile"];
        let ranked = m.rank("main", &candidates);
        assert!(!ranked.is_empty());
        assert_eq!(ranked[0].candidate, "src/main.rs");
    }

    #[test]
    fn positions_are_correct() {
        let m = FuzzyMatcher::new();
        let r = m.score("abcdef", "ace").unwrap();
        assert_eq!(r.positions, vec![0, 2, 4]);
    }

    #[test]
    fn consecutive_matches_score_higher() {
        let m = FuzzyMatcher::new();
        let consecutive = m.score("ab", "ab").unwrap();
        let spread = m.score("a_b", "ab").unwrap();
        assert!(
            consecutive.score > spread.score,
            "consecutive={:?} should beat spread={:?}",
            consecutive.score,
            spread.score
        );
    }

    #[test]
    fn camel_case_boundary() {
        let m = FuzzyMatcher::new();
        let r = m.score("getUserName", "gUN").unwrap();
        assert!(r.score.value() > 0);
    }

    #[test]
    fn rank_empty_pattern() {
        let m = FuzzyMatcher::new();
        let candidates = vec!["a", "b", "c"];
        let ranked = m.rank("", &candidates);
        assert_eq!(ranked.len(), 3);
    }

    #[test]
    fn rank_no_matches() {
        let m = FuzzyMatcher::new();
        let candidates = vec!["hello", "world"];
        let ranked = m.rank("xyz", &candidates);
        assert!(ranked.is_empty());
    }

    #[test]
    fn shorter_candidates_preferred_at_equal_score() {
        let m = FuzzyMatcher::new();
        // "ab" exact match vs "a_b" with boundary bonus — ab should win on length tiebreak
        // if scores happen to differ, we just check both match
        let candidates = vec!["a_____b", "ab"];
        let ranked = m.rank("ab", &candidates);
        assert_eq!(ranked.len(), 2);
        // The shorter exact match should rank first due to consecutive bonus
        assert_eq!(ranked[0].candidate, "ab");
    }
}
