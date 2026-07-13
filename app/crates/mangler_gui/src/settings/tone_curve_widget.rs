//! Embedded Photoshop-style tone-curve editor for the node settings panel.
//!
//! Renders a square editing box for a `Value::Curve` input marked with
//! `InputSettings::ToneCurve`: the source image's luminance histogram behind a
//! quarter grid and identity diagonal, with the curve drawn on top and its
//! control points draggable. Interactions match the 2D preview's curve
//! overlay: drag points to move them, click the box to insert a point, double-
//! or right-click a point to delete it (floor of 2 points).
//!
//! Unlike the spatial overlay, points here are a *function* of x — dragging
//! keeps each point's x between its neighbours (Photoshop behaviour), so the
//! curve always reads left-to-right as input → output. Coordinates are the
//! curve's native y-down `[0,1]²`: the box's top edge is output 1.0, so no
//! flipping is needed when mapping to screen space.
//!
//! This is a pure widget — the caller applies [`ToneCurveResponse::changed`]
//! to its local input value every frame and pushes to the engine only when
//! `commit` is set (drag release, insert, delete), so heavy downstream nodes
//! re-run once per gesture rather than per frame.

use eframe::egui::{self, Pos2, Rect, Sense, Stroke, Vec2};
use epaint::StrokeKind;
use mangler_core::curve::Curve;

use crate::graph::graph_node::HistogramCache;
use crate::themes::theme::Theme;

/// The result of one editor frame.
pub struct ToneCurveResponse {
    /// The mutated curve, when a gesture changed it this frame. The caller
    /// mirrors this into its local input value for instant visual feedback.
    pub changed: Option<Curve>,
    /// Whether the gesture *completed* (drag release, insert, or delete) and
    /// the change should be pushed to the engine. A drag's release frame sets
    /// this with `changed: None` — the caller pushes its accumulated value.
    pub commit: bool,
}

/// Half-width of a control point's interaction rect, in screen pixels.
const POINT_HIT_HALF: f32 = 8.0;
/// Minimum horizontal spacing kept between neighbouring points while dragging,
/// in curve units (~half a 8-bit step keeps near-vertical curves possible
/// without ever letting points cross).
const MIN_X_GAP: f32 = 0.002;
/// Maximum side length of the editing box, in screen pixels. Below this the
/// box fills the panel width; wide panels get a Photoshop-sized square.
const MAX_SIDE: f32 = 320.0;

