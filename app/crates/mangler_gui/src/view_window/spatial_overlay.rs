//! Interactive spatial gizmos for the 2D preview panel.
//!
//! Draws an operation's spatial inputs directly on the image and lets the user
//! drag them: `sample pixel` gets a crosshair you can click anywhere to move,
//! `crop` gets a box with eight resize grips. Which inputs are spatial, and what
//! their numbers mean, is declared in `mangler_core::gizmo` — this module knows
//! how to draw and grab them, never which slider is which.
//!
//! A *pure widget*: no engine knowledge, no persistent state. Which handle is
//! being dragged is encoded entirely in which egui id reported the drag, which
//! is what lets a nine-region box work without a drag-start snapshot. Ids are
//! salted with the panel's `leaf_id`, so every open 2D panel draws its own copy
//! and drags never cross-talk. The caller mirrors
//! [`SpatialOverlayResponse::changed`] into its local values every frame and
//! pushes to the engine when `commit` is set. Point and rect gizmos both
//! commit every drag frame so crop / sample pixel track the overlay live.
//!
//! ## Core invariant: draw the round-tripped value
//! Every drag pushes its candidate through the input's clamp range and its value
//! type before drawing it. The handle therefore visibly sticks at a slider limit
//! and snaps per pixel on an integer input, and what you see is exactly what
//! gets committed — there is no path where the box draws one region and the
//! operation crops another.
//!
//! See [`crate::overlay::handle`] for the egui hit-test contract that keeps
//! handle drags from stealing the canvas pan.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use mangler_core::gizmo::{Gizmo, GizmoSpec, RadiusSpace, RectExtent, SpatialSpace};
use mangler_core::input::{Input, InputSettings};
use mangler_core::value::Value;
use std::collections::HashMap;

use crate::overlay::handle::{self, HandleShape};
use crate::overlay::mapping::{norm_to_screen, screen_delta_to_norm, screen_to_norm};
use crate::panels::panel_tree::LeafId;
use crate::themes::theme::Theme;

/// Half-width of a resize grip's interaction rect, in screen pixels.
const HANDLE_HIT_HALF: f32 = 8.0;
/// Half-thickness of an edge band's interaction rect. Smaller than a grip's and
/// registered earlier, so a corner wins where the two overlap.
const EDGE_HIT_HALF: f32 = 5.0;
/// Half-side of a painted resize grip.
const GRIP_RADIUS: f32 = 3.5;
const GRIP_RADIUS_ACTIVE: f32 = 5.0;
/// Smallest on-screen box, in pixels. Below this the eight grips overlap into an
/// unusable knot, so the box refuses to shrink further however far it is zoomed
/// out.
const MIN_RECT_PX: f32 = 8.0;
/// Absolute floor on a box's normalized size, independent of zoom or image size.
const MIN_RECT_NORM: f32 = 0.001;
/// Below this on-screen size the whole gizmo is drawn as a plain outline with no
/// interaction — the handles would be indistinguishable anyway.
const MIN_INTERACTIVE_PX: f32 = MIN_RECT_PX * 3.0;
/// Screen gap between a placement box's top edge and its rotation knob. Fixed
/// in screen pixels, so the knob stays grabbable however far you zoom out.
const ROTATE_KNOB_GAP: f32 = 26.0;
/// Rotation snap, in degrees, while Shift is held.
const ROTATE_SNAP_DEG: f32 = 15.0;

/// The result of one overlay frame.
#[derive(Default)]
pub struct SpatialOverlayResponse {
    /// Inputs mutated this frame, as `(input_index, new_value)`. The caller
    /// mirrors each into its local node for instant feedback. **Empty on a
    /// drag's release frame** — the pointer no longer moved.
    pub changed: Vec<(usize, Value)>,
    /// Push the listed inputs to the engine this frame.
    ///
    /// Set on every drag frame (point and rect) so sample pixel and crop track
    /// the overlay live, not only on mouse-up. Also set on release so a
    /// zero-motion drag end still commits.
    pub commit: bool,
    /// Which inputs to push when `commit` is set. For a rect, derived from the
    /// handle that moved (dragging the right edge reaches `width` but never
    /// `y`). The caller sends one `SetInput` per index, reading its
    /// *accumulated local value* rather than `changed`.
    pub commit_inputs: Vec<usize>,
}

/// Everything the widget needs about the node it edits, gathered by the caller
/// so the widget holds no borrow on the graph.
pub struct GizmoContext<'a> {
    pub specs: &'static [GizmoSpec],
    /// The node's inputs, read-only: values, `settings` (for clamping) and
    /// `connection` (for the read-only rule).
    pub inputs: &'a [Input],
    /// Pixel size of the backdrop, when one is displayed.
    pub image_dims: Option<(u32, u32)>,
    /// Pixel size of each *connected* image input, keyed by input index.
    ///
    /// For [`Gizmo::Placement`], which draws a second image inside the
    /// backdrop's space: the box's natural size is the foreground's own pixel
    /// size, which no number on the node records. Resolved upstream by the
    /// caller (the same walk the backdrop uses) so the widget holds no borrow
    /// on the graph.
    pub image_input_dims: &'a HashMap<usize, (u32, u32)>,
}

/// Draw every gizmo the node declares and return any change made this frame.
///
/// `view_rect` is the whole panel; `image_rect` is the `[0,1]²` mapping target
/// (the displayed image, or a fallback canvas).
pub fn show(
    ui: &mut egui::Ui,
    leaf_id: LeafId,
    view_rect: Rect,
    image_rect: Rect,
    ctx: &GizmoContext<'_>,
    theme: &Theme,
) -> SpatialOverlayResponse {
    let mut out = SpatialOverlayResponse::default();
    let _ = view_rect;

    // A degenerate or non-finite mapping rect would put every handle on top of
    // every other one; draw nothing rather than something un-grabbable.
    if !image_rect.is_finite() || image_rect.width() <= 0.0 || image_rect.height() <= 0.0 {
        return out;
    }
    let interactive = image_rect.width() >= MIN_INTERACTIVE_PX
        && image_rect.height() >= MIN_INTERACTIVE_PX;

    for (spec_index, spec) in ctx.specs.iter().enumerate() {
        let drag_indices = spec.kind.inputs();
        // Defensive: a graph saved before the op gained an input can present a
        // shorter slice than the table expects. Skip rather than panic.
        // `referenced_inputs` includes display-only indices (e.g. sample diameter).
        if spec.kind.referenced_inputs().iter().any(|&i| i >= ctx.inputs.len()) {
            continue;
        }
        // All-or-nothing on *drag* inputs only: a connected diameter must not
        // freeze the crosshair.
        let editable =
            interactive && drag_indices.iter().all(|&i| ctx.inputs[i].connection.is_none());

        let id = egui::Id::new(("spatial_overlay", leaf_id, spec_index));
        match spec.kind {
            Gizmo::Point {
                x,
                y,
                diameter,
                radius,
                space,
            } => show_point(
                ui,
                id,
                image_rect,
                ctx,
                [x, y],
                diameter,
                radius,
                space,
                editable,
                theme,
                &mut out,
            ),
            Gizmo::Rect {
                x,
                y,
                w,
                h,
                space,
                extent,
                aspect,
            } => show_rect(
                ui,
                id,
                image_rect,
                ctx,
                [x, y, w, h],
                aspect,
                space,
                extent,
                editable,
                theme,
                &mut out,
            ), // space reserved for future basis-aware rects
            Gizmo::Line {
                angle,
                position,
                space,
            } => show_line(ui, id, image_rect, ctx, angle, position, space, editable, theme, &mut out),
            Gizmo::Axes { x, y, space } => {
                show_axes(ui, id, image_rect, ctx, x, y, space, editable, theme, &mut out)
            }
            Gizmo::QuadCorners { corners } => {
                show_quad(ui, id, image_rect, ctx, corners, editable, theme, &mut out)
            }
            Gizmo::Transform {
                offset_x,
                offset_y,
                rotation,
                scale_x,
                scale_y,
            } => show_transform(
                ui,
                id,
                image_rect,
                ctx,
                offset_x,
                offset_y,
                rotation,
                scale_x,
                scale_y,
                editable,
                theme,
                &mut out,
            ),
            Gizmo::OffsetPx { x, y } => {
                show_offset_px(ui, id, image_rect, ctx, x, y, editable, theme, &mut out)
            }
            Gizmo::CenterRadius { radius, space } => {
                show_center_radius(ui, id, image_rect, ctx, radius, space, editable, theme, &mut out)
            }
            Gizmo::Placement { image, x, y, scale_x, scale_y, rotation } => show_placement(
                ui,
                id,
                image_rect,
                ctx,
                image,
                [x, y, scale_x, scale_y, rotation],
                editable,
                theme,
                &mut out,
            ),
        }
    }
    out
}

// ---------------------------------------------------------------- point gizmo

