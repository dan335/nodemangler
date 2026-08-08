//! The Libraries panel UI: a browser over every linked library's folders,
//! graphs, and images. Renders the shared `LibrariesState` owned by the GUI
//! `App`, so all Libraries panels (main window and secondary windows) show
//! the same content while keeping their own expansion state (each panel leaf
//! renders inside its own `push_id`).
//!
//! Rendering never mutates `LibrariesState` directly while the tree is being
//! drawn — context-menu choices are collected into local `PanelCommands` and
//! applied after the tree (and the scan-snapshot lock) are released. Actions
//! the panel can't perform itself (opening a graph needs a new program tab)
//! are queued on the state for `App` to drain after rendering.

use std::path::{Path, PathBuf};
use std::time::Duration;

use eframe::egui;
use egui_phosphor::regular as icons;

use crate::themes::theme::{Theme, ThemeValues};

use super::libraries_state::{
    LibrariesState, LibraryAction, LibraryDialog, LibraryEntry, LibraryViewStyle,
};
use super::library_scanner::{FolderScan, ImageEntry, LibraryScan, Scanner};
use super::library_thumbs::{
    prefetch_band, LibraryThumbCache, LIBRARY_THUMB_MIN_UI, LIBRARY_THUMB_PREFETCH_VIEWPORTS,
    LIBRARY_THUMB_UI,
};

/// Mutations chosen from context menus / buttons this frame. Collected while
/// the tree renders (which immutably borrows the state's entries and scan
/// snapshot) and applied to the state afterwards.
#[derive(Default)]
struct PanelCommands {
    /// A dialog to open (create/rename/delete flows all go through dialogs).
    open_dialog: Option<LibraryDialog>,
    /// Actions to queue for `App` (open graph, ...).
    actions: Vec<LibraryAction>,
    /// A folder the user picked to link as a new library.
    add_library: Option<PathBuf>,
    /// The user asked for an immediate rescan.
    refresh: bool,
    /// Show this path in the OS file manager; `true` = select the item in
    /// its parent folder, `false` = open the folder itself.
    reveal: Option<(PathBuf, bool)>,
    /// Toggle / set the list↔thumbnails view style.
    set_view_style: Option<LibraryViewStyle>,
}

/// What the user chose in the modal dialog this frame.
#[derive(PartialEq)]
enum DialogOutcome {
    /// Dialog stays open.
    Open,
    /// OK / Enter: perform the dialog's operation.
    Confirm,
    /// Cancel / Esc / click-away: close without doing anything.
    Cancel,
}

/// Renders the Libraries panel into `ui`. `current_graph` is the focused
/// program tab's save path, used to highlight that graph's row in the tree.
/// `previewed_image` is the image file (if any) currently shown in the focused
/// program's 2D preview, used to highlight its row.
pub fn show(
    ui: &mut egui::Ui,
    state: &mut LibrariesState,
    theme: &Theme,
    current_graph: Option<&Path>,
    previewed_image: Option<&Path>,
) {
    puffin::profile_scope!("libraries panel");
    let colors = theme.get();
    let mut commands = PanelCommands::default();

    // Inset the whole panel body — without this the heading and tree sit
    // flush against the panel's left edge.
    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 8,
            right: 8,
            top: 5,
            bottom: 8,
        })
        .show(ui, |ui| {
            show_header(ui, &colors, state.view_style, &mut commands);
            ui.add_space(4.0);

            // Poll thumbs and draw the tree with disjoint field borrows so
            // the scan lock and `&mut thumbs` coexist (no RefCell).
            {
                let LibrariesState {
                    entries,
                    scanner,
                    thumbs,
                    view_style,
                    ..
                } = state;
                thumbs.poll(ui.ctx());
                show_tree(
                    ui,
                    entries,
                    scanner,
                    thumbs,
                    *view_style,
                    &colors,
                    current_graph,
                    previewed_image,
                    &mut commands,
                );
            }

            show_error_strip(ui, state, &colors);
            show_background_menu(ui, &colors, &mut commands);
        });

    // --- apply collected commands (deferred: the tree render above held
    // immutable borrows of the state's entries and scan snapshot) -----------
    if let Some(path) = commands.add_library {
        state.add_library(path);
    }
    if commands.refresh {
        state.request_rescan();
    }
    if let Some((path, select)) = commands.reveal {
        state.reveal_in_explorer(&path, select);
    }
    if let Some(style) = commands.set_view_style {
        state.set_view_style(style);
    }
    for action in commands.actions {
        state.push_action(action);
    }
    if commands.open_dialog.is_some() {
        state.dialog = commands.open_dialog;
    }

    // --- modal dialog -------------------------------------------------------
    show_dialog(ui, state, &colors);
}

