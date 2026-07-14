//! App-global state for the Libraries panel: the list of linked libraries,
//! the background scanner handle, the currently open dialog, and a queue of
//! actions the panel can't perform itself (opening a graph needs a new
//! program tab, which only `App` can create).
//!
//! There is exactly one `LibrariesState` per `App`; every Libraries panel
//! (main window or secondary windows) renders the same shared state.

use std::path::{Path, PathBuf};
use std::time::Instant;

use eframe::egui;

use crate::config::AppConfig;

use super::library::{LibraryConfig, LibraryId, LibrarySource};
use super::library_scanner::Scanner;

/// One linked library with its session-local id. The id is assigned at load
/// time and never persisted — configs on disk are identified by position.
pub struct LibraryEntry {
    /// Session-local identifier, used to key scan results and dialogs.
    pub id: LibraryId,
    /// The persisted part: display name + source folder.
    pub config: LibraryConfig,
}

/// The modal dialog currently open in the Libraries panel, if any. Each
/// variant carries the text being edited (for name dialogs) or the target
/// (for confirmations); the panel renders exactly one modal at a time.
pub enum LibraryDialog {
    /// Renaming a library's display name (config-only; disk untouched).
    RenameLibrary { id: LibraryId, name: String },
    /// Creating a new subfolder inside `parent`.
    NewFolder { parent: PathBuf, name: String },
    /// Creating a new graph inside `folder`.
    NewGraph { folder: PathBuf, name: String },
    /// Renaming a folder or graph file on disk.
    RenameEntry {
        path: PathBuf,
        is_folder: bool,
        name: String,
    },
    /// Confirming a move-to-recycle-bin of a folder or graph.
    ConfirmDelete { path: PathBuf, is_folder: bool },
    /// Confirming removal of a library link (disk untouched).
    ConfirmRemoveLibrary { id: LibraryId },
}

/// Requests the Libraries panel raises but cannot satisfy itself. `App`
/// drains these each frame after rendering (same deferred-mutation idiom as
/// `PanelAction`) because they need access to the programs map.
#[derive(Debug, Clone, PartialEq)]
pub enum LibraryAction {
    /// Open the graph at `path` as a program tab (or focus an existing tab
    /// already pointing at it).
    OpenGraph { path: PathBuf },
    /// Create a brand-new graph: a blank program tab whose save path is set
    /// to `path` (full `…/name.mangler.json`), so the engine's auto-save
    /// writes the file.
    CreateGraph { path: PathBuf, name: String },
    /// A graph file was renamed on disk. Open tabs pointing at `from` must
    /// re-target `to`, otherwise their auto-save resurrects the old file.
    /// The tab's display name is derived from the file stem, so re-targeting
    /// the path is all that's needed — nothing to patch (see
    /// `App::handle_library_action`).
    PathRenamed { from: PathBuf, to: PathBuf },
    /// Add an "image from file" node wired to `path` into the currently
    /// focused program's graph (needs the programs map, which the panel can't
    /// touch — see `App::handle_library_action`).
    AddImageNode { path: PathBuf },
    /// The user started dragging the image at `path` out of the panel. Records
    /// it as the focused program's active image drag; the drop (and node
    /// creation at the drop position) is handled by `Program::show_menu_drag`,
    /// mirroring the node-list drag.
    BeginImageDrag { path: PathBuf },
    /// Show the image at `path` in the focused program's 2D preview panel
    /// (loaded off the graph). Needs the programs map, so it's handled by
    /// `App::handle_library_action`.
    PreviewImage { path: PathBuf },
}

/// Shared state behind every Libraries panel. Owned by the GUI `App`.
pub struct LibrariesState {
    /// The linked libraries, in display order.
    pub entries: Vec<LibraryEntry>,
    /// Next session-local id to hand out when a library is added.
    next_id: LibraryId,
    /// Handle to the background folder scanner; `scanner.results` holds the
    /// latest folder-tree snapshot per library.
    pub scanner: Scanner,
    /// The modal dialog currently open, if any.
    pub dialog: Option<LibraryDialog>,
    /// Actions queued for `App` to perform; drained via `take_pending`.
    pending: Vec<LibraryAction>,
    /// Most recent disk-operation error, shown as a fading strip at the
    /// bottom of the panel. The `Instant` is when it occurred.
    pub error: Option<(String, Instant)>,
    /// When `false`, `save_config` is a no-op. Only disabled by tests so
    /// they never write to the real user config file.
    persist: bool,
}

