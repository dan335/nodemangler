//! egui renderer for a [`PanelTree`]: draws the draggable splitters, each
//! leaf's content (via `Program::show_panel`), and the corner kind-switcher
//! chrome. Every color comes from the active [`Theme`] so all four themes
//! stay consistent when switched at runtime.

use eframe::egui::{self, Sense, UiBuilder};
use epaint::{pos2, vec2, CornerRadius, Rect};

use crate::{
    libraries::libraries_state::LibrariesState,
    panels::{
        panel_kind::PanelKind,
        panel_tree::{LeafId, PanelTree, SplitDirection},
    },
    program::Program,
    themes::theme::Theme,
};

/// Which OS window a panel tree belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelWindowId {
    Main,
    /// A secondary OS window.
    Secondary(u64),
}

/// A panel-management command raised by the app settings menu or a panel's
/// corner-button menu this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelAction {
    NewWindow,
    SplitHorizontal { window: PanelWindowId, leaf: LeafId },
    SplitVertical { window: PanelWindowId, leaf: LeafId },
    ClosePanel { window: PanelWindowId, leaf: LeafId },
    SaveLayoutAsDefault,
    ResetLayout,
}

/// What [`render_tree`] produced this frame.
pub struct TreeRenderResponse {
    /// On-screen rects of every Graph-kind leaf, for overlay hit-tests. Paired
    /// with each leaf's id so a hit-test can map back to that panel's own
    /// [`crate::graph::graph_editor::GraphCamera`].
    pub graph_rects: Vec<(LeafId, Rect)>,
    /// A split/close command raised from a panel's corner button this frame.
    /// The action already carries its own target window/leaf, so the app can
    /// route it through the same `handle_panel_action` path as the settings
    /// menu without any extra lookup.
    pub panel_action: Option<PanelAction>,
}

/// Render one panel tree into `ui`, filling `work_rect`.
///
/// Splitters are interactive (drag to resize), each leaf renders its content
/// clipped to its rect, and a corner button switches the leaf's kind or raises
/// a split/close [`PanelAction`] targeting that leaf.
pub fn render_tree(
    ui: &mut egui::Ui,
    tree: &mut PanelTree,
    work_rect: Rect,
    window: PanelWindowId,
    program: &mut Program,
    libraries: &mut LibrariesState,
    theme: &Theme,
) -> TreeRenderResponse {
    let colors = theme.get();
    let layout = tree.layout(work_rect);

    // --- splitters ---------------------------------------------------------
    for splitter in &layout.splitters {
        // Expand the hit area a little for easier grabbing, but paint only the
        // 4px strip.
        let hit_rect = splitter.rect.expand2(match splitter.direction {
            SplitDirection::Row => vec2(2.0, 0.0),
            SplitDirection::Column => vec2(0.0, 2.0),
        });
        let response = ui.allocate_rect(hit_rect, Sense::drag());

        if response.hovered() || response.dragged() {
            let cursor = match splitter.direction {
                SplitDirection::Row => egui::CursorIcon::ResizeHorizontal,
                SplitDirection::Column => egui::CursorIcon::ResizeVertical,
            };
            ui.ctx().set_cursor_icon(cursor);
        }

        if response.dragged() {
            if let Some(pointer) = ui.ctx().pointer_interact_pos() {
                // Drag trades space only between the two panels touching this
                // divider (Blender behavior); non-adjacent panels keep their
                // pixel size. `parent_rect` is the split node's own rect.
                let pointer_coord = match splitter.direction {
                    SplitDirection::Row => pointer.x,
                    SplitDirection::Column => pointer.y,
                };
                tree.drag_splitter(&splitter.path, splitter.parent_rect, pointer_coord);
            }
        }

        // Chrome strip matching the menu bar so the gaps read as part of the
        // app frame rather than as harsh separators; nudged toward the
        // selected-button accent while hovered/dragged so the strip a user is
        // about to grab (or is grabbing) stands out slightly.
        let strip_color = if response.dragged() {
            colors.menu_bar.lerp_to_gamma(colors.menu_bar_button_selected, 0.6)
        } else if response.hovered() {
            colors.menu_bar.lerp_to_gamma(colors.menu_bar_button_selected, 0.3)
        } else {
            colors.menu_bar
        };
        ui.painter()
            .rect_filled(splitter.rect, CornerRadius::ZERO, strip_color);
    }

    // --- leaves ------------------------------------------------------------
    let mut graph_rects = Vec::new();
    // Split/close raised from a corner button this frame (targets the panel the
    // button belongs to).
    let mut panel_action: Option<PanelAction> = None;

    for &(id, kind, rect) in &layout.leaves {
        if kind == PanelKind::Graph {
            graph_rects.push((id, rect));
        }

        // Panel content. `push_id` is mandatory: duplicate Settings/NodeList
        // panels would otherwise clash egui widget ids.
        ui.push_id(id, |ui| {
            ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                ui.set_clip_rect(rect);
                program.show_panel(ui, id, kind, theme, libraries);
            });
        });

        // Corner kind-switcher, drawn after the content so it wins the pointer.
        let btn_rect = Rect::from_min_size(
            pos2(rect.right() - 26.0, rect.top() + 4.0),
            vec2(20.0, 20.0),
        );
        let mut selected_kind: Option<PanelKind> = None;
        // Split/close chosen from this panel's popup, applied after the closure.
        let mut chosen_action: Option<PanelAction> = None;
        ui.push_id((id, "panel_chrome"), |ui| {
            let resp = ui.put(btn_rect, egui::Button::new(kind.icon()).small().frame(false));
            egui::Popup::menu(&resp).show(|ui| {
                for k in PanelKind::ALL {
                    if ui.button(format!("{}  {}", k.icon(), k.label())).clicked() {
                        selected_kind = Some(k);
                    }
                }
                ui.separator();
                // Panel management for this specific panel; the payload
                // carries this panel's own window/leaf as the target.
                if ui.button("split horizontal").clicked() {
                    chosen_action = Some(PanelAction::SplitHorizontal { window, leaf: id });
                }
                if ui.button("split vertical").clicked() {
                    chosen_action = Some(PanelAction::SplitVertical { window, leaf: id });
                }
                if ui.button("close panel").clicked() {
                    chosen_action = Some(PanelAction::ClosePanel { window, leaf: id });
                }
            });
        });
        if let Some(k) = selected_kind {
            tree.set_kind(id, k);
        }
        if let Some(action) = chosen_action {
            // Raise the command for the app to apply after rendering.
            panel_action = Some(action);
        }
    }

    TreeRenderResponse {
        graph_rects,
        panel_action,
    }
}