/// A draggable crosshair (+ optional radius rim / pixel-diameter disk).
#[allow(clippy::too_many_arguments)]
fn show_point(
    ui: &mut egui::Ui,
    id: egui::Id,
    image_rect: Rect,
    ctx: &GizmoContext<'_>,
    idx: [usize; 2],
    diameter_idx: Option<usize>,
    radius: Option<(usize, RadiusSpace)>,
    space: SpatialSpace,
    editable: bool,
    theme: &Theme,
    out: &mut SpatialOverlayResponse,
) {
    let Some(mut values) = read_pair(ctx.inputs, idx) else { return };
    // Diameter is re-read from the live input every frame so the settings-panel
    // slider (and a diameter-rim drag below) both update the disk immediately.
    let mut diameter = diameter_idx.and_then(|i| read_scalar(&ctx.inputs[i].value));
    let mut radius_val = radius.and_then(|(i, _)| read_scalar(&ctx.inputs[i].value));
    let radius_space = radius.map(|(_, s)| s);

    let center = norm_to_screen(image_rect, space.to_unit(ctx.image_dims, values));
    let mut screen_r = ring_screen_radius(diameter, radius_val, radius_space, image_rect, ctx.image_dims);

    if editable {
        // Rims first (registered before centre so centre wins on overlap at
        // small radii). Catcher last so empty-space click repositions.
        let mut rim_active = false;

        // Pixel-diameter rim (sample pixel): display-only for the editable
        // rule, but draggable here so the disk can be resized on-image. Must
        // not join `drag_indices` — a connected diameter still freezes only
        // this rim, never the crosshair.
        if let (Some(d_idx), Some(d)) = (diameter_idx, diameter) {
            if diameter_idx_is_editable(ctx, d_idx) {
                if let Some(sr) = screen_r.filter(|r| *r > HANDLE_HIT_HALF) {
                    if let Some(dims) = ctx.image_dims {
                        let rim_pos = center + Vec2::new(sr, 0.0);
                        let rim = handle::handle(ui, id.with("diam"), rim_pos, HANDLE_HIT_HALF);
                        rim_active = rim.active;
                        if let Some(to) = rim.drag_to {
                            let dist = (to - center).length();
                            let next = quantize(
                                &ctx.inputs[d_idx],
                                screen_dist_to_pixel_diameter(dist, image_rect, dims),
                            );
                            if (next - d).abs() > 1e-6 {
                                diameter = Some(next);
                                screen_r = ring_screen_radius(
                                    diameter,
                                    radius_val,
                                    radius_space,
                                    image_rect,
                                    ctx.image_dims,
                                );
                                push_live(out, d_idx, &ctx.inputs[d_idx], next);
                            }
                        }
                        if rim.commit {
                            out.commit = true;
                            out.commit_inputs.push(d_idx);
                        }
                    }
                }
            }
        }

        if let (Some((r_idx, r_space)), Some(rv)) = (radius, radius_val) {
            if let Some(sr) = screen_r.filter(|r| *r > HANDLE_HIT_HALF) {
                let rim_pos = center + Vec2::new(sr, 0.0);
                let rim = handle::handle(ui, id.with("rim"), rim_pos, HANDLE_HIT_HALF);
                rim_active = rim_active || rim.active;
                if let Some(to) = rim.drag_to {
                    if let Some(dims) = ctx.image_dims {
                        let dist = (to - center).length();
                        let next = quantize(
                            &ctx.inputs[r_idx],
                            r_space.from_screen_radius(
                                dist,
                                [image_rect.width(), image_rect.height()],
                                dims,
                            ),
                        );
                        if (next - rv).abs() > 1e-6 {
                            radius_val = Some(next);
                            screen_r = ring_screen_radius(
                                diameter,
                                radius_val,
                                radius_space,
                                image_rect,
                                ctx.image_dims,
                            );
                            push_live(out, r_idx, &ctx.inputs[r_idx], next);
                        }
                    }
                }
                if rim.commit {
                    out.commit = true;
                    out.commit_inputs.push(r_idx);
                }
            }
        }

        let catcher = handle::catcher(ui, id.with("catch"), image_rect);
        let grab = handle::handle(ui, id.with("pt"), center, HANDLE_HIT_HALF);

        let target = grab.drag_to.or(catcher.clicked_at);
        if let Some(to) = target {
            let unit = screen_to_norm(image_rect, to);
            let next = quantize_pair(ctx.inputs, idx, space.from_unit(ctx.image_dims, unit));
            if next != values {
                values = next;
                push_live(out, idx[0], &ctx.inputs[idx[0]], values[0]);
                push_live(out, idx[1], &ctx.inputs[idx[1]], values[1]);
            }
        }
        if grab.commit || catcher.clicked_at.is_some() {
            out.commit = true;
            out.commit_inputs.extend_from_slice(&idx);
        }

        draw_crosshair(
            ui,
            image_rect,
            space.to_unit(ctx.image_dims, values),
            screen_r,
            diameter,
            grab.active || rim_active,
            true,
            None,
            theme,
        );
    } else {
        let note = driven_note(ctx, &idx);
        draw_crosshair(
            ui,
            image_rect,
            space.to_unit(ctx.image_dims, values),
            screen_r,
            diameter,
            false,
            false,
            note,
            theme,
        );
    }
}

/// True when a display-only diameter input can be edited from the rim handle.
fn diameter_idx_is_editable(ctx: &GizmoContext<'_>, idx: usize) -> bool {
    ctx.inputs.get(idx).is_some_and(|i| i.connection.is_none())
}

/// Convert a screen-pixel distance from the crosshair to a source-pixel diameter.
fn screen_dist_to_pixel_diameter(dist: f32, image_rect: Rect, dims: (u32, u32)) -> f32 {
    let (iw, ih) = dims;
    if iw == 0 || ih == 0 {
        return 1.0;
    }
    // Inverse of `ring_screen_radius`'s average of the two axes.
    let sx = image_rect.width() / iw as f32;
    let sy = image_rect.height() / ih as f32;
    let scale = 0.5 * (sx + sy);
    if scale <= 1e-6 {
        return 1.0;
    }
    (2.0 * dist / scale).max(1.0)
}

/// Screen radius for a point's optional disk: pixel diameter or RadiusSpace rim.
///
/// Diameter is in **source-image pixels** (sample pixel). Returns `None` for the
/// single-pixel default (`diameter ≤ 1`) so the crosshair stays a pure point.
pub fn ring_screen_radius(
    diameter: Option<f32>,
    radius_val: Option<f32>,
    radius_space: Option<RadiusSpace>,
    image_rect: Rect,
    dims: Option<(u32, u32)>,
) -> Option<f32> {
    if let (Some(d), Some((iw, ih))) = (diameter, dims) {
        if d > 1.0 && iw > 0 && ih > 0 {
            let rx = (d * 0.5) * (image_rect.width() / iw as f32);
            let ry = (d * 0.5) * (image_rect.height() / ih as f32);
            // Average axes so a non-square image_rect still tracks the pixel
            // disk; at aspect-correct rects this is exact.
            return Some(0.5 * (rx + ry));
        }
    }
    if let (Some(rv), Some(space), Some(dims)) = (radius_val, radius_space, dims) {
        let r = space.to_screen_radius(rv, [image_rect.width(), image_rect.height()], dims);
        if r > 0.5 {
            return Some(r);
        }
    }
    None
}

/// Paint the crosshair, optional sample disk, centre dot, and note/diameter chip.
#[allow(clippy::too_many_arguments)]
fn draw_crosshair(
    ui: &egui::Ui,
    image_rect: Rect,
    unit: [f32; 2],
    ring_r: Option<f32>,
    diameter: Option<f32>,
    active: bool,
    editable: bool,
    note: Option<String>,
    theme: &Theme,
) {
    let painter = ui.painter().with_clip_rect(image_rect);
    let center = norm_to_screen(image_rect, unit);
    let colors = theme.get();
    let ring_color = if editable {
        colors.node_header_selected_border
    } else {
        handle::read_only_color(theme)
    };

    handle::draw_guide(&painter, image_rect, center, true, theme);
    handle::draw_guide(&painter, image_rect, center, false, theme);

    // Centre grip first so the sample disk (drawn next) sits on top of it —
    // otherwise a multi-pixel diameter that is only a few screen pixels when
    // the photo is fit-to-view is completely hidden under the grip and looks
    // like the slider does nothing.
    if editable {
        let radius = if active { GRIP_RADIUS_ACTIVE + 1.0 } else { GRIP_RADIUS + 1.0 };
        handle::draw_handle(ui.painter(), center, radius, active, HandleShape::Dot, theme);
    } else {
        ui.painter().circle_stroke(
            center,
            GRIP_RADIUS + 1.0,
            Stroke::new(1.5, handle::read_only_color(theme)),
        );
    }

    if let Some(r) = ring_r {
        // Soft fill marks the averaged neighbourhood even when the stroke is
        // sub-grip-size on a zoomed-out photo.
        let fill = Color32::from_rgba_unmultiplied(
            ring_color.r(),
            ring_color.g(),
            ring_color.b(),
            if active { 56 } else { 36 },
        );
        painter.circle_filled(center, r, fill);
        painter.circle_stroke(
            center,
            r,
            Stroke::new(if active { 2.0 } else { 1.5 }, ring_color),
        );
    }

    // Diameter chip: the ring alone is often sub-pixel on large fit-to-view
    // images, so the slider needs a readout that always tracks the live value.
    let diam = diameter.unwrap_or(1.0).max(1.0);
    let label = if diam > 1.0 {
        let chip = format!("⌀{diam:.0}");
        Some(match note {
            Some(n) => format!("{chip}\n{n}"),
            None => chip,
        })
    } else {
        note
    };
    if let Some(text) = label {
        draw_readout(ui, image_rect, center + Vec2::new(10.0, 10.0), &text, theme);
    }
}

/// Push a value change and mark for live engine commit.
fn push_live(out: &mut SpatialOverlayResponse, idx: usize, input: &Input, v: f32) {
    out.changed.push((idx, write_scalar(&input.value, v)));
    out.commit = true;
    out.commit_inputs.push(idx);
}

// ----------------------------------------------------------------- line gizmo

