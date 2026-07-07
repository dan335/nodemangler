//! Secondary OS windows: each hosts its own [`PanelTree`] in a separate egui
//! viewport (draggable to another monitor). Content renders the current
//! [`Program`], the same way the main window's panels do.

use eframe::egui::{self, ViewportBuilder, ViewportId};
use epaint::CornerRadius;

use crate::{
    libraries::libraries_state::LibrariesState,
    panels::{
        panel_tree::PanelTree,
        panel_view::{self, PanelAction, PanelWindowId},
    },
    program::Program,
    themes::theme::Theme,
};

/// A secondary OS window hosting a panel tree. Session-only (v1): not persisted
/// to the default layout.
pub struct SecondaryWindow {
    pub id: u64,
    pub tree: PanelTree,
    /// Set when the window's titlebar close button was pressed; the app removes
    /// the window next frame.
    pub close_requested: bool,
}

impl SecondaryWindow {
    /// Stable viewport id derived from the window id.
    pub fn viewport_id(&self) -> ViewportId {
        ViewportId::from_hash_of(("panel_window", self.id))
    }
}

/// Render one secondary window's panel tree into its own OS viewport.
///
/// Graph panels here support pan/zoom/select/connect and node-drop-from-list
/// like the main window's do — all pointer input is read per-viewport.
/// Tab-search and the status message still render only in the main window.
/// Acceptable for v1.
///
/// Returns any split/close command raised by a panel's corner button this
/// frame, for the app to apply after rendering.
pub fn show_secondary_window(
    ctx: &egui::Context,
    win: &mut SecondaryWindow,
    program: &mut Program,
    libraries: &mut LibrariesState,
    theme: &Theme,
) -> Option<PanelAction> {
    let window = PanelWindowId::Secondary(win.id);
    let viewport_id = win.viewport_id();
    let mut panel_action = None;

    ctx.show_viewport_immediate(
        viewport_id,
        ViewportBuilder::default()
            .with_title("NodeMangler — panel")
            .with_inner_size([700.0, 500.0]),
        |ctx, _class| {
            // CentralPanel::show is deprecated in egui 0.34 in favor of
            // show_inside, but show_inside requires a Ui — which only
            // CentralPanel::show can produce at the top level of a viewport.
            // egui itself wraps its internals with the same allow.
            #[allow(deprecated)]
            egui::CentralPanel::default().show(ctx, |ui| {
                let work_rect = ui.max_rect();
                ui.painter().add(egui::Shape::rect_filled(
                    work_rect,
                    CornerRadius::ZERO,
                    theme.get().panel_fill,
                ));

                let resp = panel_view::render_tree(
                    ui,
                    &mut win.tree,
                    work_rect,
                    window,
                    program,
                    libraries,
                    theme,
                );
                panel_action = resp.panel_action;

                // Node-list drag handling for this window: drop into this
                // window's graph panels, ghost node under this window's
                // pointer. (Tab-search/status overlays stay main-window-only.)
                program.show_menu_drag(ui, &resp.graph_rects, theme);
            });

            if ctx.input(|i| i.viewport().close_requested()) {
                win.close_requested = true;
            }
        },
    );

    panel_action
}
