//! Tests for the node-list panel's search caching.

use super::*;

/// Filtering must run when the query changes and not otherwise: rebuilding it
/// per frame would re-filter all 450 operations every repaint.
#[test]
fn filter_recomputes_only_on_query_change() {
    let mut panel = MenuPanel::new();

    panel.query = "blur".to_string();
    assert!(
        panel.refresh_filter_if_needed(),
        "a new query should recompute"
    );
    assert!(
        !panel.refresh_filter_if_needed(),
        "the same query should reuse the cached result"
    );

    panel.query = "noise".to_string();
    assert!(
        panel.refresh_filter_if_needed(),
        "changing the query should recompute"
    );
    assert!(!panel.refresh_filter_if_needed());
}

/// The search cache is built once and covers every operation in the menu.
#[test]
fn search_cache_covers_every_operation() {
    let mut panel = MenuPanel::new();
    panel.query = String::new();
    panel.refresh_filter_if_needed();

    let expected = flatten_operations(&mangler_core::operations::operation_list(), "").len();
    assert_eq!(
        panel.flat.len(),
        expected,
        "the cache should hold every flattened operation"
    );
    assert!(panel.flat.len() > 400, "sanity: the menu is large");
}

/// A query with no matches leaves nothing to draw, including the subgraph row.
#[test]
fn nonsense_query_matches_nothing() {
    let mut panel = MenuPanel::new();
    panel.query = "zzzznotanode".to_string();
    panel.refresh_filter_if_needed();

    assert!(panel.filtered.is_empty());
    assert!(!panel.subgraph_matches);
}
