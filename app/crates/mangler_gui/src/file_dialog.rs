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
use crate::icons;
use crate::themes::theme::{Theme, ThemeValues};
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

/// Swaps the dialog's stock emoji icons for the Phosphor glyphs the rest of
/// the UI uses.
///
/// The defaults ("🗀", "⏴", "🔍", …) resolve through egui's emoji fallback
/// rather than our icon font, so they land at a different weight and baseline
/// from every other icon in the app and make the dialog read as a foreign
/// window. These are the same constants the panel chrome uses.
fn dress_icons(config: &mut egui_file_dialog::FileDialogConfig) {
    config.err_icon = icons::WARNING.to_owned();
    config.warn_icon = icons::WARNING.to_owned();
    config.default_file_icon = icons::FILE_TEXT.to_owned();
    config.default_folder_icon = icons::FOLDER.to_owned();
    config.pinned_icon = icons::PUSH_PIN.to_owned();
    config.device_icon = icons::HARD_DRIVE.to_owned();
    config.removable_device_icon = icons::FLOPPY_DISK.to_owned();
    config.parent_directory_icon = icons::CARET_UP.to_owned();
    config.back_icon = icons::ARROW_LEFT.to_owned();
    config.forward_icon = icons::ARROW_RIGHT.to_owned();
    config.new_folder_icon = icons::FOLDER_PLUS.to_owned();
    config.menu_icon = icons::DOTS_THREE_VERTICAL.to_owned();
    config.search_icon = icons::MAGNIFYING_GLASS.to_owned();
    config.path_edit_icon = icons::PENCIL_SIMPLE.to_owned();
}

/// Rewrites the dialog's stock labels to the app's own conventions.
///
/// Two things to fix. The defaults are Title Case ("Places", "Selected file:")
/// while every label in NodeMangler is lowercase, and several of them embed
/// emoji ("🗀  Open", "🚫 Cancel", "🏠  Home") that resolve through egui's
/// emoji fallback rather than our icon font — the same mismatch `dress_icons`
/// fixes for the chrome. The sidebar entries keep an icon, but a Phosphor one;
/// the buttons drop theirs, because the app's buttons are text-only.
fn dress_labels(config: &mut egui_file_dialog::FileDialogConfig) {
    let labels = &mut config.labels;

    labels.cancel = "cancel".to_owned();
    labels.overwrite = "overwrite".to_owned();

    labels.reload = "reload".to_owned();
    labels.working_directory = "go to working directory".to_owned();
    labels.select_all = "select all".to_owned();
    labels.show_hidden = "show hidden".to_owned();
    labels.show_system_files = "show system files".to_owned();

    labels.heading_pinned = "pinned".to_owned();
    labels.heading_places = "places".to_owned();
    labels.heading_devices = "devices".to_owned();
    labels.heading_removable_devices = "removable devices".to_owned();

    labels.home_dir = format!("{}  home", icons::HOUSE);
    labels.desktop_dir = format!("{}  desktop", icons::DESKTOP);
    labels.documents_dir = format!("{}  documents", icons::FILE_TEXT);
    labels.downloads_dir = format!("{}  downloads", icons::DOWNLOAD_SIMPLE);
    labels.audio_dir = format!("{}  audio", icons::MUSIC_NOTE);
    labels.pictures_dir = format!("{}  pictures", icons::IMAGE);
    labels.videos_dir = format!("{}  videos", icons::FILM_STRIP);

    labels.pin_folder = "pin".to_owned();
    labels.unpin_folder = "unpin".to_owned();
    labels.rename_pinned_folder = "rename".to_owned();

    labels.selected_directory = "selected folder:".to_owned();
    labels.selected_file = "selected file:".to_owned();
    labels.selected_items = "selected items:".to_owned();
    labels.file_name = "file name:".to_owned();
    labels.file_filter_all_files = "all files".to_owned();
    labels.save_extension_any = "any".to_owned();

    labels.open_button = "open".to_owned();
    labels.save_button = "save".to_owned();
    labels.cancel_button = "cancel".to_owned();

    labels.err_empty_folder_name = "the folder name cannot be empty".to_owned();
    labels.err_empty_file_name = "the file name cannot be empty".to_owned();
    labels.err_directory_exists = "a folder with that name already exists".to_owned();
    labels.err_file_exists = "a file with that name already exists".to_owned();
}

/// Builds the style the dialog is drawn with: the app's own, plus the
/// structural lines it needs and the rest of the app does not.
///
/// NodeMangler's panels separate themselves by hand — `section_rule` paints an
/// explicit hairline — so the theme zeroes egui's built-in dividers
/// (`window_stroke: Stroke::NONE`, a zero-width
/// `widgets.noninteractive.bg_stroke`). The file dialog has no such hand-drawn
/// chrome: it is four stacked `egui::Panel`s that rely entirely on those two
/// values to show their edges, so under our theme it renders as one flat slab
/// with the sidebar, file list, toolbar and button row all bleeding together.
///
/// Restoring them globally would draw lines all over the app, so the override
/// is scoped to the dialog (see [`AppFileDialog::update`]). Colors still come
/// from the theme: the divider is the same token the settings panel rules use.
fn dialog_style(base: &egui::Style, colors: &ThemeValues) -> egui::Style {
    let mut style = base.clone();
    let divider = egui::Stroke::new(1.0, colors.settings_section_rule);

    // Panel edges: the line between the sidebar and the file list, and above
    // the button row.
    style.visuals.widgets.noninteractive.bg_stroke = divider;
    // The dialog floats over the graph editor, so it needs an outline of its
    // own to read as a separate surface.
    style.visuals.window_stroke = divider;
    style
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
        let mut dialog = FileDialog::new()
            .id("mangler_file_dialog")
            .as_modal(true)
            .default_size(egui::vec2(880.0, 560.0))
            .min_size(egui::vec2(640.0, 400.0));

        dress_icons(dialog.config_mut());
        dress_labels(dialog.config_mut());

        Self { dialog }
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
    pub fn update(&mut self, ctx: &egui::Context, theme: &Theme) -> Option<(FileDialogIntent, PathBuf)> {
        // Swap in the dialog's style for the duration of its draw, then put
        // the app's back. The dialog renders immediately inside this call, and
        // it is the last thing drawn each frame, so nothing else sees this.
        let app_style = ctx.global_style();
        ctx.set_global_style(dialog_style(&app_style, &theme.get()));
        self.dialog.update(ctx);
        ctx.set_global_style(app_style);

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
