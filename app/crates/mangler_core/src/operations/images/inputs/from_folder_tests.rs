use super::*;
use crate::input::Input;
use std::path::Path;

/// Creates (or clears) a fresh temp dir for a test, mirroring the pattern used
/// by the "to file" output tests (`outputs/file_tests.rs`).
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nodemangler_test_from_folder_{}", name));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes a placeholder file at `path` with no real image content. Used for
/// [`list_image_files`] tests, which only inspect extensions/file-ness and
/// never decode, so a real image isn't needed.
fn touch(path: &std::path::Path) {
    std::fs::write(path, b"not a real image, just needs to exist").unwrap();
}

/// Encodes and writes a tiny (1x1) real grayscale PNG at `path`, so
/// [`OpImageInputFromFolder::run`] (which does decode) can load it.
fn write_tiny_png(path: &std::path::Path, gray: u8) {
    image::GrayImage::from_pixel(1, 1, image::Luma([gray]))
        .save(path)
        .unwrap();
}

/// Runs the operation with `folder`/`index` inputs (unconnected, so
/// `run_context::current()` sees `None` — `folder` must therefore be
/// absolute for the run to find anything) and returns the raw result.
async fn run_from_folder(folder: PathBuf, index: i32) -> Result<OperationResponse, OperationError> {
    let mut inputs = vec![
        Input::new("folder".to_string(), Value::Path(folder), None, None),
        Input::new("index".to_string(), Value::Integer(index), None, None),
    ];
    OpImageInputFromFolder::run(&mut inputs).await
}

// --- settings / shape --------------------------------------------------

#[tokio::test]
async fn test_from_folder_exact_settings() {
    let s = OpImageInputFromFolder::settings();
    assert_eq!(s.name, "from folder");
    assert_eq!(OpImageInputFromFolder::create_inputs().len(), 2);
    assert_eq!(OpImageInputFromFolder::create_outputs().len(), 4);
}

// --- list_image_files ----------------------------------------------------

#[test]
fn test_list_image_files_filters_and_sorts_case_insensitively() {
    let dir = temp_dir("list_basic");
    touch(&dir.join("b.PNG"));
    touch(&dir.join("a.png"));
    touch(&dir.join("C.jpg"));
    touch(&dir.join("notes.txt")); // non-image extension, must be excluded

    // A subdirectory (even one containing an image) must be excluded: the
    // listing is non-recursive.
    let subdir = dir.join("subdir");
    std::fs::create_dir_all(&subdir).unwrap();
    touch(&subdir.join("d.png"));

    let files = list_image_files(&dir).unwrap();
    let names: Vec<String> = files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert_eq!(names, vec!["a.png", "b.PNG", "C.jpg"]);
}

#[test]
fn test_list_image_files_missing_dir_errors() {
    let dir = std::env::temp_dir().join("nodemangler_test_from_folder_missing_dir_does_not_exist");
    std::fs::remove_dir_all(&dir).ok(); // guarantee it really doesn't exist
    assert!(list_image_files(&dir).is_err());
}

// --- resolve_folder --------------------------------------------------------

#[test]
fn test_resolve_folder_empty_is_none() {
    assert_eq!(resolve_folder(Path::new(""), Some(Path::new("/graph/dir"))), None);
}

#[test]
fn test_resolve_folder_absolute_is_used_as_is() {
    // An absolute path (platform-appropriate) is returned unchanged,
    // regardless of the graph directory.
    let abs = if cfg!(windows) { PathBuf::from(r"C:\images") } else { PathBuf::from("/images") };
    assert_eq!(resolve_folder(&abs, Some(Path::new("/graph/dir"))), Some(abs));
}

#[test]
fn test_resolve_folder_relative_joins_graph_dir() {
    let graph_dir = Path::new("/graph/dir");
    assert_eq!(
        resolve_folder(Path::new("images"), Some(graph_dir)),
        Some(graph_dir.join("images"))
    );
}

#[test]
fn test_resolve_folder_relative_without_graph_dir_is_none() {
    // No graph directory to resolve against (graph never saved, or a direct
    // unit-test call outside the engine) — a relative path is unresolvable.
    assert_eq!(resolve_folder(Path::new("images"), None), None);
}

// --- run() end-to-end --------------------------------------------------

#[tokio::test]
async fn test_from_folder_run_selects_by_index_and_reports_count() {
    let dir = temp_dir("run_basic");
    // Case-insensitive sort order: apple, banana, cherry.
    write_tiny_png(&dir.join("banana.png"), 100);
    write_tiny_png(&dir.join("apple.png"), 50);
    write_tiny_png(&dir.join("cherry.png"), 200);

    let result = run_from_folder(dir, 1).await.unwrap();
    let Value::Image { .. } = &result.responses[0].value else { panic!("expected image output") };
    let Value::Text(name) = &result.responses[1].value else { panic!("expected file name output") };
    assert_eq!(name, "banana");
    let Value::Integer(idx) = result.responses[2].value else { panic!("expected index output") };
    assert_eq!(idx, 1);
    let Value::Integer(count) = result.responses[3].value else { panic!("expected count output") };
    assert_eq!(count, 3);
}

#[tokio::test]
async fn test_from_folder_run_clamps_negative_index_to_first() {
    let dir = temp_dir("run_clamp_low");
    write_tiny_png(&dir.join("apple.png"), 10);
    write_tiny_png(&dir.join("banana.png"), 20);

    let result = run_from_folder(dir, -5).await.unwrap();
    let Value::Text(name) = &result.responses[1].value else { panic!("expected file name output") };
    assert_eq!(name, "apple", "negative index should clamp to the first file");
    let Value::Integer(idx) = result.responses[2].value else { panic!("expected index output") };
    assert_eq!(idx, 0);
}

#[tokio::test]
async fn test_from_folder_run_clamps_large_index_to_last() {
    let dir = temp_dir("run_clamp_high");
    write_tiny_png(&dir.join("apple.png"), 10);
    write_tiny_png(&dir.join("banana.png"), 20);
    write_tiny_png(&dir.join("cherry.png"), 30);

    let result = run_from_folder(dir, 999).await.unwrap();
    let Value::Text(name) = &result.responses[1].value else { panic!("expected file name output") };
    assert_eq!(name, "cherry", "an index past the end should clamp to the last file");
    let Value::Integer(idx) = result.responses[2].value else { panic!("expected index output") };
    assert_eq!(idx, 2);
    let Value::Integer(count) = result.responses[3].value else { panic!("expected count output") };
    assert_eq!(count, 3);
}

#[tokio::test]
async fn test_from_folder_run_empty_folder_errors() {
    let dir = temp_dir("run_empty"); // exists but has no files in it
    let result = run_from_folder(dir, 0).await;
    assert!(result.is_err(), "a folder with no image files should error");
}

#[tokio::test]
async fn test_from_folder_run_unset_folder_errors() {
    let result = run_from_folder(PathBuf::new(), 0).await;
    assert!(result.is_err(), "an unset (empty) folder should error");
}
