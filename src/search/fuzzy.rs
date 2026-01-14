//! Fuzzy search implementation

use crate::discovery::AppEntry;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

/// Perform fuzzy search on app entries
pub fn fuzzy_search(apps: &[AppEntry], query: &str) -> Vec<AppEntry> {
    let matcher = SkimMatcherV2::default();
    let query_lower = query.to_lowercase();

    let mut scored: Vec<(i64, &AppEntry)> = apps
        .iter()
        .filter_map(|app| {
            matcher
                .fuzzy_match(&app.name.to_lowercase(), &query_lower)
                .map(|score| (score, app))
        })
        .collect();

    // Sort by score descending
    scored.sort_by(|a, b| b.0.cmp(&a.0));

    // Return top results
    scored
        .into_iter()
        .take(20)
        .map(|(_, app)| app.clone())
        .collect()
}
