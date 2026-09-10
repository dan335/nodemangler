//! The app's one file dialog, and the intent plumbing that routes a picked
//! path back to whoever asked for it.
//!
//! We use [`egui_file_dialog`] rather than a native OS panel so the picker is
//! drawn in our own context: it picks up the active theme, and — unlike the
//! blocking `rfd` calls this replaced — it never stalls the frame loop while
//! the user browses.
//!
//! The cost is that a dialog is no longer a function that returns a path. It
//! is a per-frame state machine, so a click can only *start* one. Panels
//! therefore raise a [`FileDialogRequest`], `App` opens the dialog and stashes
//! a [`FileDialogIntent`] describing what the eventual path is for, and a
//! later frame's [`AppFileDialog::update`] hands the two back together. This
//! is the same deferred shape as the Libraries panel's `LibraryDialog` /
//! `apply_dialog` pair.

use std::path::{Path, PathBuf};

use eframe::egui;
use egui_file_dialog::{DialogState, FileDialog, FileFilter, Filter};
use mangler_core::input::FileDialogType;
use mangler_core::naming;

/// The label on the graph-file filter. Also its identity — `FileFilter`s are
/// keyed by name, so reusing the string replaces rather than appends.
const GRAPH_FILTER_NAME: &str = "NodeMangler graph";

/// What a panel asks for. Panels don't know which `Program` they belong to,
/// so `App` stamps the program id on when it converts this into an intent.
#[derive(Debug, Clone)]
pub enum FileDialogRequest {
    /// Choose where to write a graph — first save, or "save a copy as".
    SaveGraph {
        default_dir: Option<PathBuf>,
        default_stem: String,
    },
    /// Choose the child graph backing a subgraph node.
    SubgraphPath { node_id: String },
    /// Choose the value of a node's `InputSettings::Path` input.
    InputPath {
        node_id: String,
        input_index: usize,
        extension_filter: Vec<String>,
        set_directory: Option<PathBuf>,
        set_file_name: Option<String>,
        set_title: Option<String>,
        file_dialog_type: FileDialogType,
        /// Used when the input names no directory of its own — the folder the
        /// current graph lives in.
        fallback_dir: Option<PathBuf>,
    },
    /// Choose a folder to link into the Libraries panel.
    AddLibrary,
    /// Choose a graph file to open in a tab.
    OpenGraph,
}

/// What a picked path is *for*. Stored in the dialog as user data while it is
/// open, and read back when the pick lands — possibly many frames later, by
/// which point the panel that asked has long since returned.
#[derive(Debug, Clone)]
pub enum FileDialogIntent {
    SaveGraph {
        program_id: String,
    },
    SubgraphPath {
        program_id: String,
        node_id: String,
    },
    InputPath {
        program_id: String,
        node_id: String,
        input_index: usize,
    },
    AddLibrary,
    OpenGraph,
}

impl FileDialogRequest {
    /// Pairs this request with the program that raised it. `AddLibrary` and
    /// `OpenGraph` are app-level and ignore the id.
    fn into_intent(self, program_id: Option<String>) -> FileDialogIntent {
        // A missing id can only mean a program-scoped request outlived its
        // tab, which `App` already guards; an empty id simply matches no
        // program at dispatch time and is dropped there.
        let id = program_id.unwrap_or_default();
        match self {
            Self::SaveGraph { .. } => FileDialogIntent::SaveGraph { program_id: id },
            Self::SubgraphPath { node_id } => FileDialogIntent::SubgraphPath {
                program_id: id,
                node_id,
            },
            Self::InputPath {
                node_id,
                input_index,
                ..
            } => FileDialogIntent::InputPath {
                program_id: id,
                node_id,
                input_index,
            },
            Self::AddLibrary => FileDialogIntent::AddLibrary,
            Self::OpenGraph => FileDialogIntent::OpenGraph,
        }
    }
}

/// Whether `path` carries one of `extensions`, compared case-insensitively.
///
/// This is the whole substance of our file filters. It lives outside the
/// closure below because `Filter`'s predicate is not callable from outside the
/// dialog crate, so a test can only reach the rule in this form.
pub fn extension_matches(path: &Path, extensions: &[String]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|found| {
            let found = found.to_lowercase();
            extensions.iter().any(|want| want.to_lowercase() == found)
        })
}

/// A filter matching the given extensions, case-insensitively.
///
/// Extensions arrive as owned `String`s from `InputSettings::Path` at runtime,
/// so this closes over them rather than using the crate's `&'static str`
/// convenience builder.
pub fn filter_from_extensions(name: &str, extensions: &[String]) -> FileFilter {
    let extensions = extensions.to_vec();
    FileFilter {
        id: egui::Id::new(name),
        name: name.to_owned(),
        filter: Filter::new(move |path: &Path| extension_matches(path, &extensions)),
    }
}

/// The filter for `.mangler.json` graph files.
///
/// `"json"` alone is correct and covers `x.mangler.json`: filters match
/// `Path::extension()`, which is the final dot-component only, so a
/// `"mangler.json"` token would never match anything.
pub fn graph_filter() -> FileFilter {
    filter_from_extensions(GRAPH_FILTER_NAME, &["json".to_owned()])
}

