//! Shared control-point editing for the curve editors.
//!
//! Both curve editors — the 2D preview's spatial overlay
//! (`view_window::curve_overlay`) and the settings panel's tone-curve box
//! (`settings::tone_curve_widget`) — are the same interaction: drag anchors,
//! click empty space to insert, double- or right-click an anchor to delete
//! (down to a floor), and drag mirrored bezier tangent knobs. They differ only
//! in a handful of rules, which [`PointSetPolicy`] carries.
//!
//! Interaction and painting are deliberately **two calls**: [`edit_point_set`]
//! mutates the working curve, then the caller draws the curve itself (each
//! editor draws a different path — a flattened polyline vs. a tone curve with
//! flat clamp extensions), then [`draw_point_set`] puts the handles on top. That
//! ordering means the handles and the path are always painted from the *current*
//! frame's values, so a drag tracks the pointer with no one-frame lag.
//!
//! See [`super::handle`] for the egui hit-test contract that makes handle drags
//! and canvas panning coexist, and [`super::gesture`] for the commit protocol.

use eframe::egui::{self, Pos2, Rect, Vec2};
use mangler_core::curve::{Curve, CurveInterpolation};

use super::gesture::Gesture;
use super::handle::{self, HandleShape};
use super::mapping::{norm_to_screen, screen_delta_to_norm, screen_to_norm};
use crate::themes::theme::Theme;

/// Whether and how bezier tangent knobs are offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnobMode {
    /// A **spatial path**: knobs appear in `Bezier` mode only, every anchor
    /// shows both tangent sides (a path can loop, so the ends are not special),
    /// and drags are unclamped because a tangent tip may legitimately sit off
    /// the canvas.
    Spatial,
    /// A **value-mapping function**: knobs appear whenever the curve is not
    /// `Linear`, endpoints show only their inward side so no knob dangles
    /// outside the box, drags clamp into the box and are constrained so the
    /// curve stays a left-to-right function, and the first drag promotes
    /// `Smooth` to `Bezier`.
    Function,
}

/// Handle sizes and decorations. Purely cosmetic — the two editors were tuned
/// independently (the tone-curve box is smaller, so its handles are too).
pub struct PointSetStyle {
    /// Half-width of an anchor's interaction rect, in screen pixels.
    pub anchor_hit_half: f32,
    pub anchor_radius: f32,
    pub anchor_radius_active: f32,
    /// Half-width of a knob's interaction rect. Keep this *smaller* than
    /// `anchor_hit_half`: knobs are registered first, so the anchor wins when a
    /// short tangent sits on top of it.
    pub knob_hit_half: f32,
    pub knob_radius: f32,
    pub knob_radius_active: f32,
    /// Draw a ring around point 0 so a path's start/end reads at a glance.
    pub first_point_ring: bool,
}

/// The rules that distinguish one curve editor from another.
pub struct PointSetPolicy {
    /// Deletion floor — a curve always keeps at least this many points.
    pub min_points: usize,
    pub knobs: KnobMode,
    /// Constrain a dragged anchor. See [`unconstrained`] for the spatial case.
    pub constrain: fn(&Curve, usize, [f32; 2]) -> [f32; 2],
    /// Apply a click that missed every handle, inserting or appending a point.
    /// Receives the coordinate rect so an implementation can measure in screen
    /// pixels (the spatial editor's "within N px of a segment" rule).
    pub insert: fn(&mut Curve, Rect, Pos2),
    pub style: PointSetStyle,
}

/// A `constrain` that lets an anchor go wherever it was dragged. Used by the
/// spatial path editor, where points have no ordering requirement.
pub fn unconstrained(_curve: &Curve, _index: usize, p: [f32; 2]) -> [f32; 2] {
    p
}

/// The outcome of one interaction pass, and the hover state needed to paint it.
pub struct PointSetEdit {
    pub gesture: Gesture,
    /// The anchor being dragged, for callers that show a readout.
    pub dragged_index: Option<usize>,
    /// Per-anchor active (hovered or dragged) flags, taken from the real egui
    /// responses so painting matches hit-testing exactly. Emptied when an
    /// insert or delete renumbered the points this frame.
    anchor_active: Vec<bool>,
    /// Per-anchor `[out, in]` knob active flags. Same aliasing rule.
    knob_active: Vec<[bool; 2]>,
}

