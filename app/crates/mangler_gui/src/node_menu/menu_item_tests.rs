//! Tests for the node-menu tree flattening and the click/drag distance rule.

use super::*;
use mangler_core::operations::operation_list;

fn tree() -> Vec<MenuItem> {
    operation_list()
        .iter()
        .map(|op| MenuItem::new(op.clone(), 0))
        .collect()
}

/// Every category starts collapsed, so only top-level rows are visible.
#[test]
fn visible_rows_starts_collapsed() {
    let items = tree();
    let rows = visible_rows(&items);

    assert_eq!(
        rows.len(),
        items.len(),
        "collapsed tree should show exactly the top-level items"
    );
    for row in &rows {
        assert_eq!(row.level, 0, "row '{}' should be top level", row.label);
    }
}

/// Expanding a category reveals its children, one level deeper.
#[test]
fn visible_rows_expands_children() {
    let mut items = tree();
    let before = visible_rows(&items).len();

    // Find the first category and expand it.
    let index = items
        .iter()
        .position(|item| matches!(item, MenuItem::Category { .. }))
        .expect("operation_list should contain categories");
    MenuItem::toggle_at(&mut items, &[index]);

    let rows = visible_rows(&items);
    assert!(
        rows.len() > before,
        "expanding a category should reveal rows ({} -> {})",
        before,
        rows.len()
    );
    assert!(
        rows.iter().any(|row| row.level == 1),
        "expanded children should be one level deeper"
    );
}

/// Toggling is a round trip, and reaches nested categories by index path.
#[test]
fn toggle_at_round_trips_and_nests() {
    let mut items = tree();
    let index = items
        .iter()
        .position(|item| matches!(item, MenuItem::Category { .. }))
        .unwrap();

    let collapsed = visible_rows(&items).len();
    MenuItem::toggle_at(&mut items, &[index]);
    let expanded = visible_rows(&items).len();
    MenuItem::toggle_at(&mut items, &[index]);
    assert_eq!(
        visible_rows(&items).len(),
        collapsed,
        "toggling twice should return to the original rows"
    );

    // Expand again, then expand the first nested category inside it.
    MenuItem::toggle_at(&mut items, &[index]);
    let nested = {
        let rows = visible_rows(&items);
        rows.iter()
            .find(|row| row.level == 1 && matches!(row.kind, RowKind::Category { .. }))
            .map(|row| row.path.clone())
    };
    if let Some(path) = nested {
        assert_eq!(path.len(), 2, "a nested category's path has two indices");
        MenuItem::toggle_at(&mut items, &path);
        assert!(
            visible_rows(&items).len() > expanded,
            "expanding a nested category should reveal more rows"
        );
    }
}

/// A bad path must not panic or corrupt the tree.
#[test]
fn toggle_at_ignores_bad_paths() {
    let mut items = tree();
    let before = visible_rows(&items).len();
    MenuItem::toggle_at(&mut items, &[]);
    MenuItem::toggle_at(&mut items, &[9999]);
    MenuItem::toggle_at(&mut items, &[0, 9999]);
    assert_eq!(visible_rows(&items).len(), before);
}

/// Every operation in the real menu has a label to draw.
#[test]
fn every_operation_row_has_a_label() {
    fn walk(items: &[MenuItem]) {
        for item in items {
            match item {
                MenuItem::Category { name, items, .. } => {
                    assert!(!name.is_empty(), "category should have a name");
                    walk(items);
                }
                MenuItem::OperationButton { name, .. } => {
                    assert!(!name.is_empty(), "operation should have a name");
                }
                MenuItem::SubgraphButton { name, .. } => {
                    assert!(!name.is_empty(), "subgraph row should have a name");
                }
            }
        }
    }
    walk(&tree());
}

#[test]
fn menu_items_result_is_empty() {
    let mut result = MenuItemsResult::default();
    assert!(result.is_empty());

    result.subgraph_being_created = true;
    assert!(!result.is_empty());

    let mut result = MenuItemsResult::default();
    result.operation_being_created = Some(Operation::OpNumberInputDecimal);
    assert!(!result.is_empty());
}

/// A press that went nowhere is a click, however long it was held. This is the
/// rule that rescues a slow click from egui's duration-based drag promotion.
#[test]
fn abandoned_drag_is_click_by_travel() {
    let press = Pos2::new(100.0, 100.0);

    assert!(
        abandoned_drag_is_click(press, press),
        "zero travel is a click"
    );
    assert!(
        abandoned_drag_is_click(press, Pos2::new(106.0, 100.0)),
        "exactly the threshold is still a click"
    );
    assert!(
        !abandoned_drag_is_click(press, Pos2::new(106.1, 100.0)),
        "past the threshold is a drag"
    );
    assert!(
        !abandoned_drag_is_click(press, Pos2::new(105.0, 105.0)),
        "diagonal travel is measured as distance, not per-axis (7.07px)"
    );
    assert!(
        !abandoned_drag_is_click(press, Pos2::new(400.0, 400.0)),
        "a real drag is not a click"
    );
}

/// The rule works on screen coordinates, so it stays correct when the press and
/// release land in different OS windows.
#[test]
fn abandoned_drag_is_click_across_windows() {
    assert!(abandoned_drag_is_click(
        Pos2::new(2000.0, 300.0),
        Pos2::new(2003.0, 301.0)
    ));
    assert!(!abandoned_drag_is_click(
        Pos2::new(2000.0, 300.0),
        Pos2::new(200.0, 300.0)
    ));
}