/// Graduated filter: angle (degrees) + position (0–1) along the gradient axis.
#[allow(clippy::too_many_arguments)]
fn show_line(
    ui: &mut egui::Ui,
    id: egui::Id,
    image_rect: Rect,
    ctx: &GizmoContext<'_>,
    angle_idx: usize,
    pos_idx: usize,
    _space: SpatialSpace,
    editable: bool,
    theme: &Theme,
    out: &mut SpatialOverlayResponse,
) {
    let Some(mut angle) = read_scalar(&ctx.inputs[angle_idx].value) else { return };
    let Some(mut position) = read_scalar(&ctx.inputs[pos_idx].value) else { return };

    // Gradient direction (angle 0 = +x). The soft edge is perpendicular.
    let rad = angle.to_radians();
    let dir = Vec2::new(rad.cos(), rad.sin());
    let mid = image_rect.center();
    let diag = (image_rect.width().hypot(image_rect.height())).max(1.0);
    // Project image half-diagonal so the line spans the view.
    let span = diag * 0.6;
    // Position 0.5 is centre; shift along dir by (position - 0.5) * full projection span.
    let origin = mid + dir * ((position - 0.5) * diag);
    let perp = Vec2::new(-dir.y, dir.x);
    let a = origin - perp * span;
    let b = origin + perp * span;

    let painter = ui.painter().with_clip_rect(image_rect);
    let colors = theme.get();
    let stroke = Stroke::new(
        1.5,
        if editable {
            colors.grid_connection_line
        } else {
            handle::read_only_color(theme)
        },
    );
    painter.line_segment([a, b], stroke);
    // Direction tick at mid-line.
    painter.arrow(origin, dir * 24.0, stroke);

    if editable {
        // Drag body: move position along dir.
        let body = handle::handle(ui, id.with("pos"), origin, HANDLE_HIT_HALF + 4.0);
        if let Some(to) = body.drag_to {
            let delta = to - mid;
            // Project onto dir; map to 0–1 roughly spanning the image diagonal.
            let proj = delta.dot(dir) / diag + 0.5;
            let next = quantize(&ctx.inputs[pos_idx], proj);
            if (next - position).abs() > 1e-6 {
                position = next;
                push_live(out, pos_idx, &ctx.inputs[pos_idx], next);
            }
        }
        if body.commit {
            out.commit = true;
            out.commit_inputs.push(pos_idx);
        }

        // Angle handle on the direction tick.
        let angle_pos = origin + dir * 28.0;
        let ang = handle::handle(ui, id.with("ang"), angle_pos, HANDLE_HIT_HALF);
        if let Some(to) = ang.drag_to {
            let v = to - origin;
            if v.length_sq() > 1.0 {
                let deg = v.y.atan2(v.x).to_degrees().rem_euclid(360.0);
                let next = quantize(&ctx.inputs[angle_idx], deg);
                if (next - angle).abs() > 1e-3 {
                    angle = next;
                    push_live(out, angle_idx, &ctx.inputs[angle_idx], next);
                }
            }
        }
        if ang.commit {
            out.commit = true;
            out.commit_inputs.push(angle_idx);
        }

        handle::draw_handle(ui.painter(), origin, GRIP_RADIUS + 1.0, body.active, HandleShape::Dot, theme);
        handle::draw_handle(ui.painter(), angle_pos, GRIP_RADIUS, ang.active, HandleShape::Square, theme);
    } else if let Some(note) = driven_note(ctx, &[angle_idx, pos_idx]) {
        draw_readout(ui, image_rect, origin + Vec2::new(8.0, 8.0), &note, theme);
    }
    let _ = (angle, position); // silence when not editable
}

// ---------------------------------------------------------------- axes gizmo

/// Vertical and/or horizontal split lines (mirror).
#[allow(clippy::too_many_arguments)]
fn show_axes(
    ui: &mut egui::Ui,
    id: egui::Id,
    image_rect: Rect,
    ctx: &GizmoContext<'_>,
    x_idx: Option<usize>,
    y_idx: Option<usize>,
    _space: SpatialSpace,
    editable: bool,
    theme: &Theme,
    out: &mut SpatialOverlayResponse,
) {
    let colors = theme.get();
    let stroke = Stroke::new(
        1.5,
        if editable {
            colors.grid_connection_line
        } else {
            handle::read_only_color(theme)
        },
    );
    let painter = ui.painter().with_clip_rect(image_rect);

    if let Some(xi) = x_idx {
        if let Some(xv) = read_scalar(&ctx.inputs[xi].value) {
            let mut xv = xv;
            let px = image_rect.left() + xv.clamp(0.0, 1.0) * image_rect.width();
            painter.line_segment(
                [Pos2::new(px, image_rect.top()), Pos2::new(px, image_rect.bottom())],
                stroke,
            );
            if editable {
                let mid = Pos2::new(px, image_rect.center().y);
                let grab = handle::handle(ui, id.with("ax"), mid, HANDLE_HIT_HALF);
                if let Some(to) = grab.drag_to {
                    let next = quantize(
                        &ctx.inputs[xi],
                        ((to.x - image_rect.left()) / image_rect.width()).clamp(0.0, 1.0),
                    );
                    if (next - xv).abs() > 1e-6 {
                        xv = next;
                        push_live(out, xi, &ctx.inputs[xi], next);
                    }
                }
                if grab.commit {
                    out.commit = true;
                    out.commit_inputs.push(xi);
                }
                handle::draw_handle(ui.painter(), mid, GRIP_RADIUS, grab.active, HandleShape::Square, theme);
            }
            let _ = xv;
        }
    }
    if let Some(yi) = y_idx {
        if let Some(yv) = read_scalar(&ctx.inputs[yi].value) {
            let mut yv = yv;
            let py = image_rect.top() + yv.clamp(0.0, 1.0) * image_rect.height();
            painter.line_segment(
                [Pos2::new(image_rect.left(), py), Pos2::new(image_rect.right(), py)],
                stroke,
            );
            if editable {
                let mid = Pos2::new(image_rect.center().x, py);
                let grab = handle::handle(ui, id.with("ay"), mid, HANDLE_HIT_HALF);
                if let Some(to) = grab.drag_to {
                    let next = quantize(
                        &ctx.inputs[yi],
                        ((to.y - image_rect.top()) / image_rect.height()).clamp(0.0, 1.0),
                    );
                    if (next - yv).abs() > 1e-6 {
                        yv = next;
                        push_live(out, yi, &ctx.inputs[yi], next);
                    }
                }
                if grab.commit {
                    out.commit = true;
                    out.commit_inputs.push(yi);
                }
                handle::draw_handle(ui.painter(), mid, GRIP_RADIUS, grab.active, HandleShape::Square, theme);
            }
            let _ = yv;
        }
    }
}

// ----------------------------------------------------------------- quad gizmo

/// Perspective: four corners as offsets from the unit-square corners.
#[allow(clippy::too_many_arguments)]
fn show_quad(
    ui: &mut egui::Ui,
    id: egui::Id,
    image_rect: Rect,
    ctx: &GizmoContext<'_>,
    corners: [usize; 8],
    editable: bool,
    theme: &Theme,
    out: &mut SpatialOverlayResponse,
) {
    // Base corners in unit space: TL, TR, BR, BL.
    const BASE: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let mut offs = [0.0f32; 8];
    for (i, &idx) in corners.iter().enumerate() {
        offs[i] = read_scalar(&ctx.inputs[idx].value).unwrap_or(0.0);
    }
    let mut screen = [Pos2::ZERO; 4];
    for c in 0..4 {
        let u = [
            (BASE[c][0] + offs[c * 2]).clamp(-0.5, 1.5),
            (BASE[c][1] + offs[c * 2 + 1]).clamp(-0.5, 1.5),
        ];
        screen[c] = norm_to_screen(image_rect, u);
    }

    let painter = ui.painter().with_clip_rect(image_rect);
    let colors = theme.get();
    let stroke = Stroke::new(
        1.5,
        if editable {
            colors.grid_connection_line
        } else {
            handle::read_only_color(theme)
        },
    );
    for i in 0..4 {
        painter.line_segment([screen[i], screen[(i + 1) % 4]], stroke);
    }

    if editable {
        for c in 0..4 {
            let grab = handle::handle(ui, id.with(("c", c as u8)), screen[c], HANDLE_HIT_HALF);
            if let Some(to) = grab.drag_to {
                let unit = screen_to_norm(image_rect, to);
                let nx = quantize(&ctx.inputs[corners[c * 2]], unit[0] - BASE[c][0]);
                let ny = quantize(&ctx.inputs[corners[c * 2 + 1]], unit[1] - BASE[c][1]);
                if (nx - offs[c * 2]).abs() > 1e-6 || (ny - offs[c * 2 + 1]).abs() > 1e-6 {
                    offs[c * 2] = nx;
                    offs[c * 2 + 1] = ny;
                    push_live(out, corners[c * 2], &ctx.inputs[corners[c * 2]], nx);
                    push_live(out, corners[c * 2 + 1], &ctx.inputs[corners[c * 2 + 1]], ny);
                }
            }
            if grab.commit {
                out.commit = true;
                out.commit_inputs.push(corners[c * 2]);
                out.commit_inputs.push(corners[c * 2 + 1]);
            }
            handle::draw_handle(
                ui.painter(),
                screen[c],
                if grab.active { GRIP_RADIUS_ACTIVE } else { GRIP_RADIUS },
                grab.active,
                HandleShape::Square,
                theme,
            );
        }
    }
}

// ------------------------------------------------------------ transform gizmo

/// Affine transform: drag body to offset, handle to rotate, optional scale.
#[allow(clippy::too_many_arguments)]
fn show_transform(
    ui: &mut egui::Ui,
    id: egui::Id,
    image_rect: Rect,
    ctx: &GizmoContext<'_>,
    ox_idx: usize,
    oy_idx: usize,
    rot_idx: usize,
    sx_idx: Option<usize>,
    sy_idx: Option<usize>,
    editable: bool,
    theme: &Theme,
    out: &mut SpatialOverlayResponse,
) {
    let Some(mut ox) = read_scalar(&ctx.inputs[ox_idx].value) else { return };
    let Some(mut oy) = read_scalar(&ctx.inputs[oy_idx].value) else { return };
    let Some(mut rot) = read_scalar(&ctx.inputs[rot_idx].value) else { return };
    let sx = sx_idx.and_then(|i| read_scalar(&ctx.inputs[i].value)).unwrap_or(1.0);
    let sy = sy_idx.and_then(|i| read_scalar(&ctx.inputs[i].value)).unwrap_or(1.0);

    // Offset is a fraction of size: 0 = centre stays put; positive moves content
    // right/down so the original centre lands at unit (0.5 + ox, 0.5 + oy).
    let center = Pos2::new(
        image_rect.left() + (0.5 + ox) * image_rect.width(),
        image_rect.top() + (0.5 + oy) * image_rect.height(),
    );

    let rad = rot.to_radians();
    let dir = Vec2::new(rad.cos(), rad.sin());
    let arm = 40.0 * ((sx.abs() + sy.abs()) * 0.5).clamp(0.25, 3.0);
    let tip = center + dir * arm;

    let painter = ui.painter().with_clip_rect(image_rect);
    let colors = theme.get();
    let stroke = Stroke::new(
        1.5,
        if editable {
            colors.grid_connection_line
        } else {
            handle::read_only_color(theme)
        },
    );
    painter.circle_stroke(center, 10.0, stroke);
    painter.arrow(center, dir * arm, stroke);

    if editable {
        let body = handle::handle(ui, id.with("body"), center, HANDLE_HIT_HALF + 2.0);
        if let Some(to) = body.drag_to {
            let unit = screen_to_norm(image_rect, to);
            let nx = quantize(&ctx.inputs[ox_idx], unit[0] - 0.5);
            let ny = quantize(&ctx.inputs[oy_idx], unit[1] - 0.5);
            if (nx - ox).abs() > 1e-6 || (ny - oy).abs() > 1e-6 {
                ox = nx;
                oy = ny;
                push_live(out, ox_idx, &ctx.inputs[ox_idx], nx);
                push_live(out, oy_idx, &ctx.inputs[oy_idx], ny);
            }
        }
        if body.commit {
            out.commit = true;
            out.commit_inputs.extend([ox_idx, oy_idx]);
        }

        let rot_h = handle::handle(ui, id.with("rot"), tip, HANDLE_HIT_HALF);
        if let Some(to) = rot_h.drag_to {
            let v = to - center;
            if v.length_sq() > 1.0 {
                let deg = v.y.atan2(v.x).to_degrees();
                let next = quantize(&ctx.inputs[rot_idx], deg);
                if (next - rot).abs() > 1e-3 {
                    rot = next;
                    push_live(out, rot_idx, &ctx.inputs[rot_idx], next);
                }
            }
            // Scale from arm length when scale inputs exist.
            if let (Some(sxi), Some(syi)) = (sx_idx, sy_idx) {
                let len = v.length() / 40.0;
                let ns = quantize(&ctx.inputs[sxi], len.clamp(0.01, 4.0));
                if (ns - sx).abs() > 1e-4 {
                    push_live(out, sxi, &ctx.inputs[sxi], ns);
                    push_live(out, syi, &ctx.inputs[syi], quantize(&ctx.inputs[syi], ns));
                }
            }
        }
        if rot_h.commit {
            out.commit = true;
            out.commit_inputs.push(rot_idx);
            if let Some(s) = sx_idx {
                out.commit_inputs.push(s);
            }
            if let Some(s) = sy_idx {
                out.commit_inputs.push(s);
            }
        }

        handle::draw_handle(ui.painter(), center, GRIP_RADIUS + 1.0, body.active, HandleShape::Dot, theme);
        handle::draw_handle(ui.painter(), tip, GRIP_RADIUS, rot_h.active, HandleShape::Square, theme);
    }
    let _ = (ox, oy, rot);
}

