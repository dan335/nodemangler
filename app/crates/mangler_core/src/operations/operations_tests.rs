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
    ];
    for name in expected {
        assert!(
            names.contains(name),
            "operation '{name}' is missing from the node menu (operation_list)"
        );
    }
}

/// Collapses runs of non-alphanumeric characters to a single space and
/// lowercases, so e.g. "Non-Uniform Blur" and "non-uniform blur" compare equal.
fn normalize(s: &str) -> String {
    let mut out = String::new();
    let mut last_was_space = true;
    for c in s.chars() {
        if c.is_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_was_space = false;
        } else if !last_was_space {
            out.push(' ');
            last_was_space = true;
        }
    }
    out.trim_end().to_string()
}

fn collect_operation_names(items: &[OperationListItem], out: &mut Vec<String>) {
    for item in items {
        match item {
            OperationListItem::Category { operation_list_items, .. } => {
                collect_operation_names(operation_list_items, out)
            }
            OperationListItem::Operation { operation } => out.push(operation.settings().name),
            OperationListItem::Subgraph => {}
        }
    }
}

/// Reads the top-level README.md and returns the text of its "## Node
/// Reference" section (up to, but not including, the next "## " heading).
fn read_node_reference_section() -> String {
    let readme_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../README.md");
    let readme = std::fs::read_to_string(readme_path)
        .unwrap_or_else(|e| panic!("failed to read top-level README.md at {readme_path}: {e}"));

    let start = readme
        .find("## Node Reference")
        .expect("README.md is missing a '## Node Reference' section");
    let end = readme[start..]
        .find("\n## ")
        .map(|i| start + i)
        .unwrap_or(readme.len());
    readme[start..end].to_string()
}

/// Parses the `- **Subcategory:** Name1, Name2, ...` bullet lines in the
/// "Node Reference" section into individual node display names.
fn parse_documented_node_names(section: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in section.lines() {
        let line = line.trim();
        if !line.starts_with("- **") {
            continue;
        }
        let parts: Vec<&str> = line.splitn(3, "**").collect();
        let Some(list_part) = parts.get(2) else { continue };
        for name in list_part.split(',') {
            let name = name.trim();
            if !name.is_empty() {
                names.push(name.to_string());
            }
        }
    }
    names
}

/// The top-level README advertises a full "Node Reference" listing every node
/// by name. This pins that list to the actual `operation_list()` registry so
/// newly added operations can't be silently left out of the docs.
#[test]
fn test_all_operations_documented_in_readme() {
    let section = read_node_reference_section();
    let section_normalized = normalize(&section);

    let mut names = Vec::new();
    collect_operation_names(&operation_list(), &mut names);

    let missing: Vec<String> = names
        .into_iter()
        .filter(|name| !section_normalized.contains(&normalize(name)))
        .collect();

    assert!(
        missing.is_empty(),
        "operations missing from the README.md Node Reference section: {missing:?}"
    );
}

/// The inverse of `test_all_operations_documented_in_readme`: catches nodes
/// listed in the README that no longer exist in `operation_list()` — e.g. an
/// operation was renamed or deleted but the README entry was never updated.
#[test]
fn test_no_stale_operations_in_readme() {
    let section = read_node_reference_section();
    let documented = parse_documented_node_names(&section);

    let mut real_names = std::collections::HashSet::new();
    let mut names = Vec::new();
    collect_operation_names(&operation_list(), &mut names);
    for name in names {
        real_names.insert(normalize(&name));
    }

    let extra: Vec<String> = documented
        .into_iter()
        .filter(|name| !real_names.contains(&normalize(name)))
        .collect();

    assert!(
        extra.is_empty(),
        "README.md Node Reference lists nodes that no longer exist in operation_list(): {extra:?}"
    );
}
