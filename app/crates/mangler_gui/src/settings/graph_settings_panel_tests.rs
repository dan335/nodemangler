use super::force_graph_extension;
use std::path::PathBuf;

/// A path that already carries the canonical extension passes through
/// untouched.
#[test]
fn test_force_graph_extension_canonical_unchanged() {
    let path = PathBuf::from("/tmp/my graph.mangler.json");
    assert_eq!(force_graph_extension(path.clone()), path);
}

/// A plain ".json" choice becomes ".mangler.json" — not ".json.mangler.json".
#[test]
fn test_force_graph_extension_plain_json_upgraded() {
    assert_eq!(
        force_graph_extension(PathBuf::from("/tmp/x.json")),
        PathBuf::from("/tmp/x.mangler.json")
    );
}

/// No extension at all gets the canonical one appended.
#[test]
fn test_force_graph_extension_bare_name() {
    assert_eq!(
        force_graph_extension(PathBuf::from("/tmp/x")),
        PathBuf::from("/tmp/x.mangler.json")
    );
}

/// An unrelated extension is kept as part of the stem (the user typed it),
/// with the canonical extension appended after it.
#[test]
fn test_force_graph_extension_other_extension_appended() {
    assert_eq!(
        force_graph_extension(PathBuf::from("/tmp/x.png")),
        PathBuf::from("/tmp/x.png.mangler.json")
    );
}