/// Title row with right-aligned view-style toggle and "+" menu.
fn show_header(
    ui: &mut egui::Ui,
    colors: &ThemeValues,
    view_style: LibraryViewStyle,
    commands: &mut PanelCommands,
) {
    ui.horizontal(|ui| {
        ui.heading("libraries");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Leave room for the panel's corner kind-switcher button (drawn
            // by panel_view at the panel's top-right) so the "+" doesn't sit
            // underneath it.
            ui.add_space(24.0);
            // RTL: first widget is rightmost. + first, then view (left of +).
            let plus = ui.button(icons::PLUS);
            egui::Popup::menu(&plus).show(|ui| {
                add_library_menu_items(ui, colors, commands);
            });

            let icon = match view_style {
                LibraryViewStyle::List => icons::LIST,
                LibraryViewStyle::Thumbnails => icons::SQUARES_FOUR,
            };
            let view = ui
                .button(icon)
                .on_hover_text(format!("view style: {}", view_style.label()));
            if view.clicked() {
                commands.set_view_style = Some(view_style.toggle());
            }
        });
    });
}

/// The two add-library entries shared by the "+" menu and the empty-space
/// right-click menu. Both open the OS folder picker; its built-in "New
/// Folder" button covers the create-new case, so they differ only in intent.
fn add_library_menu_items(ui: &mut egui::Ui, colors: &ThemeValues, commands: &mut PanelCommands) {
    strengthen_menu_hover(ui, colors);
    if ui.button("create new library…").clicked() {
        commands.add_library = pick_library_folder();
        ui.close();
    }
    if ui.button("add existing library…").clicked() {
        commands.add_library = pick_library_folder();
        ui.close();
    }
}

/// Menu items inherit `widgets.hovered.weak_bg_fill`, which several themes
/// keep very close to the popup background — the hovered row was barely
/// visible. Swap in the theme's stronger interactive hover fill for menus
/// spawned by this panel (still theme-derived, no hardcoded colors).
fn strengthen_menu_hover(ui: &mut egui::Ui, colors: &ThemeValues) {
    ui.style_mut().visuals.widgets.hovered.weak_bg_fill = colors.widgets_hovered_bg_fill;
}

/// An invisible click target filling the panel's leftover space, so
/// right-clicking the empty background offers the add-library menu. Placed
/// *after* the scroll area (which still auto-shrinks vertically to its
/// content) because vertical space inside a scroll area is unbounded. The
/// scroll area fills the panel width so its bar sits on the right edge.
fn show_background_menu(ui: &mut egui::Ui, colors: &ThemeValues, commands: &mut PanelCommands) {
    let leftover = ui.available_size_before_wrap().y;
    if leftover <= 0.0 {
        return;
    }
    let (_, background) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), leftover),
        egui::Sense::click(),
    );
    background.context_menu(|ui| {
        add_library_menu_items(ui, colors, commands);
    });
}