// ----------------------------------------------------------- offset-px gizmo

/// Drop-shadow style offset in px@1024, drawn from image centre.
#[allow(clippy::too_many_arguments)]
fn show_offset_px(
    ui: &mut egui::Ui,
    id: egui::Id,
    image_rect: Rect,
    ctx: &GizmoContext<'_>,
    x_idx: usize,
    y_idx: usize,
    editable: bool,
    theme: &Theme,
    out: &mut SpatialOverlayResponse,
) {
    let Some(mut ox) = read_scalar(&ctx.inputs[x_idx].value) else { return };
    let Some(mut oy) = read_scalar(&ctx.inputs[y_idx].value) else { return };
    let Some((iw, ih)) = ctx.image_dims else { return };
    if iw == 0 || ih == 0 {
        return;
    }
    // px@1024 → actual pixels → screen.
    let scale = iw.max(ih) as f32 / 1024.0;
    let px = ox * scale;
    let py = oy * scale;
    let origin = image_rect.center();
    let tip = Pos2::new(
        origin.x + px * (image_rect.width() / iw as f32),
        origin.y + py * (image_rect.height() / ih as f32),
    );

    let painter = ui.painter().with_clip_rect(image_rect);
    let colors = theme.get();
    let stroke = Stroke::new(
        1.5,
        if editable {
            colors.grid_connection_line
        } else {
            handle::read_only_color(theme)
        },
    );
    painter.arrow(origin, tip - origin, stroke);
    painter.circle_filled(origin, 3.0, stroke.color);

    if editable {
        let grab = handle::handle(ui, id.with("off"), tip, HANDLE_HIT_HALF);
        if let Some(to) = grab.drag_to {
            let dx = (to.x - origin.x) * (iw as f32 / image_rect.width()) / scale;
            let dy = (to.y - origin.y) * (ih as f32 / image_rect.height()) / scale;
            let nx = quantize(&ctx.inputs[x_idx], dx);
            let ny = quantize(&ctx.inputs[y_idx], dy);
            if (nx - ox).abs() > 1e-4 || (ny - oy).abs() > 1e-4 {
                ox = nx;
                oy = ny;
                push_live(out, x_idx, &ctx.inputs[x_idx], nx);
                push_live(out, y_idx, &ctx.inputs[y_idx], ny);
            }
        }
        if grab.commit {
            out.commit = true;
            out.commit_inputs.extend([x_idx, y_idx]);
        }
        handle::draw_handle(ui.painter(), tip, GRIP_RADIUS, grab.active, HandleShape::Dot, theme);
    }
    let _ = (ox, oy);
}

// ------------------------------------------------------- centre-radius gizmo

/// Radius ring fixed at image centre (vignette / swirl / spherize).
#[allow(clippy::too_many_arguments)]
fn show_center_radius(
    ui: &mut egui::Ui,
    id: egui::Id,
    image_rect: Rect,
    ctx: &GizmoContext<'_>,
    radius_idx: usize,
    space: RadiusSpace,
    editable: bool,
    theme: &Theme,
    out: &mut SpatialOverlayResponse,
) {
    let Some(mut rv) = read_scalar(&ctx.inputs[radius_idx].value) else { return };
    let Some(dims) = ctx.image_dims else { return };
    let center = image_rect.center();
    let screen_r = space.to_screen_radius(rv, [image_rect.width(), image_rect.height()], dims);

    let painter = ui.painter().with_clip_rect(image_rect);
    let colors = theme.get();
    let stroke = Stroke::new(
        1.5,
        if editable {
            colors.node_header_selected_border
        } else {
            handle::read_only_color(theme)
        },
    );
    if screen_r > 0.5 {
        painter.circle_stroke(center, screen_r, stroke);
    }

    if editable {
        let rim = center + Vec2::new(screen_r.max(HANDLE_HIT_HALF), 0.0);
        let grab = handle::handle(ui, id.with("r"), rim, HANDLE_HIT_HALF);
        if let Some(to) = grab.drag_to {
            let dist = (to - center).length();
            let next = quantize(
                &ctx.inputs[radius_idx],
                space.from_screen_radius(dist, [image_rect.width(), image_rect.height()], dims),
            );
            if (next - rv).abs() > 1e-5 {
                rv = next;
                push_live(out, radius_idx, &ctx.inputs[radius_idx], next);
            }
        }
        if grab.commit {
            out.commit = true;
            out.commit_inputs.push(radius_idx);
        }
        handle::draw_handle(ui.painter(), rim, GRIP_RADIUS, grab.active, HandleShape::Square, theme);
    }
    let _ = rv;
}

// ------------------------------------------------------------ placement gizmo

/// The eight resize grips of a placement box, as signs in the box's **own
/// unrotated frame**: `(1, -1)` is the top-right corner, `(1, 0)` the right
/// edge. Edges come first so a corner wins where the two overlap.
const PLACEMENT_GRIPS: [(i8, i8); 8] =
    [(0, -1), (0, 1), (-1, 0), (1, 0), (-1, -1), (1, -1), (1, 1), (-1, 1)];

