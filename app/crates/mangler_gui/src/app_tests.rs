use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

/// Monotonic counter so parallel `cargo test` runs never collide on the same
/// temp directory name (mirrors `library_scanner_tests::make_temp_dir`).
static UNIQUE: AtomicU64 = AtomicU64::new(0);

/// Builds a fresh, uniquely-named directory under the OS temp dir. Caller is
/// responsible for cleanup via `std::fs::remove_dir_all`.
fn make_temp_dir(label: &str) -> PathBuf {
    let n = UNIQUE.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "mangler_gui_app_test_{}_{}_{}",
        std::process::id(),
        label,
        n
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// An empty directory gets "untitled 1".
#[test]
fn test_next_untitled_path_empty_dir() {
    let dir = make_temp_dir("empty");
    let path = next_untitled_path(&dir, &HashSet::new());
    assert_eq!(path, dir.join("untitled 1.mangler.json"));
    std::fs::remove_dir_all(&dir).ok();
}

/// Existing "untitled 1.mangler.json"/"untitled 2.mangler.json" files on disk
/// are skipped in favor of the first free number.
#[test]
fn test_next_untitled_path_skips_existing_files() {
    let dir = make_temp_dir("existing_files");
    std::fs::write(dir.join("untitled 1.mangler.json"), "{}").unwrap();
    std::fs::write(dir.join("untitled 2.mangler.json"), "{}").unwrap();

    let path = next_untitled_path(&dir, &HashSet::new());

    assert_eq!(path, dir.join("untitled 3.mangler.json"));
    std::fs::remove_dir_all(&dir).ok();
}

/// A path already claimed by an open-but-unsaved program (`taken`) is
/// skipped too, even though nothing has been written to disk yet — this is
/// the guard against two rapid "new" clicks both probing "untitled 1" before
/// the first program's auto-save has had a chance to write its file.
#[test]
fn test_next_untitled_path_skips_taken_paths() {
    let dir = make_temp_dir("taken");
    let mut taken = HashSet::new();
    taken.insert(dir.join("untitled 1.mangler.json"));

    let path = next_untitled_path(&dir, &taken);

    assert_eq!(path, dir.join("untitled 2.mangler.json"));
    std::fs::remove_dir_all(&dir).ok();
}

/// Disk and `taken` collisions combine: the first number free in both wins.
#[test]
fn test_next_untitled_path_combines_disk_and_taken() {
    let dir = make_temp_dir("combined");
    std::fs::write(dir.join("untitled 1.mangler.json"), "{}").unwrap();
    let mut taken = HashSet::new();
    taken.insert(dir.join("untitled 2.mangler.json"));

    let path = next_untitled_path(&dir, &taken);

    assert_eq!(path, dir.join("untitled 3.mangler.json"));
    std::fs::remove_dir_all(&dir).ok();
}

/// Exactly the paths `next_untitled_path` produces are recognized as
/// auto-created, so empty-graph cleanup targets them.
#[test]
fn test_is_untitled_graph_path_matches_generated_names() {
    let dir = make_temp_dir("untitled_match");
    assert!(is_untitled_graph_path(&next_untitled_path(&dir, &HashSet::new())));
    assert!(is_untitled_graph_path(Path::new("untitled 1.mangler.json")));
    assert!(is_untitled_graph_path(Path::new("untitled 42.mangler.json")));
    assert!(is_untitled_graph_path(&dir.join("untitled 7.mangler.json")));
    std::fs::remove_dir_all(&dir).ok();
}

/// A user-chosen name (or anything not matching the exact `untitled <digits>`
/// shape) is never treated as auto-created, so cleanup can't delete it.
#[test]
fn test_is_untitled_graph_path_rejects_other_names() {
    // Deliberately named graphs.
    assert!(!is_untitled_graph_path(Path::new("my graph.mangler.json")));
    assert!(!is_untitled_graph_path(Path::new("untitled.mangler.json"))); // no number
    assert!(!is_untitled_graph_path(Path::new("untitled 1v2.mangler.json"))); // trailing junk
    assert!(!is_untitled_graph_path(Path::new("untitled 1 copy.mangler.json")));
    assert!(!is_untitled_graph_path(Path::new("prefix untitled 1.mangler.json")));
    // Right name, wrong extension.
    assert!(!is_untitled_graph_path(Path::new("untitled 1.json")));
    assert!(!is_untitled_graph_path(Path::new("untitled 1")));
}