/// The scrollable tree of libraries → folders → graphs / images.
fn show_tree(
    ui: &mut egui::Ui,
    entries: &[LibraryEntry],
    scanner: &Scanner,
    thumbs: &mut LibraryThumbCache,
    view_style: LibraryViewStyle,
    colors: &ThemeValues,
    current_graph: Option<&Path>,
    previewed_image: Option<&Path>,
    commands: &mut PanelCommands,
) {
    // The scan-snapshot lock borrows through `scanner`, so everything in
    // here treats entries as read-only; `commands` / `thumbs` pick up
    // mutations.
    let results = scanner.results.lock().unwrap();

    // Fill the panel width so the scrollbar sits on the right edge of the
    // panel, not snug against the (narrower) row text. Keep vertical
    // auto-shrink so leftover space below the tree stays available for the
    // empty-background context menu.
    egui::ScrollArea::vertical()
        .auto_shrink([false, true])
        .show(ui, |ui| {
            if entries.is_empty() {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("no libraries yet — right-click or + to add one")
                        .color(colors.text_faint),
                );
            }

            for entry in entries {
                let read_only = entry.config.source.read_only();
                let scan = results.get(&entry.id);

                // Library row: collapsible header with a books icon. The id
                // salt is the session id, so renaming a library keeps its
                // expansion state.
                let title = format!("{}  {}", icons::BOOKS, entry.config.name);
                let header = egui::CollapsingHeader::new(egui::RichText::new(title))
                    .id_salt(("library", entry.id))
                    .default_open(true)
                    .show(ui, |ui| match scan {
                        Some(LibraryScan::Ok(folder)) => {
                            show_folder_contents(
                                ui,
                                folder,
                                read_only,
                                view_style,
                                thumbs,
                                colors,
                                current_graph,
                                previewed_image,
                                commands,
                            );
                        }
                        Some(LibraryScan::Unavailable(msg)) => {
                            // Offline share / missing folder: explain inline,
                            // never block or spin. Grid never runs here.
                            ui.label(
                                egui::RichText::new(format!("{}  {}", icons::WARNING, msg))
                                    .color(colors.grid_connection_dot_error),
                            );
                        }
                        // First scan hasn't published yet.
                        None => {
                            ui.label(egui::RichText::new("scanning…").color(colors.text_faint));
                        }
                    });

                // Keep the row visually marked while its context menu is open,
                // so it's clear which library the menu belongs to.
                if header.header_response.context_menu_opened() {
                    ui.ctx().highlight_widget(header.header_response.id);
                }
                header.header_response.context_menu(|ui| {
                    strengthen_menu_hover(ui, colors);
                    if let Some(root) = entry.config.source.local_path() {
                        if !read_only {
                            if ui.button("new graph…").clicked() {
                                commands.open_dialog = Some(LibraryDialog::NewGraph {
                                    folder: root.to_path_buf(),
                                    name: String::new(),
                                });
                                ui.close();
                            }
                            if ui.button("new folder…").clicked() {
                                commands.open_dialog = Some(LibraryDialog::NewFolder {
                                    parent: root.to_path_buf(),
                                    name: String::new(),
                                });
                                ui.close();
                            }
                            ui.separator();
                        }
                    }
                    if ui.button("rename library…").clicked() {
                        commands.open_dialog = Some(LibraryDialog::RenameLibrary {
                            id: entry.id,
                            name: entry.config.name.clone(),
                        });
                        ui.close();
                    }
                    if ui.button("refresh").clicked() {
                        commands.refresh = true;
                        ui.close();
                    }
                    // Opens the library's root folder itself (not select-in-
                    // parent): seeing its contents is the useful view here.
                    if let Some(root) = entry.config.source.local_path() {
                        if ui.button("reveal in file explorer").clicked() {
                            commands.reveal = Some((root.to_path_buf(), false));
                            ui.close();
                        }
                    }
                    ui.separator();
                    if ui.button("remove library…").clicked() {
                        commands.open_dialog =
                            Some(LibraryDialog::ConfirmRemoveLibrary { id: entry.id });
                        ui.close();
                    }
                });
            }
        });
}