/// Where a compositing node's foreground lands inside the background.
///
/// Unlike the other gizmos this box's *size* is not a value the user types: it
/// is the foreground image's own pixel size times the scales, so the whole
/// thing is unavailable until a foreground is connected. Positions are
/// background pixels rather than a fraction, which is why the mapping here is
/// written out instead of going through `SpatialSpace`.
///
/// Geometry is derived fresh from the (round-tripped) values every frame, so
/// the box is always exactly what `placement::place` will render — including
/// its rounding of the scaled size to whole pixels.
#[allow(clippy::too_many_arguments)]
fn show_placement(
    ui: &mut egui::Ui,
    id: egui::Id,
    image_rect: Rect,
    ctx: &GizmoContext<'_>,
    image_idx: usize,
    idx: [usize; 5],
    editable: bool,
    theme: &Theme,
    out: &mut SpatialOverlayResponse,
) {
    let [x_idx, y_idx, sx_idx, sy_idx, rot_idx] = idx;
    // No foreground, no box: its size has nowhere to come from.
    let Some((fw, fh)) = ctx.image_input_dims.get(&image_idx).copied() else { return };
    // Positions are background pixels, so the backdrop supplies the mapping.
    let Some((bw, bh)) = ctx.image_dims else { return };
    if fw == 0 || fh == 0 || bw == 0 || bh == 0 {
        return;
    }

    let Some(mut px) = read_scalar(&ctx.inputs[x_idx].value) else { return };
    let Some(mut py) = read_scalar(&ctx.inputs[y_idx].value) else { return };
    let Some(mut sx) = read_scalar(&ctx.inputs[sx_idx].value) else { return };
    let Some(mut sy) = read_scalar(&ctx.inputs[sy_idx].value) else { return };
    let Some(mut rot) = read_scalar(&ctx.inputs[rot_idx].value) else { return };

    let (bwf, bhf) = (bw as f32, bh as f32);
    let to_screen = |p: [f32; 2]| {
        Pos2::new(
            image_rect.left() + p[0] / bwf * image_rect.width(),
            image_rect.top() + p[1] / bhf * image_rect.height(),
        )
    };
    let to_image = |s: Pos2| {
        [
            (s.x - image_rect.left()) / image_rect.width() * bwf,
            (s.y - image_rect.top()) / image_rect.height() * bhf,
        ]
    };
    let to_image_delta = |d: Vec2| {
        [d.x / image_rect.width() * bwf, d.y / image_rect.height() * bhf]
    };

    // Mirrors `placement::place`: the drawn size is the whole-pixel size the
    // operation will really produce, never the raw slider product.
    let derive = |px: f32, py: f32, sx: f32, sy: f32, rot: f32| {
        let hw = (fw as f32 * sx).round().max(1.0) * 0.5;
        let hh = (fh as f32 * sy).round().max(1.0) * 0.5;
        let (sin_t, cos_t) = rot.to_radians().sin_cos();
        Placement { hw, hh, centre: [px + hw, py + hh], sin_t, cos_t }
    };

    let mut geom = derive(px, py, sx, sy, rot);
    let mut active = false;

    if editable {
        // On-screen floor for the box, so the eight grips can't knot together
        // however far the view is zoomed out.
        let min_half_w = (MIN_RECT_PX * bwf / image_rect.width()).max(1.0) * 0.5;
        let min_half_h = (MIN_RECT_PX * bhf / image_rect.height()).max(1.0) * 0.5;

        // --- body: move ---
        // Registered only while the pointer is genuinely inside the rotated
        // quad (or already dragging it). egui can only interact with a rect, and
        // a rotated box's bounding rect covers empty canvas — claiming that
        // would kill drag-to-pan beside its corners.
        let body_id = id.with("body");
        let corners = geom.corners(&to_screen);
        let inside = ui
            .ctx()
            .pointer_latest_pos()
            .is_some_and(|p| quad_contains(&corners, p));
        if inside || ui.ctx().is_being_dragged(body_id) {
            let body = handle::region(ui, body_id, Rect::from_points(&corners));
            let anchor = press_anchor(ui, body_id, body.started, [px, py]);
            active |= body.active;
            if let (Some(to), Some(press)) =
                (body.drag_to, ui.ctx().input(|i| i.pointer.press_origin()))
            {
                // Total travel from the press, not this frame's delta: on an
                // integer input a per-frame delta can round to zero at high
                // zoom and lose the gesture entirely.
                let travel = to_image_delta(to - press);
                let nx = quantize(&ctx.inputs[x_idx], anchor[0] + travel[0]);
                let ny = quantize(&ctx.inputs[y_idx], anchor[1] + travel[1]);
                if (nx - px).abs() > 1e-6 || (ny - py).abs() > 1e-6 {
                    px = nx;
                    py = ny;
                    geom = derive(px, py, sx, sy, rot);
                    push_live(out, x_idx, &ctx.inputs[x_idx], nx);
                    push_live(out, y_idx, &ctx.inputs[y_idx], ny);
                    out.commit = true;
                    out.commit_inputs.extend([x_idx, y_idx]);
                }
            }
            if body.commit {
                out.commit = true;
                out.commit_inputs.extend([x_idx, y_idx]);
            }
        }

        // --- grips: scale, pinning the opposite corner/edge ---
        for (n, (ax, ay)) in PLACEMENT_GRIPS.iter().copied().enumerate() {
            let pos = to_screen(geom.local_to_image([
                ax as f32 * geom.hw,
                ay as f32 * geom.hh,
            ]));
            let grip = handle::handle(ui, id.with(("grip", n as u8)), pos, HANDLE_HIT_HALF);
            active |= grip.active;
            if let Some(to) = grip.drag_to {
                let l = geom.image_to_local(to_image(to));
                let nhw =
                    if ax != 0 { (ax as f32 * l[0]).max(min_half_w) } else { geom.hw };
                let nhh =
                    if ay != 0 { (ay as f32 * l[1]).max(min_half_h) } else { geom.hh };

                let nsx = quantize(&ctx.inputs[sx_idx], 2.0 * nhw / fw as f32);
                let nsy = quantize(&ctx.inputs[sy_idx], 2.0 * nhh / fh as f32);
                // Re-derive from the *quantized* scales before placing the
                // top-left, so the box's corner matches the size that will
                // actually be rendered rather than the raw drag.
                let sized = derive(0.0, 0.0, nsx, nsy, rot);
                let top_left = pinned_top_left(&geom, (ax, ay), &sized);
                let nx = quantize(&ctx.inputs[x_idx], top_left[0]);
                let ny = quantize(&ctx.inputs[y_idx], top_left[1]);

                if (nsx - sx).abs() > 1e-6
                    || (nsy - sy).abs() > 1e-6
                    || (nx - px).abs() > 1e-6
                    || (ny - py).abs() > 1e-6
                {
                    sx = nsx;
                    sy = nsy;
                    px = nx;
                    py = ny;
                    geom = derive(px, py, sx, sy, rot);
                    push_live(out, sx_idx, &ctx.inputs[sx_idx], nsx);
                    push_live(out, sy_idx, &ctx.inputs[sy_idx], nsy);
                    push_live(out, x_idx, &ctx.inputs[x_idx], nx);
                    push_live(out, y_idx, &ctx.inputs[y_idx], ny);
                    out.commit = true;
                    out.commit_inputs.extend([x_idx, y_idx, sx_idx, sy_idx]);
                }
            }
            if grip.commit {
                out.commit = true;
                out.commit_inputs.extend([x_idx, y_idx, sx_idx, sy_idx]);
            }
        }

        // --- knob: rotate ---
        let knob_pos = geom.knob_position(&to_screen);
        let knob = handle::handle(ui, id.with("rot"), knob_pos, HANDLE_HIT_HALF);
        active |= knob.active;
        if let Some(to) = knob.drag_to {
            let p = to_image(to);
            let d = [p[0] - geom.centre[0], p[1] - geom.centre[1]];
            if d[0].hypot(d[1]) > 1e-3 {
                // The knob sits straight up from the centre, i.e. at -90°.
                let mut deg = d[1].atan2(d[0]).to_degrees() + 90.0;
                if ui.input(|i| i.modifiers.shift) {
                    deg = (deg / ROTATE_SNAP_DEG).round() * ROTATE_SNAP_DEG;
                }
                // atan2 wraps at ±180; pick the equivalent turn nearest the
                // current value so dragging across the wrap doesn't spin the
                // box a full revolution.
                deg += ((rot - deg) / 360.0).round() * 360.0;
                let next = quantize(&ctx.inputs[rot_idx], deg);
                if (next - rot).abs() > 1e-4 {
                    rot = next;
                    geom = derive(px, py, sx, sy, rot);
                    push_live(out, rot_idx, &ctx.inputs[rot_idx], next);
                    out.commit = true;
                    out.commit_inputs.push(rot_idx);
                }
            }
        }
        if knob.commit {
            out.commit = true;
            out.commit_inputs.push(rot_idx);
        }
    }

    let note = (!editable).then(|| driven_note(ctx, &idx)).flatten();
    draw_placement(ui, image_rect, &geom, &to_screen, active, editable, note, theme);
}

/// A placement box's geometry in background pixels, derived from the node's
/// (already round-tripped) values.
struct Placement {
    /// Half the placed width / height, in background pixels.
    hw: f32,
    hh: f32,
    /// The rotation pivot: the centre of the placed rect.
    centre: [f32; 2],
    sin_t: f32,
    cos_t: f32,
}

impl Placement {
    /// An offset in the box's own unrotated frame → background pixels.
    fn local_to_image(&self, l: [f32; 2]) -> [f32; 2] {
        let r = rotate(l, self.sin_t, self.cos_t);
        [self.centre[0] + r[0], self.centre[1] + r[1]]
    }

    /// Exact inverse of [`Self::local_to_image`].
    fn image_to_local(&self, w: [f32; 2]) -> [f32; 2] {
        let d = [w[0] - self.centre[0], w[1] - self.centre[1]];
        // Rotation matrices are orthonormal, so the inverse is the transpose.
        [
            d[0] * self.cos_t + d[1] * self.sin_t,
            -d[0] * self.sin_t + d[1] * self.cos_t,
        ]
    }

    /// The four corners on screen, clockwise from the box's own top-left.
    fn corners(&self, to_screen: &impl Fn([f32; 2]) -> Pos2) -> [Pos2; 4] {
        [
            to_screen(self.local_to_image([-self.hw, -self.hh])),
            to_screen(self.local_to_image([self.hw, -self.hh])),
            to_screen(self.local_to_image([self.hw, self.hh])),
            to_screen(self.local_to_image([-self.hw, self.hh])),
        ]
    }

    /// The rotation knob, a fixed *screen* distance out from the top edge so it
    /// stays reachable whatever the zoom or the box's pixel size.
    fn knob_position(&self, to_screen: &impl Fn([f32; 2]) -> Pos2) -> Pos2 {
        let top = to_screen(self.local_to_image([0.0, -self.hh]));
        let centre = to_screen(self.centre);
        let up = top - centre;
        let dir = if up.length() > 1e-3 { up / up.length() } else { Vec2::new(0.0, -1.0) };
        top + dir * ROTATE_KNOB_GAP
    }
}

/// Rotate an offset by the angle whose sine and cosine are given. Positive is
/// clockwise on screen, matching `placement::place` (and the y-down convention
/// every image op uses).
fn rotate(l: [f32; 2], sin_t: f32, cos_t: f32) -> [f32; 2] {
    [l[0] * cos_t - l[1] * sin_t, l[0] * sin_t + l[1] * cos_t]
}

/// The unrotated top-left of a box resized from `geom` to `sized`'s half-extents
/// while the grip *opposite* `sign` stays exactly where it was.
///
/// Without this a resize would grow the box symmetrically about its centre and
/// the far edge would slide out from under the pointer. Solving for the centre
/// (rather than nudging it) is also what keeps the pin exact when the box is
/// rotated, where "the opposite corner" is not an axis-aligned position.
fn pinned_top_left(geom: &Placement, sign: (i8, i8), sized: &Placement) -> [f32; 2] {
    let opposite = [-(sign.0 as f32), -(sign.1 as f32)];
    let pinned = geom.local_to_image([opposite[0] * geom.hw, opposite[1] * geom.hh]);
    let offset =
        rotate([opposite[0] * sized.hw, opposite[1] * sized.hh], geom.sin_t, geom.cos_t);
    [pinned[0] - offset[0] - sized.hw, pinned[1] - offset[1] - sized.hh]
}

/// Is `p` inside the convex quad? True when every edge puts it on the same
/// side, which stays correct for a quad wound either way.
fn quad_contains(quad: &[Pos2; 4], p: Pos2) -> bool {
    let (mut positive, mut negative) = (false, false);
    for i in 0..4 {
        let a = quad[i];
        let b = quad[(i + 1) % 4];
        let cross = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
        positive |= cross > 0.0;
        negative |= cross < 0.0;
    }
    !(positive && negative)
}

/// The value a drag started from, stashed in egui's per-id memory.
///
/// The one piece of state this module keeps, and it earns its place: an integer
/// input driven by per-frame `drag_delta` loses every displacement that rounds
/// to zero, so a slow drag at high zoom moves nothing at all. Anchoring to the
/// value at press plus the pointer's *total* travel is exact at any zoom. Keyed
/// by widget id, so it needs no lifetime here and cannot leak across panels.
fn press_anchor(ui: &egui::Ui, id: egui::Id, started: bool, current: [f32; 2]) -> [f32; 2] {
    let key = id.with("press");
    if started {
        ui.data_mut(|d| d.insert_temp(key, current));
        return current;
    }
    ui.data(|d| d.get_temp(key)).unwrap_or(current)
}