/// Draw the editor and return any change made this frame.
pub fn show(
    ui: &mut egui::Ui,
    curve: &Curve,
    histogram: Option<&HistogramCache>,
    theme: &Theme,
) -> ToneCurveResponse {
    let colors = theme.get();
    let mut working = curve.clone();
    let mut changed = false;
    let mut commit = false;

    // Square box, sized to the panel but capped at Photoshop-ish dimensions.
    let side = ui.available_width().min(MAX_SIDE).max(80.0);
    let (rect, _bg) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return ToneCurveResponse { changed: None, commit: false };
    }

    // --- static chrome: background, histogram, grid, identity diagonal ---
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, 2.0, colors.histogram_bg);

    // Luminance histogram of the source image, drawn faint behind the grid so
    // the curve can be read against the tonal distribution (like Photoshop).
    if let Some(cache) = histogram {
        let bar_w = rect.width() / 256.0;
        for (i, &count) in cache.bins.iter().enumerate() {
            if count == 0 {
                continue;
            }
            let h = count as f32 / cache.max_count as f32 * rect.height();
            let x = rect.left() + i as f32 * bar_w;
            painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(x, rect.bottom() - h),
                    // 1px overlap prevents sub-pixel gaps between bars.
                    Pos2::new(x + bar_w + 1.0, rect.bottom()),
                ),
                0.0,
                colors.histogram_luminance,
            );
        }
    }

    // Quarter grid + identity diagonal, both quiet so the curve stays dominant.
    let grid_stroke = Stroke::new(1.0, colors.text_faint.gamma_multiply(0.25));
    for i in 1..4 {
        let f = i as f32 / 4.0;
        let x = rect.left() + f * rect.width();
        let y = rect.top() + f * rect.height();
        painter.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())], grid_stroke);
        painter.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)], grid_stroke);
    }
    painter.line_segment(
        [rect.left_bottom(), rect.right_top()],
        Stroke::new(1.0, colors.text_faint.gamma_multiply(0.4)),
    );

    // --- interactions (mirrors curve_overlay.rs) ---
    // Empty-space click catcher, registered before the point handles so a
    // click that lands on a handle goes to the handle (egui resolves a single
    // click winner; the topmost — the handles below — wins).
    let catcher = ui.interact(rect, ui.id().with("tone_curve_catcher"), Sense::click());

    // Point handles. Deletion changes indices, so defer it past the loop.
    let mut delete_index: Option<usize> = None;
    let mut dragged_index: Option<usize> = None;
    let n = working.points.len();
    for i in 0..n {
        let center = norm_to_screen(rect, working.points[i]);
        let hit = Rect::from_center_size(center, Vec2::splat(POINT_HIT_HALF * 2.0));
        let resp = ui.interact(hit, ui.id().with(("tone_curve_pt", i)), Sense::click_and_drag());

        if resp.dragged() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let mut p = screen_to_norm(rect, pos);
                // Photoshop rule: a point can't cross its neighbours, so the
                // curve remains a left-to-right function of the input value.
                if i > 0 {
                    p[0] = p[0].max(working.points[i - 1][0] + MIN_X_GAP);
                }
                if i + 1 < n {
                    p[0] = p[0].min(working.points[i + 1][0] - MIN_X_GAP);
                }
                p[0] = p[0].clamp(0.0, 1.0);
                working.points[i] = p;
                dragged_index = Some(i);
                changed = true;
            }
        }
        if resp.drag_stopped() {
            commit = true;
        }
        // Double- or right-click removes the point, with a floor of 2.
        if (resp.double_clicked() || resp.clicked_by(egui::PointerButton::Secondary)) && n > 2 {
            delete_index = Some(i);
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

    // Insert on a click that missed every handle: the new point lands exactly
    // where clicked, at its x-sorted position so the function stays ordered.
    if catcher.clicked() {
        if let Some(pos) = catcher.interact_pointer_pos() {
            let p = screen_to_norm(rect, pos);
            let idx = working.points.iter().take_while(|q| q[0] < p[0]).count();
            let aligned = working.handles.len() == working.points.len();
            working.points.insert(idx, p);
            if aligned {
                // The new anchor gets the auto tangent its neighbours imply.
                working.handles.insert(idx, [0.0, 0.0]);
                working.handles[idx] = working.auto_handle(idx);
            }
            changed = true;
            commit = true;
        }
    }

    // --- curve + points on top of everything ---
    draw_tone_curve(&painter, rect, &working, Stroke::new(2.0, colors.grid_connection_line));

    for (i, p) in working.points.iter().enumerate() {
        let center = norm_to_screen(rect, *p);
        // Re-derive hover from the handle rect: the interact responses above
        // are consumed, and hover alone needs no second interact.
        let hovered = ui
            .rect_contains_pointer(Rect::from_center_size(center, Vec2::splat(POINT_HIT_HALF * 2.0)));
        let active = hovered || dragged_index == Some(i);
        let radius = if active { 5.0 } else { 3.5 };
        let fill = if active { colors.grid_connection_dot_hover } else { colors.grid_connection_dot };
        // Painted through the *unclipped* painter so a point sitting exactly
        // on the box edge isn't half-clipped.
        ui.painter().circle(center, radius, fill, Stroke::new(1.5, colors.node_header_selected_border));
    }

    // Input → output readout while dragging, pinned to the top-left corner.
    if let Some(p) = dragged_index.and_then(|i| working.points.get(i).copied()) {
        painter.text(
            rect.left_top() + Vec2::new(6.0, 4.0),
            egui::Align2::LEFT_TOP,
            format!("{:.2} → {:.2}", p[0], 1.0 - p[1]),
            egui::TextStyle::Small.resolve(ui.style()),
            colors.text_faint,
        );
    }

    // Border on top of the content.
    ui.painter().rect_stroke(
        rect,
        2.0,
        Stroke::new(1.0, colors.text_faint.gamma_multiply(0.5)),
        StrokeKind::Inside,
    );

    ToneCurveResponse {
        changed: changed.then_some(working),
        commit,
    }
}

/// Draw the tone curve into `rect`: flat extensions to the box edges left of
/// the first / right of the last point (the LUT clamps there), then the
/// flattened spline with display y clamped to the box.
fn draw_tone_curve(painter: &egui::Painter, rect: Rect, curve: &Curve, stroke: Stroke) {
    // 48 samples/span matches the core LUT rasterization tolerance.
    let poly = curve.flatten(48);
    if poly.len() < 2 {
        return;
    }
    let clamp_y = |p: &[f32; 2]| [p[0], p[1].clamp(0.0, 1.0)];

    // Flat clamp extensions.
    let first = clamp_y(&poly[0]);
    let last = clamp_y(poly.last().unwrap());
    if first[0] > 0.0 {
        painter.line_segment(
            [norm_to_screen(rect, [0.0, first[1]]), norm_to_screen(rect, first)],
            stroke,
        );
    }
    if last[0] < 1.0 {
        painter.line_segment(
            [norm_to_screen(rect, last), norm_to_screen(rect, [1.0, last[1]])],
            stroke,
        );
    }

    let pts: Vec<Pos2> = poly.iter().map(|p| norm_to_screen(rect, clamp_y(p))).collect();
    painter.add(egui::Shape::line(pts, stroke));
}

/// Map a normalized y-down `[0,1]²` curve point to a screen position in `rect`.
fn norm_to_screen(rect: Rect, p: [f32; 2]) -> Pos2 {
    Pos2::new(rect.left() + p[0] * rect.width(), rect.top() + p[1] * rect.height())
}

/// Map a screen position to a normalized y-down `[0,1]²` curve point, clamped
/// to the unit square (points can't leave the box).
fn screen_to_norm(rect: Rect, pos: Pos2) -> [f32; 2] {
    let x = if rect.width() > 0.0 { (pos.x - rect.left()) / rect.width() } else { 0.0 };
    let y = if rect.height() > 0.0 { (pos.y - rect.top()) / rect.height() } else { 0.0 };
    [x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)]
}
