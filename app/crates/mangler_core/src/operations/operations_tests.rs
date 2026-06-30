use super::{operation_list, default_image, OperationListItem};

#[test]
fn test_operation_list_not_empty() {
    let list = operation_list();
    assert!(!list.is_empty());
}

#[test]
fn test_default_image() {
    let img = default_image();
    assert_eq!(img.width(), 1);
    assert_eq!(img.height(), 1);
}

#[test]
fn test_all_operations_have_valid_settings() {
    fn check_items(items: &[OperationListItem]) {
        for item in items {
            match item {
                OperationListItem::Category { name, operation_list_items } => {
                    assert!(!name.is_empty());
                    check_items(operation_list_items);
                }
                OperationListItem::Operation { operation } => {
                    let settings = operation.settings();
                    assert!(!settings.name.is_empty());
                    let _inputs = operation.create_inputs();
                    let _outputs = operation.create_outputs();
                }
                OperationListItem::Subgraph => {}
            }
        }
    }
    check_items(&operation_list());
}

/// Every node must be reachable from the add-node menu / search — both are
/// driven by `operation_list()` in the GUI (`menu_panel.rs`,
/// `node_search_popup.rs`). A node registered in the `operations!` macro but
/// left out of `operation_list()` compiles and unit-tests fine yet can never
/// be placed in a graph. This pins the recently added nodes to the menu.
#[test]
fn test_added_nodes_are_reachable_in_menu() {
    fn collect_names(items: &[OperationListItem], out: &mut std::collections::HashSet<String>) {
        for item in items {
            match item {
                OperationListItem::Category { operation_list_items, .. } => {
                    collect_names(operation_list_items, out)
                }
                OperationListItem::Operation { operation } => {
                    out.insert(operation.settings().name);
                }
                OperationListItem::Subgraph => {}
            }
        }
    }

    let mut names = std::collections::HashSet::new();
    collect_names(&operation_list(), &mut names);

    let expected = [
        // adjustments
        "saturation", "threshold", "vignette", "white balance", "color balance", "selective color",
        // transform
        "polar coordinates", "swirl", "spherize", "perspective",
        // filter
        "convolution", "morphological gradient", "top hat", "black hat",
        // noise
        "wave", "blue noise", "curl noise",
        // video input
        "video from url",
    ];
    for name in expected {
        assert!(
            names.contains(name),
            "operation '{name}' is missing from the node menu (operation_list)"
        );
    }
}
