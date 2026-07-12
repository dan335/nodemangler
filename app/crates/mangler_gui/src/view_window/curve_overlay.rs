//! Interactive curve-editing overlay for the 2D preview panel.
//!
//! Renders a [`Curve`] on top of the displayed image (or a fallback canvas) and
//! lets the user draw it directly: drag control points, click empty space to
//! insert/append a point, double- or right-click a point to delete it, drag
//! the mirrored tangent knobs in Bezier mode to shape curvature, and toggle
//! closed/interpolation from a small strip. This is a *pure widget* — it
//! has no engine knowledge and holds no persistent state (egui tracks drags per
//! widget id; ids are salted with the panel's `leaf_id` so two 2D panels don't
//! collide). The caller applies [`CurveOverlayResponse::changed`] to its local
//! value every frame and pushes it to the engine only when `commit` is set.
//!
//! ## Hit-testing (egui 0.35 spike, verified against the vendored source)
//! The overlay renders *after* the image viewer's full-rect `Sense::drag()`
//! background, so its widgets are topmost. egui resolves the click winner and
//! the drag winner independently (`hit_test.rs`):
//! - Per-point handles use `Sense::click_and_drag()`. A topmost click-and-drag
//!   handle wins the drag over the earlier full-rect drag background
//!   (`buttons_on_window` test), so dragging a handle moves it and never pans.
//! - The empty-space catcher uses `Sense::click()` only. A click-only widget
//!   over a drag-only background takes the click but leaves the drag to the
//!   background, so dragging empty space still pans the image.
//!
//! If a future egui version changed this resolution, the fallback is to drop
//! the click-catcher and detect click-on-empty from pointer press/release edges
//! plus a movement threshold via `pan_zoom::viewport_cursor` — the catcher is
//! isolated in [`handle_insert`] to keep that swap local.

use eframe::egui::{self, Pos2, Rect, Sense, Stroke, Vec2};
use mangler_core::curve::{Curve, CurveInterpolation};

use crate::panels::panel_tree::LeafId;
use crate::themes::theme::Theme;

/// The result of one overlay frame.
pub struct CurveOverlayResponse {
    /// The mutated curve, when a gesture changed it this frame. The caller
    /// mirrors this into its local input value for instant visual feedback.
    pub changed: Option<Curve>,
    /// Whether the gesture *completed* (drag release, insert, delete, or a
    /// strip toggle) and the change should be pushed to the engine. Never set
    /// mid-drag, so heavy downstream nodes re-run once per drag, not per frame.
    /// A drag's release frame sets this with `changed: None` (the pointer no
    /// longer moved) — the caller must push its accumulated local value.
    pub commit: bool,
}

/// Screen-pixel radius that a click must be within to insert a point on a
/// segment (rather than appending to the end of the curve).
const INSERT_THRESHOLD_PX: f32 = 10.0;
/// Half-width of a control point's interaction rect, in screen pixels.
const HANDLE_HIT_HALF: f32 = 8.0;
/// Half-width of a bezier tangent knob's interaction rect, in screen pixels.
/// Smaller than the anchors', and knobs are registered *before* the anchors,
/// so an anchor wins when a short handle overlaps it.
const KNOB_HIT_HALF: f32 = 6.0;