/// Paint the placement box: outline, grips, centre mark and rotation knob.
#[allow(clippy::too_many_arguments)]
fn draw_placement(
    ui: &egui::Ui,
    image_rect: Rect,
    geom: &Placement,
    to_screen: &impl Fn([f32; 2]) -> Pos2,
    active: bool,
    editable: bool,
    note: Option<String>,
    theme: &Theme,
) {
    let colors = theme.get();
    let painter = ui.painter().with_clip_rect(image_rect);
    let outline =
        if editable { colors.grid_connection_line } else { handle::read_only_color(theme) };
    let corners = geom.corners(to_screen);

    for i in 0..4 {
        painter.line_segment([corners[i], corners[(i + 1) % 4]], Stroke::new(2.0, outline));
    }
    // Centre mark: the pivot rotation turns about, so it is worth showing.
    let centre = to_screen(geom.centre);
    let tick = Stroke::new(1.0, colors.text_faint.gamma_multiply(0.6));
    painter.line_segment([centre - Vec2::new(4.0, 0.0), centre + Vec2::new(4.0, 0.0)], tick);
    painter.line_segment([centre - Vec2::new(0.0, 4.0), centre + Vec2::new(0.0, 4.0)], tick);

    if editable {
        let knob = geom.knob_position(to_screen);
        let top = to_screen(geom.local_to_image([0.0, -geom.hh]));
        painter.line_segment([top, knob], Stroke::new(1.0, outline));
        let radius = if active { GRIP_RADIUS_ACTIVE } else { GRIP_RADIUS };
        for (ax, ay) in PLACEMENT_GRIPS.iter().copied() {
            let pos = to_screen(geom.local_to_image([ax as f32 * geom.hw, ay as f32 * geom.hh]));
            let hovered = ui.rect_contains_pointer(handle::hit_rect(pos, HANDLE_HIT_HALF));
            handle::draw_handle(
                ui.painter(),
                pos,
                radius,
                hovered || active,
                HandleShape::Square,
                theme,
            );
        }
        let knob_hot = ui.rect_contains_pointer(handle::hit_rect(knob, HANDLE_HIT_HALF));
        handle::draw_handle(
            ui.painter(),
            knob,
            radius,
            knob_hot || active,
            HandleShape::Dot,
            theme,
        );
    }

    if let Some(note) = note {
        draw_readout(ui, image_rect, corners[0] + Vec2::new(4.0, 4.0), &note, theme);
    }
}

// ----------------------------------------------------------------- rect gizmo

/// Which region of a box a pointer grabbed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RectHandle {
    /// The interior — moves the whole box without resizing it.
    Body,
    N,
    S,
    W,
    E,
    NW,
    NE,
    SE,
    SW,
}

/// The four edge bands, registered before the corner grips so a corner wins.
const EDGE_HANDLES: [RectHandle; 4] = [RectHandle::N, RectHandle::S, RectHandle::W, RectHandle::E];
/// The eight resize grips, edges first so a corner wins where they overlap.
const GRIP_HANDLES: [RectHandle; 8] = [
    RectHandle::N,
    RectHandle::S,
    RectHandle::W,
    RectHandle::E,
    RectHandle::NW,
    RectHandle::NE,
    RectHandle::SE,
    RectHandle::SW,
];

/// A draggable, resizable box.
#[allow(clippy::too_many_arguments)]
fn show_rect(
    ui: &mut egui::Ui,
    id: egui::Id,
    image_rect: Rect,
    ctx: &GizmoContext<'_>,
    idx: [usize; 4],
    aspect: Option<(usize, usize)>,
    _space: SpatialSpace,
    extent: RectExtent,
    editable: bool,
    theme: &Theme,
    out: &mut SpatialOverlayResponse,
) {
    let Some(mut values) = read_quad(ctx.inputs, idx) else { return };
    let ratio = read_aspect(ctx.inputs, aspect);
    let locked = ratio.filter(|&(rw, rh)| rw > 0 && rh > 0);
    // When locked, draw and grab the *fitted* crop so the overlay matches
    // what `run()` will actually copy. A later drag writes that fitted
    // rect back, which is how the sliders catch up.
    if let (Some((rw, rh)), Some(dims)) = (locked, ctx.image_dims) {
        values = fit_values(values, rw, rh, dims);
    }

    if editable {
        let min = min_size(image_rect, ctx.image_dims);
        let mut corners = spec_to_corners(extent, values);
        let mut moved: Option<RectHandle> = None;
        let mut released: Option<RectHandle> = None;

        // Interior first, then edge bands, then grips — later registration wins
        // where they overlap, so corner beats edge beats body.
        let body_rect = screen_rect(image_rect, corners).shrink(EDGE_HIT_HALF);
        let body = handle::region(ui, id.with("body"), body_rect);
        let mut body_delta = Vec2::ZERO;
        if body.drag_to.is_some() {
            body_delta = body.drag_delta;
            moved = Some(RectHandle::Body);
        }
        if body.commit {
            released = Some(RectHandle::Body);
        }

        let mut grab_to: Option<Pos2> = None;
        for h in EDGE_HANDLES.iter().copied() {
            let resp = handle::region(
                ui,
                id.with(("edge", h as u8)),
                edge_band(screen_rect(image_rect, corners), h),
            );
            if let Some(to) = resp.drag_to {
                moved = Some(h);
                grab_to = Some(to);
            }
            if resp.commit {
                released = Some(h);
            }
        }
        for h in GRIP_HANDLES.iter().copied() {
            let resp = handle::handle(
                ui,
                id.with(("grip", h as u8)),
                grip_pos(screen_rect(image_rect, corners), h),
                HANDLE_HIT_HALF,
            );
            if let Some(to) = resp.drag_to {
                moved = Some(h);
                grab_to = Some(to);
            }
            if resp.commit {
                released = Some(h);
            }
        }

        if let Some(h) = moved {
            corners = if h == RectHandle::Body {
                move_corners(corners, screen_delta_to_norm(image_rect, body_delta))
            } else if let Some(to) = grab_to {
                let pointer = screen_to_norm(image_rect, to);
                match (locked, ctx.image_dims) {
                    (Some((rw, rh)), Some(dims)) => {
                        resize_corners_aspect(corners, h, pointer, min, (rw, rh), dims)
                    }
                    _ => resize_corners(corners, h, pointer, min),
                }
            } else {
                corners
            };
            let mut next = quantize_quad(ctx.inputs, idx, corners_to_spec(extent, corners));
            // Round-trip through the same resolver `run()` uses, so the drawn
            // box is exactly the committed crop.
            if let (Some((rw, rh)), Some(dims)) = (locked, ctx.image_dims) {
                next = fit_values(next, rw, rh, dims);
            }
            if next != values {
                values = next;
                for (slot, &i) in idx.iter().enumerate() {
                    out.changed.push((i, write_scalar(&ctx.inputs[i].value, values[slot])));
                }
                // Live engine update while dragging — crop re-runs every frame
                // so the output tracks the box, not just the mouse-up size.
                out.commit = true;
                for (slot, touched) in spec_inputs_touched_aspect(h, extent, locked.is_some()).iter().enumerate() {
                    if *touched {
                        out.commit_inputs.push(idx[slot]);
                    }
                }
            }
        }
        if let Some(h) = released {
            out.commit = true;
            for (slot, touched) in spec_inputs_touched_aspect(h, extent, locked.is_some()).iter().enumerate() {
                if *touched {
                    out.commit_inputs.push(idx[slot]);
                }
            }
        }

        let active = moved.is_some();
        draw_rect(ui, image_rect, values, extent, active, true, None, theme);
    } else {
        let note = driven_note(ctx, &idx);
        draw_rect(ui, image_rect, values, extent, false, false, note, theme);
    }
}

/// Paint the box: an outside scrim so the kept region reads instantly, the
/// outline, rule-of-thirds guides, and the grips. Pixel size lives in the
/// settings panel — no on-image coordinate readout.
#[allow(clippy::too_many_arguments)]
fn draw_rect(
    ui: &egui::Ui,
    image_rect: Rect,
    values: [f32; 4],
    extent: RectExtent,
    active: bool,
    editable: bool,
    note: Option<String>,
    theme: &Theme,
) {
    let colors = theme.get();
    let corners = spec_to_corners(extent, values);
    let r = screen_rect(image_rect, corners);
    let painter = ui.painter().with_clip_rect(image_rect);

    // Scrim the four regions *outside* the box rather than tinting the inside,
    // so the kept pixels stay true-colour.
    let scrim = colors.panel_fill.gamma_multiply(0.35);
    for part in [
        Rect::from_min_max(image_rect.left_top(), Pos2::new(image_rect.right(), r.top())),
        Rect::from_min_max(Pos2::new(image_rect.left(), r.bottom()), image_rect.right_bottom()),
        Rect::from_min_max(Pos2::new(image_rect.left(), r.top()), Pos2::new(r.left(), r.bottom())),
        Rect::from_min_max(Pos2::new(r.right(), r.top()), Pos2::new(image_rect.right(), r.bottom())),
    ] {
        if part.width() > 0.0 && part.height() > 0.0 {
            painter.rect_filled(part, 0.0, scrim);
        }
    }

    let outline = if editable { colors.grid_connection_line } else { handle::read_only_color(theme) };
    painter.rect_stroke(r, 0.0, Stroke::new(2.0, outline), epaint::StrokeKind::Inside);

    // Rule-of-thirds guides, quiet so they read as composition aids.
    let thirds = Stroke::new(1.0, colors.text_faint.gamma_multiply(0.5));
    for i in 1..3 {
        let f = i as f32 / 3.0;
        let x = r.left() + f * r.width();
        let y = r.top() + f * r.height();
        painter.line_segment([Pos2::new(x, r.top()), Pos2::new(x, r.bottom())], thirds);
        painter.line_segment([Pos2::new(r.left(), y), Pos2::new(r.right(), y)], thirds);
    }

    if editable {
        let radius = if active { GRIP_RADIUS_ACTIVE } else { GRIP_RADIUS };
        for h in GRIP_HANDLES.iter().copied() {
            let pos = grip_pos(r, h);
            let hovered = ui.rect_contains_pointer(handle::hit_rect(pos, HANDLE_HIT_HALF));
            handle::draw_handle(ui.painter(), pos, radius, hovered || active, HandleShape::Square, theme);
        }
    }

    // Only a driven-input note (wired upstream); no x/y/size chip.
    if let Some(note) = note {
        draw_readout(ui, image_rect, r.left_top() + Vec2::new(4.0, 4.0), &note, theme);
    }
}

