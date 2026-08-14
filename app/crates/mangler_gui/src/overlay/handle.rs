//! Draggable handles and click catchers for overlay editors.
//!
//! ## Hit-testing (egui 0.35 spike, verified against the vendored source)
//!
//! This is the load-bearing knowledge for every overlay drawn over the 2D
//! preview, stated here once instead of in each editor.
//!
//! An overlay renders *after* the image viewer's full-rect `Sense::drag()`
//! background (`view_window::image_viewer`), so its widgets are topmost. egui
//! resolves the click winner and the drag winner **independently**
//! (`hit_test.rs`), which is what lets both gestures coexist on one canvas:
//!
//! - [`handle`] uses `Sense::click_and_drag()`. A topmost click-and-drag widget
//!   wins the drag over the earlier full-rect drag background
//!   (`buttons_on_window` test), so dragging a handle moves it and never pans.
//! - [`catcher`] uses `Sense::click()` **only**. A click-only widget over a
//!   drag-only background takes the click but leaves the drag to the background,
//!   so dragging empty space still pans the image. Never give a full-rect
//!   catcher `click_and_drag` — that would kill panning.
//! - Registration order breaks ties between *overlapping* widgets: the later
//!   registration wins. Register the catcher first (so handles win clicks), then
//!   knobs, then anchors (so an anchor wins when a short tangent sits on it).
//! - Salt every id with something panel-unique (`LeafId`), since the same
//!   overlay is drawn by every open 2D panel, including ones in secondary OS
//!   windows.
//!
//! If a future egui version changed this resolution, the fallback is to drop
//! [`catcher`] and detect click-on-empty from pointer press/release edges plus a
//! movement threshold via `pan_zoom::viewport_cursor` — keeping the catcher
//! behind this one function is what keeps that swap local.

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::themes::theme::Theme;

/// What a handle reported this frame.
pub struct HandleResponse {
    /// Pointer position while the handle is being dragged. Use for *absolute*
    /// placement (move this point to the pointer).
    pub drag_to: Option<Pos2>,
    /// This frame's drag displacement. Use for *incremental* placement (nudge a
    /// whole shape), where absolute would teleport it to the pointer on grab.
    pub drag_delta: Vec2,
    /// The drag ended this frame. Note the pointer has not moved on this frame,
    /// so `drag_to` is `None` — see [`super::Gesture`] for the full asymmetry.
    pub commit: bool,
    /// The drag *began* this frame. The cue for snapshotting the values a drag
    /// should be measured from, which is what makes a drag exact on an integer
    /// input: per-frame deltas can each round to zero at high zoom and lose the
    /// whole gesture, while `value at press + total pointer travel` cannot.
    pub started: bool,
    /// A delete was requested (double-click or right-click). The caller applies
    /// its own floor; this only reports the input.
    pub delete: bool,
    /// Hovered or actively dragged — the cue for enlarging the handle.
    pub active: bool,
}

/// The interaction rect of a handle centered at `center`.
pub fn hit_rect(center: Pos2, hit_half: f32) -> Rect {
    Rect::from_center_size(center, Vec2::splat(hit_half * 2.0))
}

/// A draggable point handle: `Sense::click_and_drag()` so it wins the drag over
/// the pan background, and reports double/right-click as a delete request.
pub fn handle(ui: &mut egui::Ui, id: egui::Id, center: Pos2, hit_half: f32) -> HandleResponse {
    let resp = ui.interact(hit_rect(center, hit_half), id, Sense::click_and_drag());
    HandleResponse {
        drag_to: resp.dragged().then(|| resp.interact_pointer_pos()).flatten(),
        drag_delta: resp.drag_delta(),
        commit: resp.drag_stopped(),
        started: resp.drag_started(),
        delete: resp.double_clicked() || resp.clicked_by(egui::PointerButton::Secondary),
        active: resp.hovered() || resp.dragged(),
    }
}

/// A draggable region of arbitrary shape — a box's interior, an edge band.
/// Same senses as [`handle`], but sized by a rect instead of a centre point.
pub fn region(ui: &mut egui::Ui, id: egui::Id, rect: Rect) -> HandleResponse {
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    HandleResponse {
        drag_to: resp.dragged().then(|| resp.interact_pointer_pos()).flatten(),
        drag_delta: resp.drag_delta(),
        commit: resp.drag_stopped(),
        started: resp.drag_started(),
        delete: false,
        active: resp.hovered() || resp.dragged(),
    }
}