/// Opens the OS folder picker for linking a library.
fn pick_library_folder() -> Option<PathBuf> {
    rfd::FileDialog::new().pick_folder()
}

/// Renders one folder's subfolders and graphs (recursively), collecting any
/// context-menu choices into `commands`.
fn show_folder_contents(
    ui: &mut egui::Ui,
    folder: &FolderScan,
    read_only: bool,
    view_style: LibraryViewStyle,
    thumbs: &mut LibraryThumbCache,
    colors: &ThemeValues,
    current_graph: Option<&Path>,
    previewed_image: Option<&Path>,
    commands: &mut PanelCommands,
) {
    for sub in &folder.folders {
        // Folder row: collapsible, salted by full path so expansion state
        // survives sibling changes.
        let title = format!("{}  {}", icons::FOLDER, sub.name);
        let header = egui::CollapsingHeader::new(egui::RichText::new(title))
            .id_salt(sub.path.as_path())
            .default_open(false)
            .show(ui, |ui| {
                show_folder_contents(
                    ui,
                    sub,
                    read_only,
                    view_style,
                    thumbs,
                    colors,
                    current_graph,
                    previewed_image,
                    commands,
                );
            });

        // Keep the row visually marked while its context menu is open, so
        // it's clear which folder the menu belongs to.
        if header.header_response.context_menu_opened() {
            ui.ctx().highlight_widget(header.header_response.id);
        }

        header.header_response.context_menu(|ui| {
            strengthen_menu_hover(ui, colors);
            if !read_only {
                if ui.button("new graph…").clicked() {
                    commands.open_dialog = Some(LibraryDialog::NewGraph {
                        folder: sub.path.clone(),
                        name: String::new(),
                    });
                    ui.close();
                }
                if ui.button("new folder…").clicked() {
                    commands.open_dialog = Some(LibraryDialog::NewFolder {
                        parent: sub.path.clone(),
                        name: String::new(),
                    });
                    ui.close();
                }
                ui.separator();
                if ui.button("rename…").clicked() {
                    commands.open_dialog = Some(LibraryDialog::RenameEntry {
                        path: sub.path.clone(),
                        is_folder: true,
                        name: sub.name.clone(),
                    });
                    ui.close();
                }
                if ui.button("delete…").clicked() {
                    commands.open_dialog = Some(LibraryDialog::ConfirmDelete {
                        path: sub.path.clone(),
                        is_folder: true,
                    });
                    ui.close();
                }
                ui.separator();
            }
            // Select the folder in its parent, so its location is visible.
            if ui.button("reveal in file explorer").clicked() {
                commands.reveal = Some((sub.path.clone(), true));
                ui.close();
            }
        });
    }

    for graph in &folder.graphs {
        // Graph row: double-click to open (or via the context menu). The row
        // renders selected when this graph is the focused program tab.
        let is_current = current_graph == Some(graph.path.as_path());
        let label = format!("{}  {}", icons::FILE, graph.name);
        let response = ui.selectable_label(is_current, label);

        if response.double_clicked() {
            commands.actions.push(LibraryAction::OpenGraph {
                path: graph.path.clone(),
            });
        }

        // Same open-menu marker as folder rows.
        if response.context_menu_opened() {
            ui.ctx().highlight_widget(response.id);
        }

        response.context_menu(|ui| {
            strengthen_menu_hover(ui, colors);
            if ui.button("open").clicked() {
                commands.actions.push(LibraryAction::OpenGraph {
                    path: graph.path.clone(),
                });
                ui.close();
            }
            if !read_only {
                if ui.button("rename…").clicked() {
                    commands.open_dialog = Some(LibraryDialog::RenameEntry {
                        path: graph.path.clone(),
                        is_folder: false,
                        name: graph.name.clone(),
                    });
                    ui.close();
                }
                if ui.button("delete…").clicked() {
                    commands.open_dialog = Some(LibraryDialog::ConfirmDelete {
                        path: graph.path.clone(),
                        is_folder: false,
                    });
                    ui.close();
                }
                ui.separator();
            }
            // Select the graph file in its folder.
            if ui.button("reveal in file explorer").clicked() {
                commands.reveal = Some((graph.path.clone(), true));
                ui.close();
            }
        });
    }

    // Images: list rows or thumbnail grid depending on view style.
    match view_style {
        LibraryViewStyle::List => {
            for image in &folder.images {
                show_image_list_row(ui, image, previewed_image, read_only, colors, commands);
            }
        }
        LibraryViewStyle::Thumbnails => {
            let avail = ui.available_width();
            if avail < LIBRARY_THUMB_MIN_UI {
                // Deep nest + narrow panel: fall back to list so cells never
                // force horizontal clipping.
                for image in &folder.images {
                    show_image_list_row(ui, image, previewed_image, read_only, colors, commands);
                }
            } else {
                let cell = LIBRARY_THUMB_UI.min(avail);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    for image in &folder.images {
                        show_image_thumb_cell(
                            ui,
                            image,
                            previewed_image,
                            read_only,
                            thumbs,
                            colors,
                            commands,
                            cell,
                        );
                    }
                });
            }
        }
    }

    if folder.truncated {
        // The scanner hit its depth/entry cap inside this folder; say so
        // instead of silently hiding content.
        ui.label(egui::RichText::new("…more items not shown").color(colors.text_faint));
    }
}

