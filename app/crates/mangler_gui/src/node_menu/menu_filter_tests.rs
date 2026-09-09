//! Tests for the node-list search filter.

use super::*;
use crate::graph::node_search_popup::flatten_operations;
use mangler_core::operations::operation_list;

fn all() -> Vec<SearchResult> {
    flatten_operations(&operation_list(), "")
}

#[test]
fn empty_query_returns_everything_in_order() {
    let all = all();
    let filtered = filter_ranked("", &all);
    assert_eq!(filtered.len(), all.len());
    assert!(
        filtered.iter().enumerate().all(|(i, &index)| i == index),
        "an empty query should preserve menu order"
    );

    // Whitespace-only is still "no query".
    assert_eq!(filter_ranked("   ", &all).len(), all.len());
}

#[test]
fn returns_valid_indices_into_the_input() {
    let all = all();
    for &index in &filter_ranked("blur", &all) {
        let result = &all[index];
        assert!(
            result.name.to_lowercase().contains("blur")
                || result.category_path.to_lowercase().contains("blur"),
            "'{}' ({}) should match 'blur'",
            result.name,
            result.category_path
        );
    }
}

#[test]
fn matching_is_case_insensitive() {
    let all = all();
    assert_eq!(
        filter_ranked("PERLIN", &all).len(),
        filter_ranked("perlin", &all).len()
    );
    assert!(!filter_ranked("PeRlIn", &all).is_empty());
}

/// Searching a category name finds the nodes inside it, which the Tab popup's
/// name-only filter cannot do.
#[test]
fn matches_category_path() {
    let all = all();
    let filtered = filter_ranked("noise", &all);
    assert!(!filtered.is_empty());

    let by_category_only = filtered
        .iter()
        .map(|&i| &all[i])
        .any(|r| !r.name.to_lowercase().contains("noise"));
    assert!(
        by_category_only,
        "should find nodes whose category, not name, contains 'noise'"
    );
}

#[test]
fn multiple_tokens_are_anded() {
    let all = all();
    let filtered = filter_ranked("noise perlin", &all);
    assert!(!filtered.is_empty());

    for &index in &filtered {
        let result = &all[index];
        let haystack = format!(
            "{} {}",
            result.name.to_lowercase(),
            result.category_path.to_lowercase()
        );
        assert!(haystack.contains("noise") && haystack.contains("perlin"));
    }

    // A token that matches nothing removes the whole result.
    assert!(filter_ranked("perlin zzzznope", &all).is_empty());
}

/// A node actually named for the query outranks one that merely lives in a
/// category of that name.
#[test]
fn name_matches_rank_above_category_matches() {
    let all = all();
    let filtered = filter_ranked("blur", &all);
    assert!(filtered.len() > 1, "expected several blur matches");

    let names: Vec<&str> = filtered.iter().map(|&i| all[i].name.as_str()).collect();
    let exact = names
        .iter()
        .position(|n| n.eq_ignore_ascii_case("blur"))
        .expect("there should be a node called 'blur'");
    let category_only = names
        .iter()
        .position(|n| !n.to_lowercase().contains("blur"));

    if let Some(category_only) = category_only {
        assert!(
            exact < category_only,
            "the node named 'blur' should rank above category-only matches"
        );
    }
}

#[test]
fn no_matches_for_nonsense() {
    let all = all();
    assert!(filter_ranked("zzzznotanode", &all).is_empty());
}

#[test]
fn subgraph_row_matches_its_own_name() {
    assert!(subgraph_matches(""), "empty query lists everything");
    assert!(subgraph_matches("sub"));
    assert!(subgraph_matches("SUBGRAPH"));
    assert!(!subgraph_matches("blur"));
    assert!(!subgraph_matches("sub blur"));
}

#[test]
fn no_match_text_quotes_the_trimmed_query() {
    assert_eq!(no_match_text("  wat "), "no nodes match \"wat\"");
}