impl LibrariesState {
    /// Builds the state from the persisted library configs, assigns each a
    /// session id, spawns the background scanner, and points it at every
    /// local library root.
    pub fn new(ctx: egui::Context, configs: Vec<LibraryConfig>) -> Self {
        Self::with_persistence(ctx, configs, true)
    }

    /// Test-only constructor that never writes the user's config file.
    #[cfg(test)]
    pub fn new_without_persistence(ctx: egui::Context, configs: Vec<LibraryConfig>) -> Self {
        Self::with_persistence(ctx, configs, false)
    }

    fn with_persistence(ctx: egui::Context, configs: Vec<LibraryConfig>, persist: bool) -> Self {
        let mut next_id: LibraryId = 0;
        let entries: Vec<LibraryEntry> = configs
            .into_iter()
            .map(|config| {
                let id = next_id;
                next_id += 1;
                LibraryEntry { id, config }
            })
            .collect();

        let state = Self {
            entries,
            next_id,
            scanner: Scanner::spawn(ctx),
            dialog: None,
            pending: Vec::new(),
            error: None,
            persist,
        };
        // Kick off the first scan of everything loaded from config.
        state.sync_roots();
        state
    }

    /// Takes (and clears) the queued actions. Called once per frame by `App`
    /// after all panels have rendered.
    pub fn take_pending(&mut self) -> Vec<LibraryAction> {
        std::mem::take(&mut self.pending)
    }

    /// Queues an action for `App` to perform after rendering.
    pub fn push_action(&mut self, action: LibraryAction) {
        self.pending.push(action);
    }

    /// Links a new library rooted at `path`. No-op if a library already
    /// points at the same path (the panel would otherwise show confusing
    /// duplicates that scan the same folder twice). The display name
    /// defaults to the folder's own name.
    pub fn add_library(&mut self, path: PathBuf) {
        let already_linked = self.entries.iter().any(|entry| {
            entry.config.source.local_path() == Some(path.as_path())
        });
        if already_linked {
            return;
        }

        // Default the display name to the folder name; users can rename the
        // library afterwards without touching disk.
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());

        let id = self.next_id;
        self.next_id += 1;
        self.entries.push(LibraryEntry {
            id,
            config: LibraryConfig {
                name,
                source: LibrarySource::Local { path },
            },
        });