/// Applies a request to the dialog's config, immediately before opening it.
///
/// Every field this touches is cleared unconditionally, even when the request
/// doesn't set it. The dialog reuses one config across every call site, and
/// `open` resets only the dialog's *state* — so a filter or a pre-filled file
/// name left behind by the previous caller would silently leak into the next
/// one.
pub fn configure(config: &mut egui_file_dialog::FileDialogConfig, request: &FileDialogRequest) {
    config.file_filters.clear();
    config.default_file_filter = None;
    config.save_extensions.clear();
    config.default_save_extension = None;
    config.default_file_name = String::new();
    config.title = None;

    match request {
        FileDialogRequest::SaveGraph {
            default_dir,
            default_stem,
        } => {
            config.file_filters.push(graph_filter());
            config.default_file_filter = Some(GRAPH_FILTER_NAME.to_owned());
            config.default_file_name = naming::graph_file_name(default_stem);
            config.title = Some("save graph".to_owned());
            // Deliberately no `save_extensions` entry: the crate would apply
            // it with `PathBuf::set_extension`, turning `g.mangler.json` into
            // `g.mangler.mangler.json`. `force_graph_extension` guarantees the
            // extension on the way out instead.
            if let Some(dir) = default_dir {
                config.initial_directory = dir.clone();
            }
        }
        FileDialogRequest::OpenGraph => {
            config.file_filters.push(graph_filter());
            config.default_file_filter = Some(GRAPH_FILTER_NAME.to_owned());
            config.title = Some("open graph".to_owned());
        }
        FileDialogRequest::SubgraphPath { .. } => {
            config.file_filters.push(graph_filter());
            config.default_file_filter = Some(GRAPH_FILTER_NAME.to_owned());
            config.title = Some("select subgraph".to_owned());
        }
        FileDialogRequest::AddLibrary => {
            config.title = Some("add library folder".to_owned());
        }
        FileDialogRequest::InputPath {
            extension_filter,
            set_directory,
            set_file_name,
            set_title,
            fallback_dir,
            ..
        } => {
            let title = set_title.clone().unwrap_or_else(|| "file".to_owned());
            if !extension_filter.is_empty() {
                config
                    .file_filters
                    .push(filter_from_extensions(&title, extension_filter));
                config.default_file_filter = Some(title.clone());
            }
            config.title = Some(title);
            if let Some(name) = set_file_name {
                config.default_file_name = name.clone();
            }
            // An explicit per-input directory wins; otherwise fall back to the
            // folder the graph itself lives in.
            if let Some(dir) = set_directory.as_ref().or(fallback_dir.as_ref()) {
                config.initial_directory = dir.clone();
            }
        }
    }
}

/// Forces the canonical `.mangler.json` extension onto whatever the save
/// dialog returned, regardless of what the user typed as the file name —
/// plain-.json saves must be impossible. A single trailing plain ".json" is
/// stripped first, so that choice becomes "<name>.mangler.json" rather than
/// "<name>.json.mangler.json".
pub fn force_graph_extension(path: PathBuf) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or_default();
    if file_name.ends_with(naming::GRAPH_EXTENSION) {
        path
    } else {
        let stem = file_name.strip_suffix(".json").unwrap_or(file_name);
        path.with_file_name(format!("{stem}{}", naming::GRAPH_EXTENSION))
    }
}

/// The app's single file dialog.
pub struct AppFileDialog {
    dialog: FileDialog,
}

impl Default for AppFileDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl AppFileDialog {
    pub fn new() -> Self {
        Self {
            dialog: FileDialog::new()
                .id("mangler_file_dialog")
                .as_modal(true)
                .default_size(egui::vec2(720.0, 480.0)),
        }
    }

    /// Whether a dialog is currently on screen.
    pub fn is_open(&self) -> bool {
        matches!(self.dialog.state(), DialogState::Open)
    }

    /// Opens the dialog for `request`, remembering what the pick is for.
    ///
    /// Does nothing if one is already open — two pickers would share this
    /// single dialog, so the second would silently retarget the first.
    pub fn open(&mut self, request: FileDialogRequest, program_id: Option<String>) {
        if self.is_open() {
            return;
        }

        configure(self.dialog.config_mut(), &request);

        match &request {
            FileDialogRequest::SaveGraph { .. } => self.dialog.save_file(),
            FileDialogRequest::OpenGraph | FileDialogRequest::SubgraphPath { .. } => {
                self.dialog.pick_file()
            }
            FileDialogRequest::AddLibrary => self.dialog.pick_directory(),
            FileDialogRequest::InputPath {
                file_dialog_type, ..
            } => match file_dialog_type {
                FileDialogType::PickFile => self.dialog.pick_file(),
                FileDialogType::PickFolder => self.dialog.pick_directory(),
                FileDialogType::SaveFile => self.dialog.save_file(),
            },
        }

        self.dialog.set_user_data(request.into_intent(program_id));
    }

    /// Draws the dialog and returns a pick once the user makes one.
    ///
    /// Call this once per frame with the main viewport's context, after
    /// everything else has drawn — the caller dispatches the result into
    /// `programs` / `libraries`, which are still borrowed during rendering.
    pub fn update(&mut self, ctx: &egui::Context) -> Option<(FileDialogIntent, PathBuf)> {
        self.dialog.update(ctx);

        // Read the intent before taking the path: `take_picked` needs `&mut`
        // and would end the borrow `user_data` holds.
        let intent = self.dialog.user_data::<FileDialogIntent>().cloned()?;
        let path = self.dialog.take_picked()?;
        Some((intent, path))
    }
}

#[cfg(test)]
#[path = "file_dialog_tests.rs"]
mod tests;
