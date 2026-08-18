//! Fuzzy filter over entries, backed by nucleo-matcher (same engine as Helix).

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher};

use crate::model::Entry;

pub struct Fuzzy {
    matcher: Matcher,
}

impl Fuzzy {
    pub fn new() -> Self {
        Self {
            matcher: Matcher::new(Config::DEFAULT),
        }
    }

    /// Indices into `entries` ordered by match score (best first).
    /// Empty query keeps the original newest-first order.
    pub fn filter(&mut self, entries: &[Entry], query: &str) -> Vec<usize> {
        if query.trim().is_empty() {
            return (0..entries.len()).collect();
        }
        let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
        let indexed = entries.iter().enumerate().map(|(i, e)| Indexed(i, &e.raw));
        pattern
            .match_list(indexed, &mut self.matcher)
            .into_iter()
            .map(|(item, _score)| item.0)
            .collect()
    }
}

/// Carries the original index so we can recover it after sorting.
struct Indexed<'a>(usize, &'a str);

impl AsRef<str> for Indexed<'_> {
    fn as_ref(&self) -> &str {
        self.1
    }
}
