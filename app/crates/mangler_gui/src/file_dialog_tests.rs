use super::*;
use egui_file_dialog::{FileDialogConfig, OpeningMode};
use std::path::PathBuf;

/// The reset value `AppFileDialog` passes for a request that seeds no
/// directory of its own (in the app it is the crate's own construction-time
/// default).
const NEUTRAL: &str = "/neutral";

/// `configure` with the neutral fallback filled in, so each test reads as one
/// call instead of repeating the third argument.
fn conf(config: &mut FileDialogConfig, request: &FileDialogRequest) {
    configure(config, request, Path::new(NEUTRAL));
}

fn save_graph(stem: &str, dir: Option<&str>) -> FileDialogRequest {
    FileDialogRequest::SaveGraph {
        default_dir: dir.map(PathBuf::from),
        default_stem: stem.to_owned(),
    }
}

fn input_path(
    extensions: &[&str],
    set_directory: Option<&str>,
    set_file_name: Option<&str>,
    fallback_dir: Option<&str>,
) -> FileDialogRequest {
    FileDialogRequest::InputPath {
        node_id: "n1".to_owned(),
        input_index: 3,
        extension_filter: extensions.iter().map(|e| (*e).to_owned()).collect(),
        set_directory: set_directory.map(PathBuf::from),
        set_file_name: set_file_name.map(str::to_owned),
        set_title: Some("image".to_owned()),
        file_dialog_type: FileDialogType::PickFile,
        fallback_dir: fallback_dir.map(PathBuf::from),
    }
}

/// The single config is reused across every call site, so anything one
/// request sets must be cleared by the next. Without this, picking a graph and
/// then opening a node's path input would offer the graph filter and pre-fill
/// the graph's file name.
#[test]
fn configure_does_not_leak_state_between_requests() {
    let mut config = FileDialogConfig::default();

    conf(&mut config, &save_graph("my graph", Some("/tmp")));
    assert_eq!(config.file_filters.len(), 1);
    assert!(!config.default_file_name.is_empty());
    assert!(config.default_file_filter.is_some());

    // A request that sets none of those must leave none of them behind.
    conf(&mut config, &FileDialogRequest::AddLibrary);
    assert!(config.file_filters.is_empty(), "filter leaked");
    assert!(config.default_file_filter.is_none(), "default filter leaked");
    assert!(config.default_file_name.is_empty(), "file name leaked");
    assert!(config.save_extensions.is_empty(), "save extension leaked");
    assert!(config.default_save_extension.is_none());
    assert_eq!(config.title.as_deref(), Some("add library folder"));
    assert_eq!(
        config.initial_directory,
        PathBuf::from(NEUTRAL),
        "directory leaked"
    );
}

/// A save extension would be applied with `PathBuf::set_extension`, which
/// turns `g.mangler.json` into `g.mangler.mangler.json`. The extension is
/// forced by `force_graph_extension` on the way out instead.
#[test]
fn configure_save_graph_prefills_the_name_and_sets_no_save_extension() {
    let mut config = FileDialogConfig::default();
    conf(&mut config, &save_graph("my graph", None));

    assert_eq!(config.default_file_name, naming::graph_file_name("my graph"));
    assert!(config.save_extensions.is_empty());
    assert!(config.default_save_extension.is_none());
}

#[test]
fn configure_save_graph_seeds_the_directory_when_given_one() {
    let mut config = FileDialogConfig::default();

    conf(&mut config, &save_graph("g", None));
    assert_eq!(
        config.initial_directory,
        PathBuf::from(NEUTRAL),
        "no dir falls back to the neutral default"
    );

    conf(&mut config, &save_graph("g", Some("/tmp/graphs")));
    assert_eq!(config.initial_directory, PathBuf::from("/tmp/graphs"));
}

/// The crate's default opening mode is `LastPickedDir`, under which
/// `initial_directory` is read only while nothing has been picked yet. One
/// `FileDialog` lives for the whole app, so leaving that mode in place made
/// every seeded directory dead after the user's first pick — a first save no
/// longer started in the default library, and a `Value::Path` input no longer
/// started in the graph's folder. A seeded request must pin the mode.
#[test]
fn a_seeded_directory_is_actually_opened() {
    let mut config = FileDialogConfig::default();

    conf(&mut config, &save_graph("g", Some("/tmp/graphs")));
    assert_eq!(config.opening_mode, OpeningMode::AlwaysInitialDir);

    conf(
        &mut config,
        &input_path(&["png"], Some("/explicit"), None, None),
    );
    assert_eq!(config.opening_mode, OpeningMode::AlwaysInitialDir);

    conf(&mut config, &input_path(&["png"], None, None, Some("/graph")));
    assert_eq!(config.opening_mode, OpeningMode::AlwaysInitialDir);
}

