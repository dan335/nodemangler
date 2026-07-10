use super::*;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

// Only the pure functions (`scan_folder`, `is_graph_file`,
// `graph_display_name`) are tested here. `Scanner::spawn` needs a live
// `egui::Context` and a background thread, so it's exercised manually
// (per the plan's Phase 4 verification step), not in this unit-test file.

/// Monotonic counter so parallel `cargo test` runs never collide on the same
/// temp directory name.
static UNIQUE: AtomicU64 = AtomicU64::new(0);

/// Builds a fresh, uniquely-named directory under the OS temp dir, mirroring
/// the `std::env::temp_dir()` + unique-name-per-test precedent used in
/// `mangler_core/src/graph_tests.rs` (`test_save_and_load_round_trip`).
/// Caller is responsible for cleanup via `std::fs::remove_dir_all`.
fn make_temp_dir(label: &str) -> PathBuf {
    let n = UNIQUE.fetch_add(1, AtomicOrdering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "mangler_gui_scanner_test_{}_{}_{}",
        std::process::id(),
        label,
        n
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Recursively creates `depth` nested single-child folders (all named `d`)
/// starting at `base`, returning the deepest folder's path. Used to exercise
/// the `MAX_SCAN_DEPTH` cap without hand-writing every level.
fn make_chain(base: &Path, depth: usize) -> PathBuf {
    let mut current = base.to_path_buf();
    for _ in 0..depth {
        current = current.join("d");
        std::fs::create_dir(&current).unwrap();
    }
    current
}

// === scan_folder: nesting, sorting, graph filtering ===

/// A root with subfolders and graph files at multiple levels is walked
/// recursively, and both folders and graphs come back sorted
/// case-insensitively by name.
#[test]
fn test_scan_folder_nested_and_sorted() {
    let root = make_temp_dir("nested");

    // Subfolders in a deliberately non-alphabetical, mixed-case order.
    std::fs::create_dir(root.join("Zebra")).unwrap();
    std::fs::create_dir(root.join("apple")).unwrap();

    // Graphs at the root, also mixed-case / non-alphabetical.
    std::fs::write(root.join("b.mangler.json"), "{}").unwrap();
    std::fs::write(root.join("A.mangler.json"), "{}").unwrap();

    // A graph nested one level down, to confirm recursion happened.
    std::fs::write(root.join("Zebra").join("nested.mangler.json"), "{}").unwrap();

    let mut budget = MAX_ENTRIES_PER_LIBRARY;
    let scan = scan_folder(&root, 0, &mut budget).unwrap();

    assert_eq!(scan.folders.len(), 2);
    assert_eq!(scan.folders[0].name, "apple"); // "apple" < "Zebra" case-insensitively
    assert_eq!(scan.folders[1].name, "Zebra");

    assert_eq!(scan.graphs.len(), 2);
    assert_eq!(scan.graphs[0].name, "A"); // "A" < "b" case-insensitively
    assert_eq!(scan.graphs[1].name, "b");

    // The nested graph was found while recursing into "Zebra".
    let zebra = &scan.folders[1];
    assert_eq!(zebra.graphs.len(), 1);
    assert_eq!(zebra.graphs[0].name, "nested");

    assert!(!scan.truncated);

    std::fs::remove_dir_all(&root).unwrap();
}

/// Files that don't end in the exact `.mangler.json` suffix are not listed as
/// graphs, even if they look similar.
#[test]
fn test_scan_folder_ignores_non_graph_files() {
    let root = make_temp_dir("ignored");

    std::fs::write(root.join("notes.txt"), "hello").unwrap();
    std::fs::write(root.join("graph.json"), "{}").unwrap(); // wrong suffix
    std::fs::write(root.join("graph.mangler.jsonx"), "{}").unwrap(); // wrong suffix
    std::fs::write(root.join("real.mangler.json"), "{}").unwrap(); // the only real one

    let mut budget = MAX_ENTRIES_PER_LIBRARY;
    let scan = scan_folder(&root, 0, &mut budget).unwrap();

    assert_eq!(scan.graphs.len(), 1);
    assert_eq!(scan.graphs[0].name, "real");

    std::fs::remove_dir_all(&root).unwrap();
}

/// Scanning a path that doesn't exist returns an `Err`, which the caller
/// (`scan_library_root`) turns into `LibraryScan::Unavailable`.
#[test]
fn test_scan_folder_missing_root_returns_err() {
    let missing = std::env::temp_dir().join("mangler_gui_scanner_test_does_not_exist_xyz");
    let mut budget = MAX_ENTRIES_PER_LIBRARY;
    let result = scan_folder(&missing, 0, &mut budget);
    assert!(result.is_err());
}

/// Folders nested deeper than `MAX_SCAN_DEPTH` are not descended into; the
/// folder at the depth cap is marked `truncated` instead.
#[test]
fn test_scan_folder_depth_cap_sets_truncated() {
    let root = make_temp_dir("depth_cap");
    // One level deeper than the cap so the folder *at* the cap still has an
    // undescended child to flag as truncated.
    make_chain(&root, MAX_SCAN_DEPTH + 1);

    let mut budget = MAX_ENTRIES_PER_LIBRARY;
    let scan = scan_folder(&root, 0, &mut budget).unwrap();

    // Walk down the single-child chain: `scan` is depth 0 (the root),
    // `scan.folders[0]` is the folder scanned at depth 1, and so on, so N
    // hops lands on the folder whose own `scan_folder` call used depth N.
    let mut node = &scan;
    for hop in 1..=MAX_SCAN_DEPTH {
        assert_eq!(node.folders.len(), 1, "expected one child at hop {hop}");
        node = &node.folders[0];
    }

    // `node` is now the folder scanned at depth == MAX_SCAN_DEPTH: it should
    // report its own contents but not have descended into its child.
    assert!(node.truncated);
    assert!(node.folders.is_empty());

    std::fs::remove_dir_all(&root).unwrap();
}

/// Exhausting the shared entry budget mid-scan also sets `truncated`, even
/// above the depth cap.
#[test]
fn test_scan_folder_budget_exhaustion_sets_truncated() {
    let root = make_temp_dir("budget");
    for i in 0..5 {
        std::fs::write(root.join(format!("g{i}.mangler.json")), "{}").unwrap();
    }

    // Budget smaller than the number of entries in the folder.
    let mut budget = 2usize;
    let scan = scan_folder(&root, 0, &mut budget).unwrap();

    assert!(scan.truncated);
    assert!(scan.graphs.len() <= 2);

    std::fs::remove_dir_all(&root).unwrap();
}

// === is_graph_file / graph_display_name ===

#[test]
fn test_is_graph_file_cases() {
    assert!(is_graph_file("foo.mangler.json"));
    assert!(is_graph_file("My Graph.mangler.json"));
    assert!(!is_graph_file("foo.json")); // wrong suffix
    assert!(!is_graph_file("foo.mangler.jsonx")); // wrong suffix
    assert!(!is_graph_file(".mangler.json")); // suffix with no stem
    assert!(!is_graph_file("FOO.MANGLE.JSON")); // case-sensitive match
    assert!(!is_graph_file("foo")); // no extension at all
}

// === is_image_file ===

#[test]
fn test_is_image_file_cases() {
    // Common image extensions the loader supports. The list is derived from
    // `ValueType::file_extensions(Image)`, which uses each format's *primary*
    // extension (e.g. JPEG → "jpg", not "jpeg").
    assert!(is_image_file("photo.png"));
    assert!(is_image_file("photo.jpg"));
    assert!(is_image_file("photo.bmp"));
    // Case-insensitive: an uppercase extension still classifies.
    assert!(is_image_file("PHOTO.PNG"));
    assert!(is_image_file("Photo.Jpg"));
    // Not images.
    assert!(!is_image_file("notes.txt"));
    assert!(!is_image_file("graph.mangler.json"));
    assert!(!is_image_file("noext"));
}

// === scan_folder: image classification ===

/// Image files are classified into `images` (with their extension kept in the
/// name), graphs stay in `graphs`, and unrelated files are ignored — all
/// case-insensitively and sorted.
#[test]
fn test_scan_folder_classifies_images() {
    let root = make_temp_dir("images");

    // Two images in non-alphabetical, mixed-case order + one uppercase ext.
    std::fs::write(root.join("zebra.png"), "x").unwrap();
    std::fs::write(root.join("Apple.PNG"), "x").unwrap();
    // A graph — must stay a graph, not an image.
    std::fs::write(root.join("g.mangler.json"), "{}").unwrap();
    // Unrelated file — ignored by both classifiers.
    std::fs::write(root.join("readme.txt"), "hi").unwrap();

    let mut budget = MAX_ENTRIES_PER_LIBRARY;
    let scan = scan_folder(&root, 0, &mut budget).unwrap();

    // Graph classification is unchanged and doesn't leak into images.
    assert_eq!(scan.graphs.len(), 1);
    assert_eq!(scan.graphs[0].name, "g");

    // Both images listed, sorted case-insensitively, extension preserved.
    assert_eq!(scan.images.len(), 2);
    assert_eq!(scan.images[0].name, "Apple.PNG"); // "apple" < "zebra"
    assert_eq!(scan.images[1].name, "zebra.png");

    // The .txt is in neither bucket.
    assert!(scan.images.iter().all(|i| i.name != "readme.txt"));

    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn test_graph_display_name() {
    assert_eq!(graph_display_name("foo.mangler.json"), "foo");
    assert_eq!(graph_display_name("My Graph.mangler.json"), "My Graph");
    // Not the canonical extension, but still a `.json` file (e.g. legacy or
    // foreign): mangler_core::naming::graph_display_name strips the plain
    // `.json` suffix too, rather than showing the extension as part of the
    // name.
    assert_eq!(graph_display_name("foo.json"), "foo");
    // No matching suffix at all: returned unchanged.
    assert_eq!(graph_display_name("foo.txt"), "foo.txt");
}
