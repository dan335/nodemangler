//! Tests for the node search popup's filtering and navigation logic.

use super::*;
use egui::Pos2;
use mangler_core::operations::operation_list;
use mangler_core::value::ValueType;

/// Flattening the full operation list produces a non-empty list
/// where every entry has a name and category path.
#[test]
fn test_flatten_operations() {
    let results = flatten_operations(&operation_list(), "");
    assert!(!results.is_empty(), "flattened list should not be empty");

    for result in &results {
        assert!(!result.name.is_empty(), "every result should have a name");
        assert!(
            !result.category_path.is_empty(),
            "every result should have a category path, got empty for '{}'",
            result.name
        );
    }
}

/// Filtering by "perlin" returns only operations containing "perlin" (case-insensitive).
#[test]
fn test_search_filter_substring() {
    let mut popup = NodeSearchPopup::new();
    popup.search_text = "perlin".to_string();
    popup.update_filtered_results();

    assert!(
        !popup.filtered_results.is_empty(),
        "should find at least one perlin operation"
    );

    for result in &popup.filtered_results {
        assert!(
            result.name.to_lowercase().contains("perlin"),
            "result '{}' should contain 'perlin'",
            result.name
        );
    }
}

/// Empty search string returns all operations.
#[test]
fn test_search_filter_empty_returns_all() {
    let mut popup = NodeSearchPopup::new();
    popup.search_text = String::new();
    popup.update_filtered_results();

    let all_count = flatten_operations(&operation_list(), "").len();
    assert_eq!(
        popup.filtered_results.len(),
        all_count,
        "empty search should return all operations"
    );
}

/// When `from_connection` has `from_value_type: Image` and
/// `from_connection_type: Output`, results only include operations
/// with at least one input accepting `Image`.
#[test]
fn test_type_filter_from_output() {
    let mut popup = NodeSearchPopup::new();
    popup.from_connection = Some(super::TempConnection {
        from_position: Pos2::ZERO,
        from_node_id: "test".to_string(),
        from_connection_index: 0,
        from_connection_type: super::ConnectionType::Output,
        from_value_type: ValueType::Image,
        from_accepts_any_type: false,
    });
    popup.update_filtered_results();

    assert!(
        !popup.filtered_results.is_empty(),
        "should find operations that accept Image"
    );

    for result in &popup.filtered_results {
        let inputs = result.operation.create_inputs();
        let has_compatible_input = inputs.iter().any(|input| {
            input.accepts_any_type
                || input
                    .value
                    .value_type()
                    .valid_conversions()
                    .contains(&ValueType::Image)
        });
        assert!(
            has_compatible_input,
            "operation '{}' should have at least one input accepting Image",
            result.name
        );
    }
}

/// When `from_connection` has `from_connection_type: Input`, results only
/// include operations with at least one output whose type is compatible.
#[test]
fn test_type_filter_from_input() {
    let mut popup = NodeSearchPopup::new();
    popup.from_connection = Some(super::TempConnection {
        from_position: Pos2::ZERO,
        from_node_id: "test".to_string(),
        from_connection_index: 0,
        from_connection_type: super::ConnectionType::Input,
        from_value_type: ValueType::Image,
        from_accepts_any_type: false,
    });
    popup.update_filtered_results();

    assert!(
        !popup.filtered_results.is_empty(),
        "should find operations that output Image-compatible types"
    );

    let valid_from = ValueType::Image.valid_conversions_from();
    for result in &popup.filtered_results {
        let outputs = result.operation.create_outputs();
        let has_compatible_output = outputs.iter().any(|output| {
            valid_from.contains(&output.value.value_type())
        });
        assert!(
            has_compatible_output,
            "operation '{}' should have at least one output compatible with Image input",
            result.name
        );
    }
}

/// `selected_index` never exceeds `filtered_results.len() - 1` after arrow key navigation.
#[test]
fn test_selected_index_clamp() {
    let mut popup = NodeSearchPopup::new();
    popup.search_text = "perlin".to_string();
    popup.update_filtered_results();

    let count = popup.filtered_results.len();
    assert!(count > 0);

    // Set index way past the end
    popup.selected_index = 9999;
    popup.update_filtered_results();
    assert!(
        popup.selected_index < count,
        "selected_index {} should be < {}",
        popup.selected_index,
        count
    );

    // Verify it's clamped to last item
    assert_eq!(popup.selected_index, count - 1);
}

/// Searching for gibberish returns empty results.
#[test]
fn test_search_filter_no_match() {
    let mut popup = NodeSearchPopup::new();
    popup.search_text = "xyzzy_nonexistent_node_zzz".to_string();
    popup.update_filtered_results();

    assert!(
        popup.filtered_results.is_empty(),
        "gibberish search should return no results"
    );
}
