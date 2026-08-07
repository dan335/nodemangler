//! Interactive curve-editing overlay for the 2D preview panel.
//!
//! Renders a [`Curve`] on top of the displayed image (or a fallback canvas) and
//! lets the user draw it directly: drag control points, click empty space to
//! insert/append a point, double- or right-click a point to delete it, drag
//! the mirrored tangent knobs in Bezier mode to shape curvature, and toggle
//! closed/interpolation from a small strip. This is a *pure widget* — it has no
//! engine knowledge and holds no persistent state (egui tracks drags per widget
//! id; ids are salted with the panel's `leaf_id` so two 2D panels don't
//! collide). The caller applies [`CurveOverlayResponse::changed`] to its local
//! value every frame and pushes it to the engine only when `commit` is set.
//!
//! The interaction itself lives in [`crate::overlay::point_editor`], shared with
//! the settings panel's tone-curve box; this module supplies the *spatial*
//! policy (points may go anywhere and loop, tangent tips may leave the canvas)
//! and the path rendering. See [`crate::overlay::handle`] for the egui
//! hit-testing contract that keeps handle drags from stealing the canvas pan.

use eframe::egui::{self, Pos2, Rect, Stroke, Vec2};
use mangler_core::curve::{Curve, CurveInterpolation};

use crate::overlay::mapping::{norm_to_screen, screen_to_norm};
use crate::overlay::point_editor::{
    self, KnobMode, PointSetPolicy, PointSetStyle,
};
use crate::overlay::Gesture;
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

/// How a spatial path behaves: points go wherever they are dragged (a path has
/// no ordering requirement), tangents are offered in Bezier mode on both sides
/// of every anchor, and a two-point floor keeps the curve a curve.
const POLICY: PointSetPolicy = PointSetPolicy {
    min_points: 2,
    knobs: KnobMode::Spatial,
    constrain: point_editor::unconstrained,
    insert: handle_insert,
    style: PointSetStyle {
        anchor_hit_half: 8.0,
        anchor_radius: 4.0,
        anchor_radius_active: 6.0,
        knob_hit_half: 6.0,
        knob_radius: 3.0,
        knob_radius_active: 4.5,
        first_point_ring: true,
    },
};

/// Draw the interactive overlay and return any change made this frame.
///
/// `view_rect` is the whole panel (where a click counts as empty space and
/// where the controls strip is anchored); `image_rect` is the `[0,1]²` mapping
/// target — the displayed image, or a fallback canvas when there is none.
pub fn show(
    ui: &mut egui::Ui,
    leaf_id: LeafId,
    view_rect: Rect,
    image_rect: Rect,
    curve: &Curve,
    theme: &Theme,
) -> CurveOverlayResponse {
    let mut working = curve.clone();

    let edit = point_editor::edit_point_set(
        ui,
        egui::Id::new(("curve_overlay", leaf_id)),
        image_rect,
        view_rect,
        &mut working,
        &POLICY,
    );
    let mut gesture = edit.gesture;

    // Path first, handles on top — both from this frame's values, so a drag
    // tracks the pointer without a frame of lag.
    draw_polyline(
        ui.painter(),
        image_rect,
        &working,
        Stroke::new(2.0, theme.get().grid_connection_line),
    );
    point_editor::draw_point_set(ui, image_rect, &working, &POLICY, &edit, theme);

    // Controls strip pinned to the panel's top-left corner. Registered after
    // the handles so its widgets win clicks where they overlap.
    if show_controls(ui, leaf_id, view_rect, &mut working, theme) {
        gesture.merge(Gesture::edited());
    }

    CurveOverlayResponse {
        changed: gesture.changed.then_some(working),
        commit: gesture.commit,
    }
}

/// Insert a new control point where the user clicked: on the nearest segment if
/// the click is within [`INSERT_THRESHOLD_PX`], otherwise appended to the end.
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
    crate::overlay::strip::top_left(ui, view_rect, Vec2::new(280.0, 26.0), theme, |ui| {
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

/// Display name for an interpolation kind (matches the settings-panel summary).
fn interp_name(interp: CurveInterpolation) -> &'static str {
    match interp {
        CurveInterpolation::Linear => "linear",
        CurveInterpolation::Smooth => "smooth",
        CurveInterpolation::Bezier => "bezier",
    }
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