/// Run one interaction pass over the curve's control points, mutating `working`.
///
/// `rect` is the coordinate space the curve's `[0,1]²` maps onto. `catch_rect`
/// is where a click counts as "empty space" — the whole panel for the 2D
/// overlay, just the box for the tone curve. `id` salts every widget id and must
/// be unique per editor instance (per panel, per input).
pub fn edit_point_set(
    ui: &mut egui::Ui,
    id: egui::Id,
    rect: Rect,
    catch_rect: Rect,
    working: &mut Curve,
    policy: &PointSetPolicy,
) -> PointSetEdit {
    let mut gesture = Gesture::IDLE;
    let mut dragged_index: Option<usize> = None;

    // Read the catcher first so the handles registered below win the click;
    // apply its result last so a click a handle took is correctly excluded.
    let catcher = handle::catcher(ui, id.with("catcher"), catch_rect);

    // Tangent knobs, registered before the anchors so an anchor wins when a
    // near-zero handle sits on top of it.
    let mut knob_active: Vec<[bool; 2]> = Vec::new();
    if knobs_visible(policy.knobs, working.interpolation) {
        // Materialize silently: auto tangents become concrete so a drag can
        // write `handles[i]`. Not an edit until a gesture actually moves one.
        working.materialize_handles();
        let n = working.points.len();
        knob_active = vec![[false; 2]; n];
        for i in 0..n {
            let anchor = norm_to_screen(rect, working.points[i]);
            let sides = knob_sides(policy.knobs, i, n, working.closed);
            for (side_idx, sign) in [(0usize, 1.0f32), (1, -1.0)] {
                if !sides[side_idx] {
                    continue;
                }
                let pos = knob_pos(rect, anchor, working.handles[i], sign);
                let resp =
                    handle::knob(ui, id.with(("knob", i, side_idx)), pos, policy.style.knob_hit_half);
                knob_active[i][side_idx] = resp.active;

                if let Some(to) = resp.drag_to {
                    // Compute against an immutable borrow first, then write.
                    let tangent =
                        tangent_from_drag(rect, anchor, to, sign, policy.knobs, &working.points, i);
                    working.handles[i] = tangent;
                    if policy.knobs == KnobMode::Function {
                        // Shaping a tangent is what promotes Smooth to Bezier.
                        working.interpolation = CurveInterpolation::Bezier;
                    }
                    gesture.merge(Gesture::dragging());
                }
                if resp.commit {
                    gesture.merge(Gesture::released());
                }
            }
        }
    }

    // Anchors. Deletion renumbers indices, so defer it past the loop.
    let n = working.points.len();
    let mut anchor_active = vec![false; n];
    let mut delete_index: Option<usize> = None;
    for i in 0..n {
        let center = norm_to_screen(rect, working.points[i]);
        let resp = handle::handle(ui, id.with(("pt", i)), center, policy.style.anchor_hit_half);
        anchor_active[i] = resp.active;

        if let Some(to) = resp.drag_to {
            let p = (policy.constrain)(working, i, screen_to_norm(rect, to));
            working.points[i] = p;
            dragged_index = Some(i);
            gesture.merge(Gesture::dragging());
        }
        if resp.commit {
            gesture.merge(Gesture::released());
        }
        if resp.delete && n > policy.min_points {
            delete_index = Some(i);
        }
    }

    let mut indices_moved = false;
    if let Some(idx) = delete_index {
        // Keep handles index-aligned with points (only when they already are —
        // a stale mismatched vec is left for `materialize_handles` to rebuild).
        if working.handles.len() == working.points.len() {
            working.handles.remove(idx);
        }
        working.points.remove(idx);
        gesture.merge(Gesture::edited());
        indices_moved = true;
    }

    if let Some(pos) = catcher.clicked_at {
        (policy.insert)(working, rect, pos);
        gesture.merge(Gesture::edited());
        indices_moved = true;
    }

    if indices_moved {
        // The cached flags are keyed on the pre-edit indexing. Drop them so
        // `draw_point_set` falls back to plain pointer geometry for one frame —
        // invisible next to the shape change that just happened.
        anchor_active.clear();
        knob_active.clear();
        dragged_index = None;
    }

    PointSetEdit { gesture, dragged_index, anchor_active, knob_active }
}

