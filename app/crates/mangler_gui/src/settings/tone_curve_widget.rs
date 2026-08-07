//! Embedded Photoshop-style tone-curve editor for the node settings panel.
//!
//! Renders a square editing box for a `Value::Curve` input marked with
//! `InputSettings::ToneCurve`: the source image's luminance histogram behind a
//! quarter grid and identity diagonal, with the curve drawn on top and its
//! control points draggable. Interactions match the 2D preview's curve
//! overlay: drag points to move them, click the box to insert a point, double-
//! or right-click a point to delete it (floor of 2 points), and drag a point's
//! mirrored tangent knob to shape the slope through it — including at the
//! endpoints, so the curve can leave/enter the box at any angle. The first
//! knob drag switches the curve from `Smooth` to `Bezier`; knobs stay
//! constrained so the curve remains a left-to-right function.
//!
//! Unlike the spatial overlay, points here are a *function* of x — dragging
//! keeps each point's x between its neighbours (Photoshop behaviour), so the
//! curve always reads left-to-right as input → output. Coordinates are the
//! curve's native y-down `[0,1]²`: the box's top edge is output 1.0, so no
//! flipping is needed when mapping to screen space. That difference is the
//! whole of [`KnobMode::Function`] and [`constrain_to_function`]; everything
//! else about the interaction is shared with the overlay via
//! [`crate::overlay::point_editor`].
//!
//! This is a pure widget — the caller applies [`ToneCurveResponse::changed`]
//! to its local input value every frame and pushes to the engine only when
//! `commit` is set (drag release, insert, delete), so heavy downstream nodes
//! re-run once per gesture rather than per frame.

use eframe::egui::{self, Pos2, Rect, Sense, Stroke, Vec2};
use epaint::StrokeKind;
use mangler_core::curve::Curve;

use crate::graph::graph_node::HistogramCache;
use crate::overlay::mapping::{norm_to_screen, screen_to_norm};
use crate::overlay::point_editor::{self, KnobMode, PointSetPolicy, PointSetStyle};
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

/// Minimum horizontal spacing kept between neighbouring points while dragging,
/// in curve units (~half a 8-bit step keeps near-vertical curves possible
/// without ever letting points cross).
const MIN_X_GAP: f32 = 0.002;
/// Maximum side length of the editing box, in screen pixels. Below this the
/// box fills the panel width; wide panels get a Photoshop-sized square.
const MAX_SIDE: f32 = 320.0;

/// How a value-mapping curve behaves: points stay x-ordered so the curve is a
/// function, tangents are box-clamped and offered in Smooth as well as Bezier,
/// and no first-point ring (there is no "start" of a tone curve to mark).
const POLICY: PointSetPolicy = PointSetPolicy {
    min_points: 2,
    knobs: KnobMode::Function,
    constrain: constrain_to_function,
    insert: insert_x_sorted,
    style: PointSetStyle {
        anchor_hit_half: 8.0,
        anchor_radius: 3.5,
        anchor_radius_active: 5.0,
        knob_hit_half: 6.0,
        knob_radius: 2.75,
        knob_radius_active: 4.0,
        first_point_ring: false,
    },
};

/// Draw the editor and return any change made this frame.
pub fn show(
    ui: &mut egui::Ui,
    curve: &Curve,
    histogram: Option<&HistogramCache>,
    theme: &Theme,
) -> ToneCurveResponse {
    let colors = theme.get();
    let mut working = curve.clone();

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

    // --- interactions ---
    // The box is both the coordinate space and the empty-space click target.
    // `ui.id()` is already unique per box: the settings panel wraps each in a
    // `push_id(("tone_curve_editor", input_index))` scope.
    let id = ui.id();
    let edit = point_editor::edit_point_set(ui, id, rect, rect, &mut working, &POLICY);

    // --- curve + points on top of everything ---
    draw_tone_curve(&painter, rect, &working, Stroke::new(2.0, colors.grid_connection_line));
    point_editor::draw_point_set(ui, rect, &working, &POLICY, &edit, theme);

    // Input → output readout while dragging, pinned to the top-left corner.
    if let Some(p) = edit.dragged_index.and_then(|i| working.points.get(i).copied()) {
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
        changed: edit.gesture.changed.then_some(working),
        commit: edit.gesture.commit,
    }
}

/// Keep a dragged point's x strictly between its neighbours (Photoshop rule), so
/// the curve remains a left-to-right function of the input value.
fn constrain_to_function(curve: &Curve, index: usize, mut p: [f32; 2]) -> [f32; 2] {
    let n = curve.points.len();
    if index > 0 {
        p[0] = p[0].max(curve.points[index - 1][0] + MIN_X_GAP);
    }
    if index + 1 < n {
        p[0] = p[0].min(curve.points[index + 1][0] - MIN_X_GAP);
    }
    p[0] = p[0].clamp(0.0, 1.0);
    p
}

/// Insert a clicked point at its x-sorted position, so the point list stays
/// ordered and the curve stays a function.
fn insert_x_sorted(working: &mut Curve, rect: Rect, click: Pos2) {
    let p = screen_to_norm(rect, click);
    let idx = working.points.iter().take_while(|q| q[0] < p[0]).count();
    let aligned = working.handles.len() == working.points.len();
    working.points.insert(idx, p);
    if aligned {
        // The new anchor gets the auto tangent its neighbours imply.
        working.handles.insert(idx, [0.0, 0.0]);
        working.handles[idx] = working.auto_handle(idx);
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

#[cfg(test)]
#[path = "tone_curve_widget_tests.rs"]
mod tests;