        self.save_config();
        self.sync_roots();
    }

    /// Unlinks a library. Only the config entry is removed — nothing on disk
    /// is touched, so other users of a shared library are unaffected.
    pub fn remove_library(&mut self, id: LibraryId) {
        self.entries.retain(|entry| entry.id != id);
        self.save_config();
        self.sync_roots();
    }

    /// Renames a library's display name (config-only; the folder on disk
    /// keeps its name).
    pub fn rename_library(&mut self, id: LibraryId, name: String) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) {
            entry.config.name = name;
            self.save_config();
        }
    }

    /// Pushes the current set of local library roots to the scanner so its
    /// next pass reflects the library list.
    fn sync_roots(&self) {
        let roots: Vec<(LibraryId, PathBuf)> = self
            .entries
            .iter()
            .filter_map(|entry| {
                entry
                    .config
                    .source
                    .local_path()
                    .map(|path| (entry.id, path.to_path_buf()))
            })
            .collect();
        self.scanner.set_roots(roots);
    }

    /// Persists the library list into the app config. Uses load-modify-save
    /// so concurrent settings (theme, layout) are never clobbered — the same
    /// pattern the theme picker uses.
    fn save_config(&self) {
        if !self.persist {
            return;
        }
        let mut config = AppConfig::load();
        config.libraries = self
            .entries
            .iter()
            .map(|entry| entry.config.clone())
            .collect();
        config.save();
    }

    /// Creates a subfolder named `name` (already sanitized by the dialog)
    /// inside `parent`. Errors surface in the panel's error strip.
    pub fn create_folder(&mut self, parent: &Path, name: &str) {
        let target = parent.join(name);
        if let Err(err) = std::fs::create_dir(&target) {
            self.set_error(format!("couldn't create folder '{}': {}", name, err));
        }
        // Rescan either way so the panel reflects reality (e.g. the folder
        // already existed).
        self.scanner.request_rescan();
    }

    /// Renames a folder or graph file on disk to `new_name` (already
    /// sanitized; for graphs the caller passes the stem — the
    /// `.mangler.json` extension is re-appended here). On a successful graph
    /// rename, queues `PathRenamed` so `App` re-targets any open tab still
    /// auto-saving to the old path.
    pub fn rename_path(&mut self, from: &Path, new_name: &str) {
        let is_graph = from
            .file_name()
            .map(|n| super::library_scanner::is_graph_file(&n.to_string_lossy()))
            .unwrap_or(false);

        // Rebuild the sibling path with the new name, keeping graph files'
        // full `.mangler.json` extension.
        let file_name = if is_graph {
            mangler_core::naming::graph_file_name(new_name)
        } else {
            new_name.to_string()
        };
        let to = from.with_file_name(file_name);

        if to == from {
            return;
        }

        match std::fs::rename(from, &to) {
            Ok(()) => {
                if is_graph {
                    // Open tabs pointing at the old path must follow the
                    // file, or their next auto-save recreates it. The graph's
                    // display name is derived from the file stem now, so once
                    // a tab re-targets the new path its name follows — there's
                    // nothing about the file's contents to patch.
                    self.push_action(LibraryAction::PathRenamed {
                        from: from.to_path_buf(),
                        to,
                    });
                }
            }
            Err(err) => {
                self.set_error(format!("couldn't rename '{}': {}", from.display(), err));
            }
        }
        self.scanner.request_rescan();
    }

    /// Moves a folder or graph to the OS recycle bin (never a permanent
    /// delete — recoverable if someone removes the wrong thing from a shared
    /// library). On failure (e.g. network shares without a recycle bin) the
    /// item is left in place and the error surfaces in the panel.
    pub fn delete_to_trash(&mut self, path: &Path) {
        if let Err(err) = trash::delete(path) {
            self.set_error(format!("couldn't delete '{}': {}", path.display(), err));
        }
        self.scanner.request_rescan();
    }

    /// Opens the OS file manager at `path`: Explorer on Windows, Finder on
    /// macOS, the default file manager on Linux. With `select`, the item is
    /// highlighted inside its parent folder instead of opened (Linux's
    /// `xdg-open` can't select, so it falls back to opening the parent).
    pub fn reveal_in_explorer(&mut self, path: &Path, select: bool) {
        let result = spawn_file_manager(path, select);
        if let Err(err) = result {
            self.set_error(format!("couldn't open file manager: {}", err));
        }
    }

    /// Records a disk-operation error for the panel's fading error strip.
    /// `pub(crate)` so `App::handle_library_action` can surface failures
    /// (e.g. a failed embedded-name patch after a rename) through the same
    /// strip instead of a separate error path.
    pub(crate) fn set_error(&mut self, message: String) {
        self.error = Some((message, Instant::now()));
    }

    /// Sanitizes a user-entered name into something safe to use as a file or
    /// folder name. Thin wrapper over `mangler_core::naming::sanitize_name`
    /// so library-created files match dialog-created ones (one sanitizer,
    /// not several drifting copies).
    pub fn sanitize(name: &str) -> String {
        mangler_core::naming::sanitize_name(name)
    }
}

/// Launches the platform file manager showing `path` (see
/// `LibrariesState::reveal_in_explorer`). Spawns detached — we never wait on
/// the file manager.
#[cfg(target_os = "windows")]
fn spawn_file_manager(path: &Path, select: bool) -> std::io::Result<std::process::Child> {
    use std::os::windows::process::CommandExt;
    let mut command = std::process::Command::new("explorer.exe");
    if select {
        // Explorer needs `/select,"C:\path"` as one literal argument; the
        // default quoting mangles the comma form, hence `raw_arg`.
        command.raw_arg(format!("/select,\"{}\"", path.display()));
    } else {
        command.arg(path);
    }
    command.spawn()
}

/// See the Windows variant above. `open -R` reveals (selects) in Finder.
#[cfg(target_os = "macos")]
fn spawn_file_manager(path: &Path, select: bool) -> std::io::Result<std::process::Child> {
    let mut command = std::process::Command::new("open");
    if select {
        command.arg("-R");
    }
    command.arg(path);
    command.spawn()
}

/// See the Windows variant above. `xdg-open` has no select mode, so
/// "reveal" opens the containing folder instead.
#[cfg(all(unix, not(target_os = "macos")))]
fn spawn_file_manager(path: &Path, select: bool) -> std::io::Result<std::process::Child> {
    let target = if select {
        path.parent().unwrap_or(path)
    } else {
        path
    };
    std::process::Command::new("xdg-open").arg(target).spawn()
}

#[cfg(test)]
#[path = "libraries_state_tests.rs"]
mod tests;
