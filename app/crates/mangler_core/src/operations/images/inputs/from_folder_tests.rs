use super::*;
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
    run_from_folder_pinned(folder, index, PathBuf::new()).await
}

/// As [`run_from_folder`], but also sets the hidden `pinned path` input the
/// engine's watch driver uses. An empty `pinned` is the normal, index-driven case.
async fn run_from_folder_pinned(
    folder: PathBuf,
    index: i32,
    pinned: PathBuf,
) -> Result<OperationResponse, OperationError> {
    // Start from the real schema so raw-develop inputs (appended after the
    // pin) are present with their defaults; only override the selection ports.
    let mut inputs = OpImageInputFromFolder::create_inputs();
    inputs[FOLDER].value = Value::Path(folder);
    inputs[INDEX].value = Value::Integer(index);
    inputs[PINNED_PATH].value = Value::Path(pinned);
    OpImageInputFromFolder::run(&mut inputs).await
}

/// Like [`run_from_folder_pinned`], but also sets the raw develop controls.
async fn run_from_folder_with_raw_options(
    folder: PathBuf,
    index: i32,
    white_balance: &str,
    output: &str,
    demosaic: bool,
    exposure: f32,
    max_size: i32,
) -> Result<OperationResponse, OperationError> {
    let mut inputs = OpImageInputFromFolder::create_inputs();
    inputs[FOLDER].value = Value::Path(folder);
    inputs[INDEX].value = Value::Integer(index);
    inputs[WHITE_BALANCE].value = Value::Text(white_balance.to_string());
    inputs[OUTPUT_ENCODING].value = Value::Text(output.to_string());
    inputs[DEMOSAIC].value = Value::Bool(demosaic);
    inputs[EXPOSURE].value = Value::Decimal(exposure);
    inputs[MAX_SIZE].value = Value::Integer(max_size);
    OpImageInputFromFolder::run(&mut inputs).await
}

// --- settings / shape --------------------------------------------------

#[tokio::test]
async fn test_from_folder_exact_settings() {
    let s = OpImageInputFromFolder::settings();
    assert_eq!(s.name, "from folder");
    assert_eq!(OpImageInputFromFolder::create_inputs().len(), 8);
    assert_eq!(OpImageInputFromFolder::create_outputs().len(), 4);
}

/// The raw develop ports mirror `from raw` (same labels, same defaults, same
/// indices after the selection trio) so a shoot recipe is one place to learn.
#[test]
fn test_raw_develop_inputs_match_from_raw() {
    let inputs = OpImageInputFromFolder::create_inputs();
    assert_eq!(inputs[WHITE_BALANCE].name, "white balance");
    assert_eq!(inputs[OUTPUT_ENCODING].name, "output");
    assert_eq!(inputs[DEMOSAIC].name, "demosaic");
    assert_eq!(inputs[EXPOSURE].name, "exposure");
    assert_eq!(inputs[MAX_SIZE].name, "max size");

    let Value::Text(wb) = &inputs[WHITE_BALANCE].value else { panic!("expected text") };
    assert_eq!(wb, "as shot");
    let Value::Text(enc) = &inputs[OUTPUT_ENCODING].value else { panic!("expected text") };
    assert_eq!(enc, "srgb");
    let Value::Bool(demosaic) = inputs[DEMOSAIC].value else { panic!("expected bool") };
    assert!(demosaic);
    let Value::Decimal(exp) = inputs[EXPOSURE].value else { panic!("expected decimal") };
    assert_eq!(exp, 0.0);
    let Value::Integer(max) = inputs[MAX_SIZE].value else { panic!("expected integer") };
    assert_eq!(max, crate::operations::images::inputs::raw::DEFAULT_MAX_SIZE);

    // Selection contract used by the engine must not shift when develop ports
    // are added — they are appended after the pin.
    assert_eq!(FOLDER, 0);
    assert_eq!(INDEX, 1);
    assert_eq!(PINNED_PATH, 2);
    assert!(inputs[PINNED_PATH].hide_in_graph);
}