/// Draw the interactive overlay and return any change made this frame.
pub fn show(
    ui: &mut egui::Ui,
    leaf_id: LeafId,
    view_rect: Rect,
    image_rect: Rect,
    curve: &Curve,
    theme: &Theme,
) -> CurveOverlayResponse {
    let colors = theme.get();
    let mut working = curve.clone();
    let mut changed = false;
    let mut commit = false;

    // Empty-space click catcher (click-only, so it never steals the pan). Read
    // its response first; the actual insert is applied after the handle loop so
    // a click that landed on a handle (egui's single click winner) is excluded.
    let catcher = ui.interact(
        view_rect,
        egui::Id::new(("curve_overlay_catcher", leaf_id)),
        Sense::click(),
    );

    // The flattened path (topmost visual, drawn under the handles).
    draw_polyline(ui.painter(), image_rect, &working, Stroke::new(2.0, colors.grid_connection_line));

    // Bezier tangent knobs: one mirrored pair per anchor. Registered before the
    // anchor widgets so anchors win overlapping hits (a near-zero handle sits
    // on its anchor). Dragging either knob rewrites the shared offset, so the
    // twin follows point-reflected — the curve stays smooth by construction.
    if working.interpolation == CurveInterpolation::Bezier {
        // Materialize silently: auto tangents become concrete so a drag can
        // write `handles[i]`. Not an edit until a gesture actually changes one.
        working.materialize_handles();
        for i in 0..working.points.len() {
            let anchor = norm_to_screen(image_rect, working.points[i]);
            let h = working.handles[i];
            let offset = Vec2::new(h[0] * image_rect.width(), h[1] * image_rect.height());
            // side 1.0 = out-knob (anchor + h), side -1.0 = in-knob (anchor - h).
            for (side_idx, sign) in [(0u8, 1.0f32), (1, -1.0)] {
                let knob = anchor + offset * sign;
                let hit = Rect::from_center_size(knob, Vec2::splat(KNOB_HIT_HALF * 2.0));
                let resp = ui.interact(
                    hit,
                    egui::Id::new(("curve_overlay_knob", leaf_id, i, side_idx)),
                    Sense::drag(),
                );
                if resp.dragged() {
                    if let Some(pos) = resp.interact_pointer_pos() {
                        // Unclamped: handle tips may leave the canvas.
                        working.handles[i] = [
                            (pos.x - anchor.x) / image_rect.width().max(1e-6) * sign,
                            (pos.y - anchor.y) / image_rect.height().max(1e-6) * sign,
                        ];
                        changed = true;
                    }
                }
                if resp.drag_stopped() {
                    commit = true;
                }

                let hovered = resp.hovered() || resp.dragged();
                let radius = if hovered { 4.5 } else { 3.0 };
                ui.painter().line_segment([anchor, knob], Stroke::new(1.0, colors.text_faint));
                ui.painter().circle(
                    knob,
                    radius,
                    colors.panel_fill,
                    Stroke::new(1.5, colors.node_header_selected_border),
                );
            }
        }
    }

    // Control-point handles. Deletion changes indices, so defer it past the loop.
    let mut delete_index: Option<usize> = None;
    for i in 0..working.points.len() {
        let center = norm_to_screen(image_rect, working.points[i]);
        let hit = Rect::from_center_size(center, Vec2::splat(HANDLE_HIT_HALF * 2.0));
        let resp = ui.interact(
            hit,
            egui::Id::new(("curve_overlay_pt", leaf_id, i)),
            Sense::click_and_drag(),
        );

        if resp.dragged() {
            if let Some(pos) = resp.interact_pointer_pos() {
                working.points[i] = screen_to_norm(image_rect, pos);
                changed = true;
            }
        }
        if resp.drag_stopped() {
            commit = true;
        }
        // Double- or right-click removes the point, with a floor of 2.
        if (resp.double_clicked() || resp.clicked_by(egui::PointerButton::Secondary))
            && working.points.len() > 2
        {
            delete_index = Some(i);
        }

        // Enlarge the handle while hovered or dragged for a clear affordance.
        let hovered = resp.hovered() || resp.dragged();
        let radius = if hovered { 6.0 } else { 4.0 };
        let fill = if hovered { colors.grid_connection_dot_hover } else { colors.grid_connection_dot };
        ui.painter().circle(center, radius, fill, Stroke::new(1.5, colors.node_header_selected_border));

        // The first point gets a distinguishing ring so start/end are legible.
        if i == 0 {
            ui.painter().circle_stroke(center, radius + 3.0, Stroke::new(1.5, colors.node_header_selected_border));
        }
    }

    if let Some(idx) = delete_index {
        // Keep handles index-aligned with points (only when they already are —
        // a stale mismatched vec is left for `materialize_handles` to rebuild).
        if working.handles.len() == working.points.len() {
            working.handles.remove(idx);
        }
        working.points.remove(idx);
        changed = true;
        commit = true;
    }

    // Insert/append on a click that missed every handle.
    if catcher.clicked() {
        if let Some(pos) = catcher.interact_pointer_pos() {
            handle_insert(&mut working, image_rect, pos);
            changed = true;
            commit = true;
        }
    }

    // Controls strip pinned to the panel's top-left corner.
    if show_controls(ui, leaf_id, view_rect, &mut working, theme) {
        changed = true;
        commit = true;
    }

    CurveOverlayResponse {
        changed: changed.then_some(working),
        commit,
    }
}

