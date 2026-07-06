//! egui renderer for a [`PanelTree`]: draws the draggable splitters, each
//! leaf's content (via `Program::show_panel`), the corner kind-switcher chrome,
//! and the focus highlight. Every color comes from the active [`Theme`] so all
//! four themes stay consistent when switched at runtime.

use eframe::egui::{self, Sense, UiBuilder};
use epaint::{pos2, vec2, CornerRadius, Rect, Stroke, StrokeKind};

use crate::{
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

/// The last-focused (hovered/clicked) panel — the target for split/close.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelFocus {
    pub window: PanelWindowId,
    pub leaf: LeafId,
}

/// A panel-management command raised by the app settings menu this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelAction {
    NewWindow,
    SplitHorizontal,
    SplitVertical,
    ClosePanel,
    SaveLayoutAsDefault,
    ResetLayout,
}

/// What [`render_tree`] produced this frame.
pub struct TreeRenderResponse {
    /// On-screen rects of every Graph-kind leaf, for overlay hit-tests.
    pub graph_rects: Vec<Rect>,
}

/// Render one panel tree into `ui`, filling `work_rect`.
///
/// Splitters are interactive (drag to resize), each leaf renders its content
/// clipped to its rect, and a corner button switches the leaf's kind. `focused`
/// is updated (sticky) to the panel under the pointer.
pub fn render_tree(
    ui: &mut egui::Ui,
    tree: &mut PanelTree,
    work_rect: Rect,
    window: PanelWindowId,
    focused: &mut Option<PanelFocus>,
    program: &mut Program,
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
                let parent = splitter.parent_rect;
                let fraction = match splitter.direction {
                    SplitDirection::Row => (pointer.x - parent.min.x) / parent.width().max(1.0),
                    SplitDirection::Column => (pointer.y - parent.min.y) / parent.height().max(1.0),
                };
                tree.set_fraction(&splitter.path, fraction);
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
    let hover_pos = ui.ctx().pointer_hover_pos();

    for &(id, kind, rect) in &layout.leaves {
        if kind == PanelKind::Graph {
            graph_rects.push(rect);
        }

        // Panel content. `push_id` is mandatory: duplicate Settings/NodeList
        // panels would otherwise clash egui widget ids.
        ui.push_id(id, |ui| {
            ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                ui.set_clip_rect(rect);
                program.show_panel(ui, id, kind, theme);
            });
        });

        // Sticky focus: hovering a panel focuses it; nothing clears focus here.
        if let Some(pos) = hover_pos {
            if rect.contains(pos) {
                *focused = Some(PanelFocus { window, leaf: id });
            }
        }

        // Corner kind-switcher, drawn after the content so it wins the pointer.
        let btn_rect = Rect::from_min_size(
            pos2(rect.right() - 26.0, rect.top() + 4.0),
            vec2(20.0, 20.0),
        );
        let mut selected_kind: Option<PanelKind> = None;
        ui.push_id((id, "panel_chrome"), |ui| {
            let resp = ui.put(btn_rect, egui::Button::new(kind.icon()).small().frame(false));
            egui::Popup::menu(&resp).show(|ui| {
                for k in PanelKind::ALL {
                    if ui.button(format!("{}  {}", k.icon(), k.label())).clicked() {
                        selected_kind = Some(k);
                    }
                }
            });
        });
        if let Some(k) = selected_kind {
            tree.set_kind(id, k);
        }

        // Focus highlight: a subtle 1px inside stroke on the focused leaf.
        if *focused == Some(PanelFocus { window, leaf: id }) {
            let color = colors.menu_bar_button_selected.gamma_multiply(0.6);
            ui.painter().rect_stroke(
                rect.shrink(1.0),
                CornerRadius::ZERO,
                Stroke::new(1.0, color),
                StrokeKind::Inside,
            );
        }
    }

    TreeRenderResponse { graph_rects }
}