/// List-style image row (also the narrow-panel fallback in thumbnails mode).
fn show_image_list_row(
    ui: &mut egui::Ui,
    image: &ImageEntry,
    previewed_image: Option<&Path>,
    read_only: bool,
    colors: &ThemeValues,
    commands: &mut PanelCommands,
) {
    // Image row: single-click previews the image in the 2D view (and the
    // row highlights while it's showing there); double-click (or the
    // context menu) adds an "image from file" node to the current graph;
    // dragging the row drops that node wherever it's released on a graph
    // panel (handled by the focused program, like a node-list drag).
    let is_previewed = previewed_image == Some(image.path.as_path());
    let label = format!("{}  {}", icons::IMAGE, image.name);
    let response = ui
        .selectable_label(is_previewed, label)
        .interact(egui::Sense::click_and_drag())
        .on_hover_text(image_hover_text(image));

    wire_image_interactions(&response, image, read_only, colors, commands);
}

/// Thumbnail-grid cell for one image file.
fn show_image_thumb_cell(
    ui: &mut egui::Ui,
    image: &ImageEntry,
    previewed_image: Option<&Path>,
    read_only: bool,
    thumbs: &mut LibraryThumbCache,
    colors: &ThemeValues,
    commands: &mut PanelCommands,
    cell: f32,
) {
    let text_h = 14.0;
    let gap = 2.0;
    let total = egui::vec2(cell, cell + gap + text_h);
    let (rect, response) = ui.allocate_exact_size(total, egui::Sense::click_and_drag());

    let is_previewed = previewed_image == Some(image.path.as_path());
    let thumb_rect = egui::Rect::from_min_size(rect.min, egui::vec2(cell, cell));

    // Visible → high-priority decode. Near the viewport (prefetch band above
    // and below the clip rect) → low-priority so scrolling lands on ready
    // thumbs. Far off-screen cells stay cold so huge folders don't enqueue
    // everything ScrollArea lays out.
    let visible = ui.is_rect_visible(rect);
    let in_prefetch_band =
        prefetch_band(ui.clip_rect(), LIBRARY_THUMB_PREFETCH_VIEWPORTS).intersects(rect);
    let texture = if visible {
        thumbs.get(&image.path)
    } else if in_prefetch_band {
        thumbs.prefetch(&image.path);
        None
    } else {
        None
    };

    // Background: selection / hover using theme fills.
    if is_previewed {
        ui.painter()
            .rect_filled(rect, 3.0, colors.widgets_hovered_bg_fill);
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, 3.0, colors.widgets_interactive_bg_fill);
    }

    if let Some(tex) = texture {
        // Letterbox the texture inside the square cell.
        let tex_size = tex.size_vec2();
        let scale = (cell / tex_size.x).min(cell / tex_size.y);
        let draw_size = tex_size * scale;
        let draw_rect = egui::Rect::from_center_size(thumb_rect.center(), draw_size);
        ui.painter().image(
            tex.id(),
            draw_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    } else {
        // Placeholder while loading / failed / offscreen.
        ui.painter()
            .rect_filled(thumb_rect, 2.0, colors.widgets_interactive_bg_fill);
        ui.painter().text(
            thumb_rect.center(),
            egui::Align2::CENTER_CENTER,
            icons::IMAGE,
            egui::FontId::proportional(20.0),
            colors.text_faint,
        );
    }

    // Filename under the thumb, truncated to cell width.
    let name_pos = egui::pos2(rect.min.x, thumb_rect.max.y + gap);
    let name_rect = egui::Rect::from_min_size(name_pos, egui::vec2(cell, text_h));
    ui.painter().text(
        name_rect.left_top(),
        egui::Align2::LEFT_TOP,
        truncate_name(&image.name, cell, ui),
        egui::FontId::proportional(11.0),
        if is_previewed {
            colors.override_text_color
        } else {
            colors.text_faint
        },
    );

    let response = response.on_hover_text(image_hover_text(image));
    wire_image_interactions(&response, image, read_only, colors, commands);
}