/// ...and a request that seeds nothing must hand the mode back, so opening a
/// graph or linking a library still resumes where the user left off rather
/// than being pinned to whatever the last seeded caller chose.
#[test]
fn an_unseeded_request_resumes_from_the_last_picked_directory() {
    let mut config = FileDialogConfig::default();

    conf(&mut config, &save_graph("g", Some("/tmp/graphs")));
    conf(&mut config, &FileDialogRequest::OpenGraph);

    assert_eq!(config.opening_mode, OpeningMode::LastPickedDir);
    assert_eq!(
        config.initial_directory,
        PathBuf::from(NEUTRAL),
        "the previous caller's directory must not leak into an unseeded request"
    );
}

/// An input's own `set_directory` is explicit intent; the graph folder is only
/// a fallback for inputs that name none.
#[test]
fn configure_input_path_prefers_set_directory_over_the_graph_folder() {
    let mut config = FileDialogConfig::default();

    conf(&mut config,
        &input_path(&["png"], Some("/explicit"), None, Some("/graph")),
    );
    assert_eq!(config.initial_directory, PathBuf::from("/explicit"));

    conf(&mut config, &input_path(&["png"], None, None, Some("/graph")));
    assert_eq!(config.initial_directory, PathBuf::from("/graph"));
}

#[test]
fn configure_input_path_with_no_extensions_adds_no_filter() {
    let mut config = FileDialogConfig::default();
    conf(&mut config, &input_path(&[], None, None, None));

    assert!(config.file_filters.is_empty());
    assert!(config.default_file_filter.is_none());
    assert_eq!(config.title.as_deref(), Some("image"));
}

#[test]
fn extension_filters_match_case_insensitively_and_reject_others() {
    let exts = ["png".to_owned(), "JPG".to_owned()];

    assert!(extension_matches(Path::new("/a/b.png"), &exts));
    assert!(extension_matches(Path::new("/a/b.PNG"), &exts));
    assert!(extension_matches(Path::new("/a/b.jpg"), &exts));
    assert!(!extension_matches(Path::new("/a/b.txt"), &exts));
    assert!(!extension_matches(Path::new("/a/b"), &exts));

    // The filter is named after what it shows, and that name is its identity
    // in the config (filters are replaced by name, not appended).
    let filter = filter_from_extensions("image", &exts);
    assert_eq!(filter.name, "image");
}

/// Filters match `Path::extension()` — the final dot-component — so the plain
/// `"json"` token covers our double-barrelled `.mangler.json` too. A
/// `"mangler.json"` token would match nothing at all.
#[test]
fn the_graph_filter_accepts_both_plain_and_mangler_json() {
    let json = ["json".to_owned()];

    assert!(extension_matches(Path::new("/a/graph.mangler.json"), &json));
    assert!(extension_matches(Path::new("/a/graph.json"), &json));
    assert!(!extension_matches(Path::new("/a/graph.png"), &json));

    assert_eq!(graph_filter().name, "NodeMangler graph");
}

#[test]
fn force_graph_extension_leaves_a_canonical_name_alone() {
    let path = PathBuf::from("/a/b/my graph.mangler.json");
    assert_eq!(force_graph_extension(path.clone()), path);
}

#[test]
fn force_graph_extension_upgrades_a_plain_json_name() {
    assert_eq!(
        force_graph_extension(PathBuf::from("/a/b/my graph.json")),
        PathBuf::from("/a/b/my graph.mangler.json")
    );
}

#[test]
fn force_graph_extension_appends_to_a_bare_name() {
    assert_eq!(
        force_graph_extension(PathBuf::from("/a/b/my graph")),
        PathBuf::from("/a/b/my graph.mangler.json")
    );
}

/// An unrelated extension is kept as part of the stem (the user typed it),
/// with the canonical extension appended after it.
#[test]
fn force_graph_extension_keeps_an_unrelated_extension_in_the_stem() {
    assert_eq!(
        force_graph_extension(PathBuf::from("/tmp/x.png")),
        PathBuf::from("/tmp/x.png.mangler.json")
    );
}