/// Insert a new control point where the user clicked: on the nearest segment if
/// the click is within [`INSERT_THRESHOLD_PX`], otherwise appended to the end.
/// Isolated so a future manual click-detection fallback can replace only this.
fn handle_insert(working: &mut Curve, image_rect: Rect, click: Pos2) {
    let screen_pts: Vec<[f32; 2]> = working
        .points
        .iter()
        .map(|p| {
            let s = norm_to_screen(image_rect, *p);
            [s.x, s.y]
        })
        .collect();

    let aligned = working.handles.len() == working.points.len();
    let idx = match nearest_segment_insertion(&screen_pts, working.closed, [click.x, click.y]) {
        Some((idx, dist, _)) if dist <= INSERT_THRESHOLD_PX => {
            working.points.insert(idx, screen_to_norm(image_rect, click));
            idx
        }
        _ => {
            working.points.push(screen_to_norm(image_rect, click));
            working.points.len() - 1
        }
    };
    // Keep handles index-aligned with points; the new anchor gets the auto
    // tangent its (post-insert) neighbors imply, so the bezier doesn't kink.
    if aligned {
        working.handles.insert(idx, [0.0, 0.0]);
        working.handles[idx] = working.auto_handle(idx);
    }
}

/// Draw the closed/interpolation/point-count strip. Returns true if the user
/// changed a control this frame (a completed edit, so the caller commits).
fn show_controls(
    ui: &mut egui::Ui,
    leaf_id: LeafId,
    view_rect: Rect,
    working: &mut Curve,
    theme: &Theme,
) -> bool {
    let mut changed = false;
    let strip_rect = Rect::from_min_size(view_rect.left_top() + Vec2::new(8.0, 8.0), Vec2::new(280.0, 26.0));
    ui.painter().rect_filled(strip_rect, 4.0, theme.get().panel_fill);

    ui.scope_builder(
        egui::UiBuilder::new().max_rect(strip_rect.shrink2(Vec2::new(6.0, 2.0))),
        |ui| {
            ui.horizontal_centered(|ui| {
                if ui.checkbox(&mut working.closed, "closed").changed() {
                    changed = true;
                }

                let mut interp = working.interpolation;
                egui::ComboBox::from_id_salt(("curve_overlay_interp", leaf_id))
                    .selected_text(interp_name(interp))
                    .show_ui(ui, |ui| {
                        for variant in CurveInterpolation::types() {
                            ui.selectable_value(&mut interp, variant, interp_name(variant));
                        }
                    });
                if interp != working.interpolation {
                    working.interpolation = interp;
                    changed = true;
                }

                ui.label(
                    egui::RichText::new(format!("{} pts", working.points.len()))
                        .color(theme.get().text_faint),
                );
            });
        },
    );

    changed
}

/// Read-only paint of a curve into `image_rect`: the flattened polyline plus
/// control-point dots and a distinguishing ring on the first point. Used by the
/// preview panel's `Value::Curve` arm and reused by [`show`] for the polyline.
pub fn draw_curve(
    painter: &egui::Painter,
    image_rect: Rect,
    curve: &Curve,
    stroke: Stroke,
    theme: &Theme,
) {
    draw_polyline(painter, image_rect, curve, stroke);
    let colors = theme.get();
    for (i, p) in curve.points.iter().enumerate() {
        let center = norm_to_screen(image_rect, *p);
        painter.circle(center, 4.0, colors.grid_connection_dot, Stroke::new(1.5, colors.node_header_selected_border));
        if i == 0 {
            painter.circle_stroke(center, 7.0, Stroke::new(1.5, colors.node_header_selected_border));
        }
    }
}