/// Tooltip body for an image entry: full name, path, size, and modified time.
/// Metadata is read only when the tooltip is built (hover), so it does not
/// tax the scanner poll.
fn image_hover_text(image: &ImageEntry) -> String {
    let mut lines = Vec::with_capacity(4);
    lines.push(image.name.clone());
    lines.push(image.path.display().to_string());

    if let Ok(meta) = std::fs::metadata(&image.path) {
        let mut detail = format_byte_size(meta.len());
        if let Ok(modified) = meta.modified() {
            if let Some(when) = format_modified(modified) {
                detail = format!("{detail}  ·  {when}");
            }
        }
        lines.push(detail);

        // Extension hint for formats the user might not recognize by suffix.
        if let Some(ext) = image
            .path
            .extension()
            .and_then(|e| e.to_str())
            .filter(|e| !e.is_empty())
        {
            lines.push(format!("type  {ext}"));
        }
    }

    lines.join("\n")
}

/// Human-readable file size (B / KB / MB / GB).
fn format_byte_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

/// UTC calendar time for a `SystemTime`, or `None` if conversion fails.
/// (Local offset needs an optional `time` crate feature we don't enable.)
fn format_modified(modified: std::time::SystemTime) -> Option<String> {
    use std::time::UNIX_EPOCH;
    let secs = modified.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
    let dt = time::OffsetDateTime::from_unix_timestamp(secs).ok()?;
    Some(format!(
        "{:04}-{:02}-{:02} {:02}:{:02} UTC",
        dt.year(),
        dt.month() as u8,
        dt.day(),
        dt.hour(),
        dt.minute()
    ))
}

/// Shared click / drag / context-menu wiring for list rows and thumb cells.
fn wire_image_interactions(
    response: &egui::Response,
    image: &ImageEntry,
    read_only: bool,
    colors: &ThemeValues,
    commands: &mut PanelCommands,
) {
    if response.drag_started() {
        commands.actions.push(LibraryAction::BeginImageDrag {
            path: image.path.clone(),
        });
    }

    if response.clicked() {
        commands.actions.push(LibraryAction::PreviewImage {
            path: image.path.clone(),
        });
    }

    if response.double_clicked() {
        commands.actions.push(LibraryAction::AddImageNode {
            path: image.path.clone(),
        });
    }

    if response.context_menu_opened() {
        response.ctx.highlight_widget(response.id);
    }

    response.context_menu(|ui| {
        strengthen_menu_hover(ui, colors);
        if ui.button("add to current graph").clicked() {
            commands.actions.push(LibraryAction::AddImageNode {
                path: image.path.clone(),
            });
            ui.close();
        }
        if !read_only {
            if ui.button("delete…").clicked() {
                commands.open_dialog = Some(LibraryDialog::ConfirmDelete {
                    path: image.path.clone(),
                    is_folder: false,
                });
                ui.close();
            }
            ui.separator();
        }
        if ui.button("reveal in file explorer").clicked() {
            commands.reveal = Some((image.path.clone(), true));
            ui.close();
        }
    });
}