/// Paint the tangent knobs and anchors for a curve, on top of whatever path the
/// caller drew. Pass the [`PointSetEdit`] from this frame's [`edit_point_set`].
pub fn draw_point_set(
    ui: &egui::Ui,
    rect: Rect,
    curve: &Curve,
    policy: &PointSetPolicy,
    edit: &PointSetEdit,
    theme: &Theme,
) {
    // Painted through the *unclipped* painter so a point sitting exactly on the
    // canvas edge isn't half-clipped.
    let painter = ui.painter();
    let style = &policy.style;
    let n = curve.points.len();

    // Knobs under the anchors, so an overlapping anchor stays legible.
    if knobs_visible(policy.knobs, curve.interpolation) {
        for i in 0..n {
            let anchor = norm_to_screen(rect, curve.points[i]);
            let h = curve.handles.get(i).copied().unwrap_or([0.0, 0.0]);
            let sides = knob_sides(policy.knobs, i, n, curve.closed);
            for (side_idx, sign) in [(0usize, 1.0f32), (1, -1.0)] {
                if !sides[side_idx] {
                    continue;
                }
                let pos = knob_pos(rect, anchor, h, sign);
                let active = edit.knob_active.get(i).map_or_else(
                    || ui.rect_contains_pointer(handle::hit_rect(pos, style.knob_hit_half)),
                    |flags| flags[side_idx],
                );
                let radius = if active { style.knob_radius_active } else { style.knob_radius };
                handle::draw_knob(painter, anchor, pos, radius, theme);
            }
        }
    }

    for i in 0..n {
        let center = norm_to_screen(rect, curve.points[i]);
        let active = edit.anchor_active.get(i).copied().unwrap_or_else(|| {
            ui.rect_contains_pointer(handle::hit_rect(center, style.anchor_hit_half))
        }) || edit.dragged_index == Some(i);
        let radius = if active { style.anchor_radius_active } else { style.anchor_radius };
        handle::draw_handle(painter, center, radius, active, HandleShape::Dot, theme);
        if style.first_point_ring && i == 0 {
            handle::draw_handle_ring(painter, center, radius + 3.0, theme);
        }
    }
}

/// Whether tangent knobs are shown for this interpolation under this mode.
pub fn knobs_visible(mode: KnobMode, interp: CurveInterpolation) -> bool {
    match mode {
        KnobMode::Spatial => interp == CurveInterpolation::Bezier,
        KnobMode::Function => interp != CurveInterpolation::Linear,
    }
}

/// Which of anchor `index`'s two tangent sides are shown, as `[out, in]`.
///
/// A function curve hides an endpoint's outward side so no knob dangles outside
/// the box; a spatial path shows both on every anchor.
pub fn knob_sides(mode: KnobMode, index: usize, n: usize, closed: bool) -> [bool; 2] {
    match mode {
        KnobMode::Function => [closed || index + 1 < n, closed || index > 0],
        _ => [true, true],
    }
}

/// Screen position of one mirrored tangent knob. `sign` is `+1` for the
/// out-knob (`anchor + h`) and `-1` for the in-knob (`anchor - h`), which is
/// what makes the pair point-reflected and the anchor C¹ by construction.
fn knob_pos(rect: Rect, anchor: Pos2, h: [f32; 2], sign: f32) -> Pos2 {
    anchor + Vec2::new(h[0] * rect.width(), h[1] * rect.height()) * sign
}

/// Convert a knob drag to anchor `index`'s mirrored tangent offset.
fn tangent_from_drag(
    rect: Rect,
    anchor: Pos2,
    to: Pos2,
    sign: f32,
    mode: KnobMode,
    points: &[[f32; 2]],
    index: usize,
) -> [f32; 2] {
    // A function curve keeps the knob inside the box so it can't overrun the
    // panel. A steep slope is made with a *short* handle, not a long one, so
    // this doesn't limit the reachable angle. Spatial paths stay unclamped —
    // their tangent tips may legitimately leave the canvas.
    let to = if mode == KnobMode::Function {
        Pos2::new(to.x.clamp(rect.left(), rect.right()), to.y.clamp(rect.top(), rect.bottom()))
    } else {
        to
    };

    let d = screen_delta_to_norm(rect, to - anchor);
    let mut h = [d[0] * sign, d[1] * sign];

    if mode == KnobMode::Function {
        // Function guard: the tangent points right (`h.x >= 0`) and neither
        // mirrored control passes a neighbouring anchor, so the spline stays a
        // left-to-right function of the input value.
        let n = points.len();
        let right =
            if index + 1 < n { points[index + 1][0] - points[index][0] } else { f32::INFINITY };
        let left = if index > 0 { points[index][0] - points[index - 1][0] } else { f32::INFINITY };
        h[0] = h[0].clamp(0.0, right.min(left));
    }
    h
}

#[cfg(test)]
#[path = "point_editor_tests.rs"]
mod tests;