/// Draw just the flattened path (including the closing segment when the curve
/// is closed — `Curve::flatten` re-appends the first point in that case).
fn draw_polyline(painter: &egui::Painter, image_rect: Rect, curve: &Curve, stroke: Stroke) {
    // 48 samples/span matches the rasterizer's standard tolerance — a single
    // high-curvature bezier span stays smooth on screen.
    let poly = curve.flatten(48);
    if poly.len() < 2 {
        return;
    }
    let pts: Vec<Pos2> = poly.iter().map(|p| norm_to_screen(image_rect, *p)).collect();
    painter.add(egui::Shape::line(pts, stroke));
}

/// A letterboxed square canvas centered in `view_rect`, used when no image is
/// displayed to draw over (a curve is being edited/viewed on its own).
pub fn fallback_canvas_rect(view_rect: Rect) -> Rect {
    let size = view_rect.width().min(view_rect.height()) * 0.9;
    Rect::from_center_size(view_rect.center(), Vec2::splat(size))
}

/// Display name for an interpolation kind (matches the settings-panel summary).
fn interp_name(interp: CurveInterpolation) -> &'static str {
    match interp {
        CurveInterpolation::Linear => "linear",
        CurveInterpolation::Smooth => "smooth",
        CurveInterpolation::Bezier => "bezier",
    }
}

/// Map a normalized `[0,1]²` point to a screen position within `rect`.
fn norm_to_screen(rect: Rect, p: [f32; 2]) -> Pos2 {
    Pos2::new(rect.left() + p[0] * rect.width(), rect.top() + p[1] * rect.height())
}

/// Map a screen position to a normalized `[0,1]²` point within `rect`,
/// clamped to the unit square (points can't leave the canvas).
fn screen_to_norm(rect: Rect, pos: Pos2) -> [f32; 2] {
    let x = if rect.width() > 0.0 { (pos.x - rect.left()) / rect.width() } else { 0.0 };
    let y = if rect.height() > 0.0 { (pos.y - rect.top()) / rect.height() } else { 0.0 };
    [x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)]
}

/// Find where to insert a new point so it lands on the curve's nearest segment.
///
/// Returns `(insertion_index, distance, projected_point)` in the same space as
/// the inputs (screen pixels at the call site), or `None` for fewer than two
/// points. Considers the closing segment (last → first) when `closed`, whose
/// insertion index is `points.len()` (appended between the last and the wrap).
fn nearest_segment_insertion(
    points: &[[f32; 2]],
    closed: bool,
    query: [f32; 2],
) -> Option<(usize, f32, [f32; 2])> {
    let n = points.len();
    if n < 2 {
        return None;
    }
    let seg_count = if closed { n } else { n - 1 };
    let mut best: Option<(usize, f32, [f32; 2])> = None;
    for i in 0..seg_count {
        let a = points[i];
        let b = points[(i + 1) % n];
        let (d, proj) = project_point_segment(query, a, b);
        if best.map_or(true, |(_, bd, _)| d < bd) {
            best = Some((i + 1, d, proj));
        }
    }
    best
}

/// Distance from `p` to segment `a`–`b` and the projected point on it.
fn project_point_segment(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> (f32, [f32; 2]) {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-12 {
        let ex = p[0] - a[0];
        let ey = p[1] - a[1];
        return ((ex * ex + ey * ey).sqrt(), a);
    }
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len_sq).clamp(0.0, 1.0);
    let proj = [a[0] + t * dx, a[1] + t * dy];
    let ex = p[0] - proj[0];
    let ey = p[1] - proj[1];
    ((ex * ex + ey * ey).sqrt(), proj)
}

#[cfg(test)]
#[path = "curve_overlay_tests.rs"]
mod tests;
