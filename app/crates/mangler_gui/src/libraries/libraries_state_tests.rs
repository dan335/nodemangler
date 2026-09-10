//! Tests for `LibrariesState` entry management and action queueing. All
//! states are built with `new_without_persistence` so nothing here ever
//! writes to the real user config file. The spawned scanner thread is
//! harmless in tests: it scans an empty/missing root set and sleeps.

use std::path::PathBuf;

use eframe::egui;

use super::*;
use crate::libraries::library::{LibraryConfig, LibrarySource};
use crate::libraries::libraries_state::LibraryViewStyle;

/// Builds a test state from (name, path) pairs.
fn state_with(configs: &[(&str, &str)]) -> LibrariesState {
    let configs: Vec<LibraryConfig> = configs
        .iter()
        .map(|(name, path)| LibraryConfig {
            name: (*name).to_string(),
            source: LibrarySource::Local {
                path: PathBuf::from(path),
            },
        })
        .collect();
    LibrariesState::new_without_persistence(egui::Context::default(), configs)
}

#[test]
fn new_assigns_sequential_ids() {
    let state = state_with(&[("a", "C:/libs/a"), ("b", "C:/libs/b")]);
    assert_eq!(state.entries.len(), 2);
    assert_eq!(state.entries[0].id, 0);
    assert_eq!(state.entries[1].id, 1);
    assert_eq!(state.entries[0].config.name, "a");
}

#[test]
fn add_library_defaults_name_to_folder_name() {
    let mut state = state_with(&[]);
    state.add_library(PathBuf::from("C:/projects/my game company"));
    assert_eq!(state.entries.len(), 1);
    assert_eq!(state.entries[0].config.name, "my game company");
    assert_eq!(
        state.entries[0].config.source.local_path(),
        Some(PathBuf::from("C:/projects/my game company").as_path())
    );
}

#[test]
fn add_library_dedupes_by_path() {
    let mut state = state_with(&[("existing", "C:/libs/shared")]);
    // Same path again: ignored, even though the display name would differ.
    state.add_library(PathBuf::from("C:/libs/shared"));
    assert_eq!(state.entries.len(), 1);
    assert_eq!(state.entries[0].config.name, "existing");
}

#[test]
fn add_library_assigns_fresh_ids_after_load() {
    let mut state = state_with(&[("a", "C:/libs/a")]);
    state.add_library(PathBuf::from("C:/libs/b"));
    // The new entry's id must not collide with the loaded one.
    assert_eq!(state.entries[1].id, 1);
}

#[test]
fn remove_library_unlinks_only_that_entry() {
    let mut state = state_with(&[("a", "C:/libs/a"), ("b", "C:/libs/b")]);
    let id_a = state.entries[0].id;
    state.remove_library(id_a);
    assert_eq!(state.entries.len(), 1);
    assert_eq!(state.entries[0].config.name, "b");
}

#[test]
fn rename_library_changes_display_name_only() {
    let mut state = state_with(&[("old", "C:/libs/a")]);
    let id = state.entries[0].id;
    state.rename_library(id, "new".to_string());
    assert_eq!(state.entries[0].config.name, "new");
    // The source path is untouched by a rename.
    assert_eq!(
        state.entries[0].config.source.local_path(),
        Some(PathBuf::from("C:/libs/a").as_path())
    );
}

#[test]
fn rename_library_with_unknown_id_is_a_noop() {
    let mut state = state_with(&[("a", "C:/libs/a")]);
    state.rename_library(999, "ghost".to_string());
    assert_eq!(state.entries[0].config.name, "a");
}

#[test]
fn default_view_style_is_list() {
    let state = state_with(&[]);
    assert_eq!(state.view_style, LibraryViewStyle::List);
}

#[test]
fn set_view_style_updates_without_persist() {
    // new_without_persistence: style still mutates in memory; save_config is a no-op.
    let mut state = state_with(&[]);
    state.set_view_style(LibraryViewStyle::Thumbnails);
    assert_eq!(state.view_style, LibraryViewStyle::Thumbnails);
    state.set_view_style(LibraryViewStyle::List);
    assert_eq!(state.view_style, LibraryViewStyle::List);
}

#[test]
fn view_style_toggle_cycles() {
    assert_eq!(
        LibraryViewStyle::List.toggle(),
        LibraryViewStyle::Thumbnails
    );
    assert_eq!(
        LibraryViewStyle::Thumbnails.toggle(),
        LibraryViewStyle::List
    );
}

#[test]
fn take_pending_drains_the_queue() {
    let mut state = state_with(&[]);
    state.push_action(LibraryAction::OpenGraph {
        path: PathBuf::from("C:/libs/a/x.mangler.json"),
    });
    state.push_action(LibraryAction::PathRenamed {
        from: PathBuf::from("C:/libs/a/x.mangler.json"),
        to: PathBuf::from("C:/libs/a/y.mangler.json"),
    });

    let drained = state.take_pending();
    assert_eq!(drained.len(), 2);
    assert_eq!(
        drained[0],
        LibraryAction::OpenGraph {
            path: PathBuf::from("C:/libs/a/x.mangler.json")
        }
    );
    // A second take returns nothing: the queue was emptied.
    assert!(state.take_pending().is_empty());
}