/// Truncates `name` so it fits roughly in `width` points (simple char budget).
fn truncate_name(name: &str, width: f32, ui: &egui::Ui) -> String {
    // ~6 px per character at 11 pt is a coarse estimate; prefer egui layout
    // when we need pixel-perfect, but this keeps the painter path cheap.
    let max_chars = ((width / 6.0).floor() as usize).max(4);
    if name.chars().count() <= max_chars {
        return name.to_string();
    }
    let _ = ui;
    let keep = max_chars.saturating_sub(1);
    let mut out: String = name.chars().take(keep).collect();
    out.push('…');
    out
}

/// Renders the active modal dialog (if any) and applies its outcome.
fn show_dialog(ui: &mut egui::Ui, state: &mut LibrariesState, colors: &ThemeValues) {
    let Some(dialog) = &mut state.dialog else {
        return;
    };

    let mut outcome = DialogOutcome::Open;

    let modal = egui::Modal::new(egui::Id::new("library_dialog")).show(ui.ctx(), |ui| {
        ui.set_width(280.0);

        // Text boxes paint their background with `extreme_bg_color`, which
        // several themes keep nearly identical to the modal background,
        // making the input box invisible. Swap in the theme's control
        // surface fill, which is designed to stay legible on popups.
        ui.style_mut().visuals.extreme_bg_color = colors.widgets_interactive_bg_fill;

        // Title + body per dialog kind. Name dialogs edit their embedded
        // string in place; confirm dialogs just show a sentence.
        match dialog {
            LibraryDialog::RenameLibrary { name, .. } => {
                ui.heading("rename library");
                ui.add_space(8.0);
                name_field(ui, name, &mut outcome);
            }
            LibraryDialog::NewFolder { name, .. } => {
                ui.heading("new folder");
                ui.add_space(8.0);
                name_field(ui, name, &mut outcome);
            }
            LibraryDialog::NewGraph { name, .. } => {
                ui.heading("new graph");
                ui.add_space(8.0);
                name_field(ui, name, &mut outcome);
            }
            LibraryDialog::RenameEntry { name, is_folder, .. } => {
                ui.heading(if *is_folder { "rename folder" } else { "rename graph" });
                ui.add_space(8.0);
                name_field(ui, name, &mut outcome);
            }
            LibraryDialog::ConfirmDelete { path, is_folder } => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                // Non-folder deletes can be a graph or an image; only graphs
                // carry the "an open tab will re-save it" caveat.
                let is_graph = !*is_folder && super::library_scanner::is_graph_file(&name);
                let heading = if *is_folder {
                    "delete folder"
                } else if is_graph {
                    "delete graph"
                } else {
                    "delete file"
                };
                ui.heading(heading);
                ui.add_space(8.0);
                ui.label(format!("Move '{}' to the recycle bin?", name));
                if is_graph {
                    // The engine auto-saves open graphs; warn that an open
                    // tab will recreate the file on its next save.
                    ui.label("If this graph is open in a tab, it will be re-saved on its next change.");
                }
            }
            LibraryDialog::ConfirmRemoveLibrary { .. } => {
                ui.heading("remove library");
                ui.add_space(8.0);
                ui.label("Unlink this library? Nothing on disk will be deleted.");
            }
        }

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button("ok").clicked() {
                outcome = DialogOutcome::Confirm;
            }
            if ui.button("cancel").clicked() {
                outcome = DialogOutcome::Cancel;
            }
        });
    });

    // Esc or clicking outside the modal closes it.
    if modal.should_close() && outcome == DialogOutcome::Open {
        outcome = DialogOutcome::Cancel;
    }

    match outcome {
        DialogOutcome::Open => {}
        DialogOutcome::Cancel => {
            state.dialog = None;
        }
        DialogOutcome::Confirm => {
            // Take the dialog out first so the state methods below can
            // borrow `state` mutably.
            if let Some(dialog) = state.dialog.take() {
                apply_dialog(state, dialog);
            }
        }
    }
}