/// A tangent knob: `Sense::drag()` only, so a click that lands on both a knob
/// and its anchor still reaches the anchor. Register knobs *before* anchors.
pub fn knob(ui: &mut egui::Ui, id: egui::Id, center: Pos2, hit_half: f32) -> HandleResponse {
    let resp = ui.interact(hit_rect(center, hit_half), id, Sense::drag());
    HandleResponse {
        drag_to: resp.dragged().then(|| resp.interact_pointer_pos()).flatten(),
        drag_delta: resp.drag_delta(),
        commit: resp.drag_stopped(),
        started: resp.drag_started(),
        delete: false,
        active: resp.hovered() || resp.dragged(),
    }
}

/// An empty-space click catcher over `rect`. Returns the click position when
/// the click did not land on any handle registered *after* this call.
///
/// `Sense::click()` only — see the module docs. Read it first, apply its result
/// last, so a click that a handle won is correctly excluded.
pub fn catcher(ui: &mut egui::Ui, id: egui::Id, rect: Rect) -> CatcherResponse {
    let resp = ui.interact(rect, id, Sense::click());
    CatcherResponse { clicked_at: resp.clicked().then(|| resp.interact_pointer_pos()).flatten() }
}

/// What an empty-space catcher reported this frame.
pub struct CatcherResponse {
    /// Where the user clicked, if the click reached the catcher.
    pub clicked_at: Option<Pos2>,
}

/// How a handle is painted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleShape {
    /// Filled circle — control points on a path.
    Dot,
    /// Filled square — resize grips on a box.
    Square,
}

/// Paint a handle. Colors come from the theme; `active` enlarges it and swaps to
/// the hover fill, which is the affordance that tells the user it is grabbable.
pub fn draw_handle(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    active: bool,
    shape: HandleShape,
    theme: &Theme,
) {
    let colors = theme.get();
    let fill = if active { colors.grid_connection_dot_hover } else { colors.grid_connection_dot };
    let stroke = Stroke::new(1.5, colors.node_header_selected_border);
    match shape {
        HandleShape::Dot => {
            painter.circle(center, radius, fill, stroke);
        }
        HandleShape::Square => {
            painter.rect(
                Rect::from_center_size(center, Vec2::splat(radius * 2.0)),
                0.0,
                fill,
                stroke,
                epaint::StrokeKind::Inside,
            );
        }
    }
}

/// Paint a ring around a handle, used to distinguish a path's first point.
pub fn draw_handle_ring(painter: &egui::Painter, center: Pos2, radius: f32, theme: &Theme) {
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(1.5, theme.get().node_header_selected_border),
    );
}

/// Paint a tangent knob and its leader line back to the anchor.
pub fn draw_knob(
    painter: &egui::Painter,
    anchor: Pos2,
    knob: Pos2,
    radius: f32,
    theme: &Theme,
) {
    let colors = theme.get();
    painter.line_segment([anchor, knob], Stroke::new(1.0, colors.text_faint));
    painter.circle(
        knob,
        radius,
        colors.panel_fill,
        Stroke::new(1.5, colors.node_header_selected_border),
    );
}

/// Paint a guide line spanning `rect` through `at`, used by crosshairs and
/// mirror-axis gizmos. Quiet by design — the handle is the focus, not the rule.
pub fn draw_guide(painter: &egui::Painter, rect: Rect, at: Pos2, vertical: bool, theme: &Theme) {
    let stroke = Stroke::new(1.0, theme.get().text_faint);
    let (a, b) = if vertical {
        (Pos2::new(at.x, rect.top()), Pos2::new(at.x, rect.bottom()))
    } else {
        (Pos2::new(rect.left(), at.y), Pos2::new(rect.right(), at.y))
    };
    painter.line_segment([a, b], stroke);
}

/// The dimmed stroke colour used when an overlay is read-only because its
/// inputs are driven from upstream.
pub fn read_only_color(theme: &Theme) -> Color32 {
    theme.get().text_faint.gamma_multiply(0.7)
}
