use super::*;

// === parse_version ===

#[test]
fn parses_valid_three_part_version() {
    assert_eq!(parse_version("1.2.3"), Some((1, 2, 3)));
    assert_eq!(parse_version("0.0.0"), Some((0, 0, 0)));
    assert_eq!(parse_version("10.20.30"), Some((10, 20, 30)));
}

#[test]
fn rejects_garbage_text() {
    assert_eq!(parse_version("not a version"), None);
    assert_eq!(parse_version("a.b.c"), None);
    assert_eq!(parse_version("v1.2.3"), None);
}

#[test]
fn rejects_empty_string() {
    // Pre-versioning saves deserialize `version` to "" via #[serde(default)].
    assert_eq!(parse_version(""), None);
}

#[test]
fn rejects_two_part_version() {
    assert_eq!(parse_version("1.0"), None);
}

#[test]
fn rejects_four_part_version() {
    assert_eq!(parse_version("1.0.0.1"), None);
}

// === is_newer_than_app ===

#[test]
fn detects_newer_version() {
    assert!(is_newer_than_app("999.0.0"));
}

#[test]
fn detects_older_version() {
    assert!(!is_newer_than_app("0.0.1"));
}

#[test]
fn equal_version_is_not_newer() {
    assert!(!is_newer_than_app(crate::APP_VERSION));
}

#[test]
fn unparseable_file_version_is_never_newer() {
    assert!(!is_newer_than_app(""));
    assert!(!is_newer_than_app("garbage"));
}

// === numeric, not lexicographic, comparison ===

#[test]
fn compares_numerically_not_lexicographically() {
    // Lexicographic string comparison would say "1.0.9" > "1.0.10" (since
    // the character '9' sorts after '1'); numeric comparison of the parsed
    // tuple must get this right.
    assert!(parse_version("1.0.10") > parse_version("1.0.9"));
    assert!(parse_version("2.0.0") > parse_version("1.99.99"));
}