/// A single-line name editor used by all name dialogs. Focuses itself when
/// nothing else has focus (i.e. when the dialog just opened) and treats
/// Enter as OK.
fn name_field(ui: &mut egui::Ui, name: &mut String, outcome: &mut DialogOutcome) {
    let response = ui.text_edit_singleline(name);
    if ui.memory(|mem| mem.focused().is_none()) {
        response.request_focus();
    }
    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        *outcome = DialogOutcome::Confirm;
    }
}

/// Performs a confirmed dialog's operation against the state.
fn apply_dialog(state: &mut LibrariesState, dialog: LibraryDialog) {
    match dialog {
        LibraryDialog::RenameLibrary { id, name } => {
            // Display name only — not a filename, so no sanitizing.
            if !name.trim().is_empty() {
                state.rename_library(id, name.trim().to_string());
            }
        }
        LibraryDialog::NewFolder { parent, name } => {
            let sanitized = LibrariesState::sanitize(name.trim());
            if !sanitized.is_empty() {
                state.create_folder(&parent, &sanitized);
            }
        }
        LibraryDialog::NewGraph { folder, name } => {
            let name = name.trim().to_string();
            // Same filename derivation as the graph-settings save dialog:
            // sanitize (keeping spaces) then append the canonical extension,
            // so the display name and the on-disk file-name stem agree.
            let file_name = mangler_core::naming::graph_file_name(&name);
            if file_name != mangler_core::naming::GRAPH_EXTENSION {
                let path = folder.join(file_name);
                state.push_action(LibraryAction::CreateGraph { path, name });
            }
        }
        LibraryDialog::RenameEntry { path, is_folder: _, name } => {
            // Folders and graph files are sanitized the same way now that
            // graph filenames preserve spaces (`rename_path` re-appends the
            // `.mangler.json` extension itself for graphs).
            let sanitized = LibrariesState::sanitize(name.trim());
            if !sanitized.is_empty() {
                state.rename_path(&path, &sanitized);
            }
        }
        LibraryDialog::ConfirmDelete { path, .. } => {
            state.delete_to_trash(&path);
        }
        LibraryDialog::ConfirmRemoveLibrary { id } => {
            state.remove_library(id);
        }
    }
}

/// Shows the most recent disk-operation error at the bottom of the panel,
/// fading out after a few seconds (same idiom as the graph status message).
fn show_error_strip(ui: &mut egui::Ui, state: &mut LibrariesState, colors: &ThemeValues) {
    const SHOW_FOR: Duration = Duration::from_secs(5);
    const FADE_SECS: f32 = 1.0;

    if let Some((message, at)) = &state.error {
        let elapsed = at.elapsed();
        if elapsed < SHOW_FOR {
            // Fade alpha to zero over the last `FADE_SECS`.
            let remaining = (SHOW_FOR - elapsed).as_secs_f32();
            let alpha = (remaining / FADE_SECS).clamp(0.0, 1.0);
            let color = colors.grid_connection_dot_error.gamma_multiply(alpha);
            ui.label(egui::RichText::new(message).color(color));
            // Keep repainting so the fade animates even when idle.
            ui.ctx().request_repaint();
        } else {
            state.error = None;
        }
    }
}