/// Resolve an origin/size box to pixels exactly as `crop`'s `run()` does.
///
/// Delegates to [`mangler_core::operations::images::transform::crop::resolve_crop`]
/// so the overlay and the operation cannot drift. `ratio` is the optional
/// integer W:H lock (both 0 = free).
pub fn crop_pixels(values: [f32; 4], dims: (u32, u32)) -> (i64, i64, i64, i64) {
    crop_pixels_aspect(values, 0, 0, dims)
}

/// Same as [`crop_pixels`] with an explicit aspect-lock pair.
pub fn crop_pixels_aspect(
    values: [f32; 4],
    ratio_w: i32,
    ratio_h: i32,
    dims: (u32, u32),
) -> (i64, i64, i64, i64) {
    let p = mangler_core::operations::images::transform::crop::resolve_crop(
        values[0], values[1], values[2], values[3], ratio_w, ratio_h, dims.0, dims.1,
    );
    (p.x, p.y, p.w, p.h)
}

/// Fit an origin-size fraction quad to the pixel crop `run()` will produce.
fn fit_values(values: [f32; 4], ratio_w: i32, ratio_h: i32, dims: (u32, u32)) -> [f32; 4] {
    mangler_core::operations::images::transform::crop::resolve_crop(
        values[0], values[1], values[2], values[3], ratio_w, ratio_h, dims.0, dims.1,
    )
    .to_norm(dims.0, dims.1)
}

/// Read the optional integer W:H pair a rect gizmo names. Missing or
/// non-numeric slots are treated as 0 (free).
fn read_aspect(inputs: &[Input], aspect: Option<(usize, usize)>) -> Option<(i32, i32)> {
    let (a, b) = aspect?;
    let rw = inputs.get(a).and_then(|i| read_scalar(&i.value)).unwrap_or(0.0) as i32;
    let rh = inputs.get(b).and_then(|i| read_scalar(&i.value)).unwrap_or(0.0) as i32;
    Some((rw, rh))
}

// ------------------------------------------------------------- rect geometry

/// Convert a spec's four values to corner form `[x0, y0, x1, y1]`.
pub fn spec_to_corners(extent: RectExtent, v: [f32; 4]) -> [f32; 4] {
    match extent {
        RectExtent::OriginSize => [v[0], v[1], v[0] + v[2], v[1] + v[3]],
        RectExtent::TwoCorner => v,
    }
}

/// Inverse of [`spec_to_corners`].
pub fn corners_to_spec(extent: RectExtent, c: [f32; 4]) -> [f32; 4] {
    match extent {
        RectExtent::OriginSize => [c[0], c[1], c[2] - c[0], c[3] - c[1]],
        RectExtent::TwoCorner => c,
    }
}

/// Which corner coordinates `[x0, y0, x1, y1]` a handle moves.
pub fn handle_moves(h: RectHandle) -> [bool; 4] {
    match h {
        RectHandle::Body => [true, true, true, true],
        RectHandle::N => [false, true, false, false],
        RectHandle::S => [false, false, false, true],
        RectHandle::W => [true, false, false, false],
        RectHandle::E => [false, false, true, false],
        RectHandle::NW => [true, true, false, false],
        RectHandle::NE => [false, true, true, false],
        RectHandle::SE => [false, false, true, true],
        RectHandle::SW => [true, false, false, true],
    }
}

/// Which of the spec's inputs `[x, y, w, h]` a handle's gesture can reach.
///
/// Under [`RectExtent::OriginSize`] the size is `x1 - x0`, so moving *either* x
/// corner changes `width` while only the near one changes `x`; moving the whole
/// body shifts both corners equally and so leaves the size untouched.
///
/// When `aspect_locked`, every resize handle rewrites all four values: the
/// orthogonal side is derived from the ratio (and an edge recenters on the
/// free axis). Body still only translates.
pub fn spec_inputs_touched(h: RectHandle, extent: RectExtent) -> [bool; 4] {
    spec_inputs_touched_aspect(h, extent, false)
}

/// [`spec_inputs_touched`] with the aspect-lock rule applied when `locked`.
pub fn spec_inputs_touched_aspect(h: RectHandle, extent: RectExtent, locked: bool) -> [bool; 4] {
    if locked && h != RectHandle::Body {
        return [true, true, true, true];
    }
    let m = handle_moves(h);
    match extent {
        RectExtent::TwoCorner => m,
        RectExtent::OriginSize => {
            let body = h == RectHandle::Body;
            [m[0], m[1], !body && (m[0] || m[2]), !body && (m[1] || m[3])]
        }
    }
}

/// Apply a resize toward normalized position `to`, then clamp: into the unit
/// square, and to `min` without ever flipping — the dragged edge stops rather
/// than crossing the fixed one (`crop` cannot represent an inverted region).
pub fn resize_corners(mut c: [f32; 4], h: RectHandle, to: [f32; 2], min: [f32; 2]) -> [f32; 4] {
    let m = handle_moves(h);
    if m[0] {
        c[0] = to[0];
    }
    if m[1] {
        c[1] = to[1];
    }
    if m[2] {
        c[2] = to[0];
    }
    if m[3] {
        c[3] = to[1];
    }
    for v in c.iter_mut() {
        *v = v.clamp(0.0, 1.0);
    }
    let min = [min[0].clamp(0.0, 1.0), min[1].clamp(0.0, 1.0)];
    if m[0] {
        c[0] = (c[0]).min(c[2] - min[0]).max(0.0);
    }
    if m[2] {
        c[2] = (c[2]).max(c[0] + min[0]).min(1.0);
    }
    if m[1] {
        c[1] = (c[1]).min(c[3] - min[1]).max(0.0);
    }
    if m[3] {
        c[3] = (c[3]).max(c[1] + min[1]).min(1.0);
    }
    c
}

/// Aspect-locked resize: the box's *pixel* width/height stays `ratio`, not its
/// normalized width/height. A non-square image therefore produces a
/// non-square normalized box for a 1:1 lock.
///
/// Corner: opposite corner stays fixed; the pointer is projected onto the
/// aspect-correct size (whichever axis it overshoots), then the box shrinks
/// about that fixed corner to stay in the unit square.
/// Edge: the dragged edge moves; the orthogonal size is derived from the
/// ratio and the box recenters on the free axis so an east drag does not
/// walk the crop up or down.
pub fn resize_corners_aspect(
    c: [f32; 4],
    h: RectHandle,
    to: [f32; 2],
    min: [f32; 2],
    ratio: (i32, i32),
    dims: (u32, u32),
) -> [f32; 4] {
    let (rw, rh) = ratio;
    if rw <= 0 || rh <= 0 || dims.0 == 0 || dims.1 == 0 {
        return resize_corners(c, h, to, min);
    }
    // Desired normalized width/height: (rw / iw) / (rh / ih).
    let aspect_norm = (rw as f32 * dims.1 as f32) / (rh as f32 * dims.0 as f32);
    if !aspect_norm.is_finite() || aspect_norm <= 0.0 {
        return resize_corners(c, h, to, min);
    }
    let min = [
        min[0].clamp(0.0, 1.0),
        min[1].clamp(0.0, 1.0),
    ];
    // Minimum size that satisfies both the floor and the ratio.
    let min_w = min[0].max(min[1] * aspect_norm).min(1.0);
    let min_h = min[1].max(min[0] / aspect_norm).min(1.0);

    match h {
        RectHandle::Body => c,
        RectHandle::NW | RectHandle::NE | RectHandle::SE | RectHandle::SW => {
            resize_corner_aspect(c, h, to, aspect_norm, min_w, min_h)
        }
        RectHandle::N | RectHandle::S | RectHandle::W | RectHandle::E => {
            resize_edge_aspect(c, h, to, aspect_norm, min_w, min_h)
        }
    }
}

fn resize_corner_aspect(
    c: [f32; 4],
    h: RectHandle,
    to: [f32; 2],
    aspect_norm: f32,
    min_w: f32,
    min_h: f32,
) -> [f32; 4] {
    let (fx, fy, sx, sy) = match h {
        RectHandle::SE => (c[0], c[1], 1.0, 1.0),
        RectHandle::SW => (c[2], c[1], -1.0, 1.0),
        RectHandle::NE => (c[0], c[3], 1.0, -1.0),
        RectHandle::NW => (c[2], c[3], -1.0, -1.0),
        _ => return c,
    };
    // Signed proposals: positive = pointer is on the handle's side of the
    // fixed corner. Past the fixed corner we stop at the minimum rather than
    // flipping (crop cannot represent an inverted region).
    let pw = ((to[0] - fx) * sx).max(0.0);
    let ph = ((to[1] - fy) * sy).max(0.0);
    let (mut w, mut hgt) = if pw >= ph * aspect_norm {
        let w = pw;
        (w, w / aspect_norm)
    } else {
        let hgt = ph;
        (hgt * aspect_norm, hgt)
    };
    // Room from the fixed corner to the image edge in the handle's direction.
    let max_w = if sx > 0.0 { (1.0 - fx).max(0.0) } else { fx.max(0.0) };
    let max_h = if sy > 0.0 { (1.0 - fy).max(0.0) } else { fy.max(0.0) };
    if w > max_w {
        w = max_w;
        hgt = w / aspect_norm;
    }
    if hgt > max_h {
        hgt = max_h;
        w = hgt * aspect_norm;
    }
    w = w.max(min_w.min(max_w));
    hgt = w / aspect_norm;
    if hgt > max_h {
        hgt = max_h.max(min_h.min(max_h));
        w = hgt * aspect_norm;
    }
    [
        if sx > 0.0 { fx } else { fx - w },
        if sy > 0.0 { fy } else { fy - hgt },
        if sx > 0.0 { fx + w } else { fx },
        if sy > 0.0 { fy + hgt } else { fy },
    ]
}

