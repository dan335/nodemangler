//! Libraries: named links to on-disk folders of saved graphs, browsable from
//! an in-app panel. A library maps to a folder on local disk or a network
//! share (multiple users can link the same folder); subfolders map 1:1 to
//! disk folders and contain `.mangle.json` graphs. Only the links are
//! persisted (in `AppConfig::libraries`) — the folder trees themselves are
//! rescanned from disk by a background poll thread, so external changes show
//! up in the panel automatically.

/// Persisted library data model: `LibraryId`, `LibrarySource`, `LibraryConfig`.
pub mod library;
/// Background folder-tree scanner that walks a library's root folder for
/// `.mangle.json` graphs on a poll interval.
pub mod library_scanner;
/// The Libraries panel UI (tree of libraries → folders → graphs).
pub mod libraries_panel;
/// App-global panel state: linked libraries, scanner handle, open dialog,
/// and the action queue drained by `App`.
pub mod libraries_state;