/// `rename_path` renames a real file and must queue `PathRenamed` with the
/// old and new paths so `App` can retarget any open tab; the sanitized stem
/// lives in the new file name (`App` re-derives the display name from it).
#[test]
fn rename_path_queues_path_renamed_with_sanitized_new_name() {
    let dir = std::env::temp_dir().join(format!(
        "mangler_gui_libstate_rename_test_{}_{}",
        std::process::id(),
        get_id_for_test(),
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let from = dir.join("old_name.mangler.json");
    std::fs::write(&from, "{}").unwrap();

    let mut state = state_with(&[]);
    state.rename_path(&from, "new_name");

    let drained = state.take_pending();
    assert_eq!(drained.len(), 1, "a graph rename should queue exactly one action");
    match &drained[0] {
        LibraryAction::PathRenamed { from: queued_from, to } => {
            assert_eq!(queued_from, &from);
            assert_eq!(to, &dir.join("new_name.mangler.json"));
        }
        other => panic!("expected PathRenamed, got {:?}", other),
    }
    // The file itself actually moved on disk.
    assert!(dir.join("new_name.mangler.json").exists());
    assert!(!from.exists());

    let _ = std::fs::remove_dir_all(&dir);
}

/// `fs::rename` REPLACES an existing destination file, so without a guard the
/// Libraries panel's rename would destroy a sibling graph with no error at all
/// — the same collision `Graph::rename_file` already refuses on the in-app
/// rename path.
#[test]
fn rename_path_refuses_to_overwrite_an_existing_sibling() {
    let dir = temp_dir_for_test("collide");
    let from = dir.join("keep_me.mangler.json");
    let victim = dir.join("existing.mangler.json");
    std::fs::write(&from, "{\"a\":1}").unwrap();
    std::fs::write(&victim, "{\"victim\":true}").unwrap();

    let mut state = state_with(&[]);
    state.rename_path(&from, "existing");

    // Neither file moved, and the one that would have been clobbered is intact.
    assert!(from.exists(), "the source must stay put");
    assert_eq!(std::fs::read_to_string(&victim).unwrap(), "{\"victim\":true}");
    assert!(state.error.is_some(), "the refusal must be reported");
    // A refused rename is not a rename: no tab should be re-targeted.
    assert!(state.take_pending().is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}

/// The guard must not block renaming a file onto *itself* in a different case.
/// On a case-insensitive filesystem (macOS, Windows) the destination "exists"
/// because it IS the source, so a bare `exists()` check would wrongly refuse a
/// legitimate `graph` → `Graph` rename.
#[test]
fn rename_path_allows_a_case_only_rename() {
    let dir = temp_dir_for_test("case");
    let from = dir.join("lower.mangler.json");
    std::fs::write(&from, "{}").unwrap();

    let mut state = state_with(&[]);
    state.rename_path(&from, "LOWER");

    assert!(state.error.is_none(), "a case-only rename must be allowed");
    assert_eq!(
        state.take_pending().len(),
        1,
        "a successful graph rename queues PathRenamed"
    );
    assert!(dir.join("LOWER.mangler.json").exists());

    let _ = std::fs::remove_dir_all(&dir);
}

/// The engine writes a `CreateGraph` target unconditionally, so the Libraries
/// panel's name-field dialog — which confirms no overwrite — must refuse a name
/// that is already taken, or creating "wood" a second time replaces the real
/// graph with a blank one.
#[test]
fn create_graph_refuses_to_replace_an_existing_graph() {
    let dir = temp_dir_for_test("create");
    let path = dir.join("wood.mangler.json");
    std::fs::write(&path, "{\"nodes\":[]}").unwrap();

    let mut state = state_with(&[]);
    state.create_graph(path.clone(), "wood".to_string());

    assert!(
        state.take_pending().is_empty(),
        "no tab should be opened onto an existing graph's path"
    );
    assert!(state.error.is_some(), "the refusal must be reported");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{\"nodes\":[]}");

    let _ = std::fs::remove_dir_all(&dir);
}

/// ...and a free name still goes through, so the guard didn't break creation.
#[test]
fn create_graph_queues_the_action_for_a_free_name() {
    let dir = temp_dir_for_test("create_free");

    let mut state = state_with(&[]);
    let path = dir.join("fresh.mangler.json");
    state.create_graph(path.clone(), "fresh".to_string());

    let drained = state.take_pending();
    assert_eq!(drained.len(), 1);
    match &drained[0] {
        LibraryAction::CreateGraph { path: queued, name } => {
            assert_eq!(queued, &path);
            assert_eq!(name, "fresh");
        }
        other => panic!("expected CreateGraph, got {:?}", other),
    }
    assert!(state.error.is_none());

    let _ = std::fs::remove_dir_all(&dir);
}

/// A fresh, empty temp directory named for this test run.
fn temp_dir_for_test(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "mangler_gui_libstate_{}_{}_{}",
        tag,
        std::process::id(),
        get_id_for_test(),
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Small unique-id helper so parallel test runs never collide on the same
/// temp directory name (mirrors `library_scanner_tests.rs`'s `UNIQUE`
/// counter, kept local since this is the only test here that needs one).
fn get_id_for_test() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static UNIQUE: AtomicU64 = AtomicU64::new(0);
    UNIQUE.fetch_add(1, Ordering::SeqCst)
}

#[test]
fn sanitize_strips_illegal_filename_characters() {
    assert_eq!(LibrariesState::sanitize("my: graph?"), "my graph");
    assert_eq!(LibrariesState::sanitize("a/b\\c"), "abc");
    // Plain names pass through unchanged.
    assert_eq!(LibrariesState::sanitize("wood_floor"), "wood_floor");
}
