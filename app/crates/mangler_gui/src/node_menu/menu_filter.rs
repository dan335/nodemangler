//! Pure search logic for the Node List's filter box.
//!
//! Operates over the `SearchResult` cache the panel builds once at startup and
//! returns **indices** into it, so a keystroke costs one small allocation
//! rather than cloning matched entries.

use crate::graph::node_search_popup::SearchResult;

/// The label of the one non-operation row, which `flatten_operations` excludes.
pub const SUBGRAPH_LABEL: &str = "subgraph";

/// Rank buckets, best first. Ordering within a bucket is the input order, so
/// results stay stable and grouped by category.
const RANK_NAME_PREFIX: u8 = 0;
const RANK_NAME_CONTAINS: u8 = 1;
const RANK_CATEGORY_ONLY: u8 = 2;

/// Ranked indices into `all` for `query`.
///
/// Every whitespace-separated token must appear in the name **or** the category
/// path, so "noise" and "blur" pull up whole categories — the Tab popup's
/// name-only filter is deliberately not copied here, because this panel is
/// where people browse by category. An empty query returns everything in order.
pub fn filter_ranked(query: &str, all: &[SearchResult]) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return (0..all.len()).collect();
    }
    let tokens: Vec<&str> = query.split_whitespace().collect();

    let mut matches: Vec<(u8, usize)> = Vec::new();
    for (index, result) in all.iter().enumerate() {
        let name = result.name.to_lowercase();
        let category = result.category_path.to_lowercase();

        if !tokens
            .iter()
            .all(|token| name.contains(token) || category.contains(token))
        {
            continue;
        }

        // Rank on the whole query so that typing "blur" puts the node actually
        // called "blur" above one that merely lives in a blur category.
        let rank = if name.starts_with(&query) {
            RANK_NAME_PREFIX
        } else if name.contains(&query) {
            RANK_NAME_CONTAINS
        } else {
            RANK_CATEGORY_ONLY
        };
        matches.push((rank, index));
    }

    // Stable, so input order (i.e. menu order) breaks ties.
    matches.sort_by_key(|(rank, _)| *rank);
    matches.into_iter().map(|(_, index)| index).collect()
}

/// Whether the "subgraph" row should be listed for `query`. It isn't in the
/// `SearchResult` cache, so the panel appends it separately.
pub fn subgraph_matches(query: &str) -> bool {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return true;
    }
    query
        .split_whitespace()
        .all(|token| SUBGRAPH_LABEL.contains(token))
}

/// Message shown when a query matches nothing.
pub fn no_match_text(query: &str) -> String {
    format!("no nodes match \"{}\"", query.trim())
}

#[cfg(test)]
#[path = "menu_filter_tests.rs"]
mod tests;