/// Non-raw files must still load when develop knobs are set — the options only
/// apply to camera RAW, not JPEG/PNG.
#[tokio::test]
async fn test_raw_options_ignored_for_non_raw() {
    let dir = temp_dir("raw_opts_non_raw");
    write_tiny_png(&dir.join("shot.png"), 128);

    let result = run_from_folder_with_raw_options(
        dir, 0, "camera neutral", "linear", true, 1.5, 512,
    )
    .await
    .unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else {
        panic!("expected image output")
    };
    assert_eq!(data.width(), 1);
    assert_eq!(data.height(), 1);
    let Value::Text(name) = &result.responses[1].value else { panic!("expected file name") };
    assert_eq!(name, "shot");
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

// --- pinned path (watch driver) -----------------------------------------

/// The pinned input must be hidden: it is set by the engine, not wired by hand,
/// and a visible port would clutter every from-folder node in the graph.
#[test]
fn test_pinned_path_input_is_hidden_and_empty_by_default() {
    let inputs = OpImageInputFromFolder::create_inputs();
    assert!(inputs[PINNED_PATH].hide_in_graph, "pinned path must not show a connection dot");
    let Value::Path(p) = &inputs[PINNED_PATH].value else { panic!("expected a path") };
    assert!(p.as_os_str().is_empty(), "default must be empty so index selection stays the norm");
}

/// A pinned file wins over the index, and reports its own position.
#[tokio::test]
async fn test_pinned_path_overrides_the_index() {
    let dir = temp_dir("pinned_overrides");
    write_tiny_png(&dir.join("a.png"), 10);
    write_tiny_png(&dir.join("b.png"), 20);
    write_tiny_png(&dir.join("c.png"), 30);

    // index says 0 (a.png), the pin says c.png — the pin must win.
    let response = run_from_folder_pinned(dir.clone(), 0, dir.join("c.png")).await.unwrap();
    let Value::Text(stem) = &response.responses[1].value else { panic!("expected text") };
    assert_eq!(stem, "c");
    let Value::Integer(used) = response.responses[2].value else { panic!("expected integer") };
    assert_eq!(used, 2, "the reported index must be the pinned file's position");
}

/// The whole point of pinning: a file arriving *before* the pinned one in sort
/// order must not shift the selection, which is exactly what an index would do.
#[tokio::test]
async fn test_pinned_path_survives_a_file_appearing_earlier_in_sort_order() {
    let dir = temp_dir("pinned_shift");
    write_tiny_png(&dir.join("m.png"), 10);
    let target = dir.join("z.png");
    write_tiny_png(&target, 20);

    let before = run_from_folder_pinned(dir.clone(), 0, target.clone()).await.unwrap();
    let Value::Integer(index_before) = before.responses[2].value else { panic!("expected integer") };
    assert_eq!(index_before, 1);

    // A new frame lands that sorts first; the pin must still resolve to z.png.
    write_tiny_png(&dir.join("a.png"), 30);
    let after = run_from_folder_pinned(dir.clone(), 0, target).await.unwrap();
    let Value::Text(stem) = &after.responses[1].value else { panic!("expected text") };
    assert_eq!(stem, "z", "the pinned file must not shift when the folder grows");
    let Value::Integer(index_after) = after.responses[2].value else { panic!("expected integer") };
    assert_eq!(index_after, 2, "its position moved, but it is still the same file");
}

/// An empty pin must be indistinguishable from the node as it behaved before
/// the input existed.
#[tokio::test]
async fn test_empty_pinned_path_is_plain_index_selection() {
    let dir = temp_dir("pinned_empty");
    write_tiny_png(&dir.join("a.png"), 10);
    write_tiny_png(&dir.join("b.png"), 20);

    for index in [0, 1] {
        let pinned = run_from_folder_pinned(dir.clone(), index, PathBuf::new()).await.unwrap();
        let plain = run_from_folder(dir.clone(), index).await.unwrap();
        let (Value::Text(a), Value::Text(b)) = (&pinned.responses[1].value, &plain.responses[1].value)
            else { panic!("expected text") };
        assert_eq!(a, b);
    }
}

/// A pinned file that has been deleted must fail loudly against the pinned
/// input rather than silently falling back to some other frame.
#[tokio::test]
async fn test_pinned_path_not_in_folder_errors() {
    let dir = temp_dir("pinned_missing");
    write_tiny_png(&dir.join("a.png"), 10);

    let err = run_from_folder_pinned(dir.clone(), 0, dir.join("gone.png")).await.unwrap_err();
    assert!(
        err.input_errors.iter().any(|(index, _)| *index == PINNED_PATH),
        "the error must be attributed to the pinned input, got {:?}",
        err.input_errors
    );
}