fn resize_edge_aspect(
    c: [f32; 4],
    h: RectHandle,
    to: [f32; 2],
    aspect_norm: f32,
    min_w: f32,
    min_h: f32,
) -> [f32; 4] {
    let (mut x0, mut y0, mut x1, mut y1) = (c[0], c[1], c[2], c[3]);
    match h {
        RectHandle::E => {
            x1 = to[0].clamp(x0 + min_w, 1.0);
            let w = (x1 - x0).max(min_w);
            let hgt = w / aspect_norm;
            (y0, y1) = recenter_axis(y0, y1, hgt, min_h);
            x1 = x0 + w;
        }
        RectHandle::W => {
            x0 = to[0].clamp(0.0, x1 - min_w);
            let w = (x1 - x0).max(min_w);
            let hgt = w / aspect_norm;
            (y0, y1) = recenter_axis(y0, y1, hgt, min_h);
            x0 = x1 - w;
        }
        RectHandle::S => {
            y1 = to[1].clamp(y0 + min_h, 1.0);
            let hgt = (y1 - y0).max(min_h);
            let w = hgt * aspect_norm;
            (x0, x1) = recenter_axis(x0, x1, w, min_w);
            y1 = y0 + hgt;
        }
        RectHandle::N => {
            y0 = to[1].clamp(0.0, y1 - min_h);
            let hgt = (y1 - y0).max(min_h);
            let w = hgt * aspect_norm;
            (x0, x1) = recenter_axis(x0, x1, w, min_w);
            y0 = y1 - hgt;
        }
        _ => return c,
    }
    // If recentering overflowed the image, slide, then shrink if still too big.
    clamp_box_into_unit([x0, y0, x1, y1], aspect_norm)
}

/// Place a span of `size` on `axis`, centered on the old span, then slide
/// into `[0, 1]`. Does not shrink — [`clamp_box_into_unit`] does that after.
fn recenter_axis(a0: f32, a1: f32, size: f32, _min: f32) -> (f32, f32) {
    let mid = 0.5 * (a0 + a1);
    (mid - 0.5 * size, mid + 0.5 * size)
}

/// Slide a box into the unit square; if it is still larger than the image
/// along an axis, shrink about the centre keeping `aspect_norm`.
fn clamp_box_into_unit(mut c: [f32; 4], aspect_norm: f32) -> [f32; 4] {
    let mut w = c[2] - c[0];
    let mut h = c[3] - c[1];
    if w > 1.0 || h > 1.0 {
        if w > 1.0 {
            w = 1.0;
            h = w / aspect_norm;
        }
        if h > 1.0 {
            h = 1.0;
            w = h * aspect_norm;
        }
        w = w.min(1.0);
        h = h.min(1.0);
    }
    let x0 = c[0].clamp(0.0, (1.0 - w).max(0.0));
    let y0 = c[1].clamp(0.0, (1.0 - h).max(0.0));
    c[0] = x0;
    c[1] = y0;
    c[2] = x0 + w;
    c[3] = y0 + h;
    c
}

/// Translate the whole box by a normalized delta, **sliding** along the image
/// boundary rather than shrinking: the origin is clamped into `[0, 1 - size]`,
/// so a box pushed against an edge keeps its dimensions exactly.
pub fn move_corners(c: [f32; 4], delta: [f32; 2]) -> [f32; 4] {
    let w = c[2] - c[0];
    let h = c[3] - c[1];
    let x0 = (c[0] + delta[0]).clamp(0.0, (1.0 - w).max(0.0));
    let y0 = (c[1] + delta[1]).clamp(0.0, (1.0 - h).max(0.0));
    [x0, y0, x0 + w, y0 + h]
}

/// The smallest normalized box size, as the largest of three floors: an absolute
/// one, one source pixel (so the box can never be smaller than `crop` can
/// produce), and a screen-pixel one (so the grips stay separable when zoomed
/// out).
pub fn min_size(image_rect: Rect, dims: Option<(u32, u32)>) -> [f32; 2] {
    let screen = [
        MIN_RECT_PX / image_rect.width().max(1.0),
        MIN_RECT_PX / image_rect.height().max(1.0),
    ];
    let pixel = match dims {
        Some((w, h)) => [1.0 / w.max(1) as f32, 1.0 / h.max(1) as f32],
        None => [0.0, 0.0],
    };
    [
        MIN_RECT_NORM.max(screen[0]).max(pixel[0]).min(1.0),
        MIN_RECT_NORM.max(screen[1]).max(pixel[1]).min(1.0),
    ]
}

/// Screen rect of a normalized corner quad.
fn screen_rect(image_rect: Rect, c: [f32; 4]) -> Rect {
    Rect::from_min_max(
        norm_to_screen(image_rect, [c[0], c[1]]),
        norm_to_screen(image_rect, [c[2], c[3]]),
    )
}

/// Screen position of a resize grip on `r`.
fn grip_pos(r: Rect, h: RectHandle) -> Pos2 {
    match h {
        RectHandle::Body => r.center(),
        RectHandle::N => Pos2::new(r.center().x, r.top()),
        RectHandle::S => Pos2::new(r.center().x, r.bottom()),
        RectHandle::W => Pos2::new(r.left(), r.center().y),
        RectHandle::E => Pos2::new(r.right(), r.center().y),
        RectHandle::NW => r.left_top(),
        RectHandle::NE => r.right_top(),
        RectHandle::SE => r.right_bottom(),
        RectHandle::SW => r.left_bottom(),
    }
}

/// Interaction band along one edge of `r`, so an edge can be grabbed anywhere
/// along its length and not only at its midpoint grip.
fn edge_band(r: Rect, h: RectHandle) -> Rect {
    let e = EDGE_HIT_HALF;
    match h {
        RectHandle::N => Rect::from_min_max(
            Pos2::new(r.left(), r.top() - e),
            Pos2::new(r.right(), r.top() + e),
        ),
        RectHandle::S => Rect::from_min_max(
            Pos2::new(r.left(), r.bottom() - e),
            Pos2::new(r.right(), r.bottom() + e),
        ),
        RectHandle::W => Rect::from_min_max(
            Pos2::new(r.left() - e, r.top()),
            Pos2::new(r.left() + e, r.bottom()),
        ),
        _ => Rect::from_min_max(
            Pos2::new(r.right() - e, r.top()),
            Pos2::new(r.right() + e, r.bottom()),
        ),
    }
}

// ------------------------------------------------------------ value plumbing

/// Read an input's numeric value, whatever variant it holds.
pub fn read_scalar(value: &Value) -> Option<f32> {
    match value {
        Value::Decimal(v) => Some(*v),
        Value::Integer(v) => Some(*v as f32),
        _ => None,
    }
}

/// Write a number back in the input's own variant, so an integer position snaps
/// to whole pixels instead of silently changing the value's type.
pub fn write_scalar(existing: &Value, v: f32) -> Value {
    match existing {
        Value::Integer(_) => Value::Integer(v.round() as i32),
        _ => Value::Decimal(v),
    }
}

/// Apply the input's own clamp range, read from its widget settings. Inputs with
/// no range (an unbounded `DragValue`) are left alone on purpose.
pub fn clamp_for(settings: Option<&InputSettings>, v: f32) -> f32 {
    match settings {
        Some(InputSettings::Slider { range, clamp_to_range: true, .. }) => {
            v.clamp(range.0, range.1)
        }
        Some(InputSettings::DragValue { clamp: Some((lo, hi)), .. }) => v.clamp(*lo, *hi),
        _ => v,
    }
}

/// Clamp and round-trip one value through its input's type, so what is drawn is
/// exactly what will be committed.
fn quantize(input: &Input, v: f32) -> f32 {
    let clamped = clamp_for(input.settings.as_ref(), v);
    read_scalar(&write_scalar(&input.value, clamped)).unwrap_or(clamped)
}

fn read_pair(inputs: &[Input], idx: [usize; 2]) -> Option<[f32; 2]> {
    Some([read_scalar(&inputs[idx[0]].value)?, read_scalar(&inputs[idx[1]].value)?])
}

fn read_quad(inputs: &[Input], idx: [usize; 4]) -> Option<[f32; 4]> {
    let mut out = [0.0; 4];
    for (slot, &i) in idx.iter().enumerate() {
        out[slot] = read_scalar(&inputs[i].value)?;
    }
    Some(out)
}

fn quantize_pair(inputs: &[Input], idx: [usize; 2], v: [f32; 2]) -> [f32; 2] {
    [quantize(&inputs[idx[0]], v[0]), quantize(&inputs[idx[1]], v[1])]
}

fn quantize_quad(inputs: &[Input], idx: [usize; 4], v: [f32; 4]) -> [f32; 4] {
    let mut out = [0.0; 4];
    for (slot, &i) in idx.iter().enumerate() {
        out[slot] = quantize(&inputs[i], v[slot]);
    }
    out
}

// -------------------------------------------------------------------- chrome

/// A readout chip on an opaque plate, so it stays legible over any image.
fn draw_readout(ui: &egui::Ui, clip: Rect, at: Pos2, text: &str, theme: &Theme) {
    if text.is_empty() {
        return;
    }
    let colors = theme.get();
    let painter = ui.painter().with_clip_rect(clip);
    let font = egui::TextStyle::Small.resolve(ui.style());
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, colors.text_faint);
    let plate = Rect::from_min_size(at, galley.size() + Vec2::new(8.0, 4.0));
    painter.rect_filled(plate, 3.0, colors.panel_fill);
    painter.galley(plate.min + Vec2::new(4.0, 2.0), galley, colors.text_faint);
}

/// Why a gizmo has no handles: which of its inputs are driven from upstream.
///
/// Shown as a small plate rather than a hover tooltip, because a read-only
/// gizmo deliberately registers no interactive widget at all — there would be
/// nothing to hover, and adding one purely for the tooltip would put a dead
/// zone over the image's drag-to-pan background.
fn driven_note(ctx: &GizmoContext<'_>, indices: &[usize]) -> Option<String> {
    let driven: Vec<&str> = indices
        .iter()
        .filter(|&&i| ctx.inputs[i].connection.is_some())
        .map(|&i| ctx.inputs[i].name.as_str())
        .collect();
    (!driven.is_empty()).then(|| format!("driven upstream: {}", driven.join(", ")))
}

#[cfg(test)]
#[path = "spatial_overlay_tests.rs"]
mod tests;
