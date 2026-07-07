//! Tests for `LibrariesState` entry management and action queueing. All
//! states are built with `new_without_persistence` so nothing here ever
//! writes to the real user config file. The spawned scanner thread is
//! harmless in tests: it scans an empty/missing root set and sleeps.

use std::path::PathBuf;

use eframe::egui;

use super::*;
use crate::libraries::library::{LibraryConfig, LibrarySource};

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
fn take_pending_drains_the_queue() {
    let mut state = state_with(&[]);
    state.push_action(LibraryAction::OpenGraph {
        path: PathBuf::from("C:/libs/a/x.mangle.json"),
    });
    state.push_action(LibraryAction::PathRenamed {
        from: PathBuf::from("C:/libs/a/x.mangle.json"),
        to: PathBuf::from("C:/libs/a/y.mangle.json"),
    });

    let drained = state.take_pending();
    assert_eq!(drained.len(), 2);
    assert_eq!(
        drained[0],
        LibraryAction::OpenGraph {
            path: PathBuf::from("C:/libs/a/x.mangle.json")
        }
    );
    // A second take returns nothing: the queue was emptied.
    assert!(state.take_pending().is_empty());
}

#[test]
fn sanitize_strips_illegal_filename_characters() {
    assert_eq!(LibrariesState::sanitize("my: graph?"), "my graph");
    assert_eq!(LibrariesState::sanitize("a/b\\c"), "abc");
    // Plain names pass through unchanged.
    assert_eq!(LibrariesState::sanitize("wood_floor"), "wood_floor");
}
