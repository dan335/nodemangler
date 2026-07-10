use super::*;

#[test]
fn display_name_strips_mangle_json_suffix() {
    assert_eq!(graph_display_name("my graph.mangle.json"), "my graph");
}

#[test]
fn display_name_strips_plain_json_suffix() {
    assert_eq!(graph_display_name("legacy.json"), "legacy");
}

#[test]
fn display_name_prefers_mangle_json_over_plain_json() {
    // ".mangle.json" ends in ".json" too; make sure the more specific suffix
    // wins and we don't leave a dangling ".mangle".
    assert_eq!(graph_display_name("thing.mangle.json"), "thing");
}

#[test]
fn display_name_returns_unchanged_name_as_is() {
    assert_eq!(graph_display_name("no_extension_here"), "no_extension_here");
}

#[test]
fn display_name_from_path_uses_file_name() {
    let path = std::path::Path::new("/some/dir/my graph.mangle.json");
    assert_eq!(graph_display_name_from_path(path), "my graph");
}

#[test]
fn sanitize_name_preserves_spaces() {
    assert_eq!(sanitize_name("my cool graph"), "my cool graph");
}

#[test]
fn sanitize_name_strips_path_separators() {
    let sanitized = sanitize_name("a/b\\c");
    assert!(!sanitized.contains('/'));
    assert!(!sanitized.contains('\\'));
}

#[test]
fn sanitize_name_strips_illegal_characters() {
    let sanitized = sanitize_name("weird:name*?\"<>|");
    assert!(!sanitized.contains(':'));
    assert!(!sanitized.contains('*'));
    assert!(!sanitized.contains('?'));
    assert!(!sanitized.contains('"'));
    assert!(!sanitized.contains('<'));
    assert!(!sanitized.contains('>'));
    assert!(!sanitized.contains('|'));
}

#[test]
fn sanitize_name_of_only_illegal_characters_is_non_empty_safe_string() {
    // Not literally guaranteed non-empty by the underlying crate for every
    // input, but a purely-illegal-character input should not panic and
    // should not contain any of the stripped characters.
    let sanitized = sanitize_name(":::");
    assert!(!sanitized.contains(':'));
}

#[test]
fn graph_file_name_sanitizes_and_appends_extension() {
    assert_eq!(graph_file_name("my graph"), "my graph.mangle.json");
}

#[test]
fn graph_file_name_strips_illegal_chars_but_keeps_spaces() {
    assert_eq!(graph_file_name("a/b c"), "ab c.mangle.json");
}
