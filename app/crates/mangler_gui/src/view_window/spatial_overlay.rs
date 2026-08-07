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
//! pushes to the engine only on `commit`.
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

use eframe::egui::{self, Pos2, Rect, Stroke, Vec2};
use mangler_core::float_image::FloatImage;
use mangler_core::gizmo::{Gizmo, GizmoSpec, PixelBasis, RectExtent, SpatialSpace};
use mangler_core::input::{Input, InputSettings};
use mangler_core::value::Value;

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

/// The result of one overlay frame.
#[derive(Default)]
pub struct SpatialOverlayResponse {
    /// Inputs mutated this frame, as `(input_index, new_value)`. The caller
    /// mirrors each into its local node for instant feedback. **Empty on a
    /// drag's release frame** — the pointer no longer moved.
    pub changed: Vec<(usize, Value)>,
    /// A gesture completed; push to the engine.
    pub commit: bool,
    /// Which inputs the completed gesture could have moved, derived from the
    /// handle that reported the release (dragging the right edge of a crop box
    /// reaches `width` but never `y`). The caller sends one `SetInput` per
    /// index, reading its *accumulated local value* rather than `changed`.
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
    /// The backdrop's pixels, but **only** when the backdrop really is this
    /// node's spatial source. `None` suppresses the colour readout rather than
    /// reporting a sample from an unrelated image.
    pub sample_source: Option<&'a FloatImage>,
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
        let indices = spec.kind.inputs();
        // Defensive: a graph saved before the op gained an input can present a
        // shorter slice than the table expects. Skip rather than panic.
        if indices.iter().any(|&i| i >= ctx.inputs.len()) {
            continue;
        }
        // All-or-nothing: a gizmo whose inputs are driven upstream draws
        // read-only rather than vanishing, so it still explains itself.
        let editable =
            interactive && indices.iter().all(|&i| ctx.inputs[i].connection.is_none());

        let id = egui::Id::new(("spatial_overlay", leaf_id, spec_index));
        match spec.kind {
            Gizmo::Point { x, y, space } => {
                show_point(ui, id, image_rect, ctx, spec, [x, y], space, editable, theme, &mut out)
            }
            Gizmo::Rect { x, y, w, h, space, extent } => show_rect(
                ui,
                id,
                image_rect,
                ctx,
                spec,
                [x, y, w, h],
                space,
                extent,
                editable,
                theme,
                &mut out,
            ),
        }
    }
    out
}

// ---------------------------------------------------------------- point gizmo

/// A draggable crosshair. Clicking anywhere on the image jumps the point there,
/// which is what makes `sample pixel` usable as an eyedropper.
#[allow(clippy::too_many_arguments)]
fn show_point(
    ui: &mut egui::Ui,
    id: egui::Id,
    image_rect: Rect,
    ctx: &GizmoContext<'_>,
    spec: &GizmoSpec,
    idx: [usize; 2],
    space: SpatialSpace,
    editable: bool,
    theme: &Theme,
    out: &mut SpatialOverlayResponse,
) {
    let Some(mut values) = read_pair(ctx.inputs, idx) else { return };

    if editable {
        // Catcher first so the handle below wins the click; the catcher then
        // only fires for clicks that missed it.
        let catcher = handle::catcher(ui, id.with("catch"), image_rect);
        let center = norm_to_screen(image_rect, space.to_unit(ctx.image_dims, values));
        let grab = handle::handle(ui, id.with("pt"), center, HANDLE_HIT_HALF);

        let target = grab.drag_to.or(catcher.clicked_at);
        if let Some(to) = target {
            let unit = screen_to_norm(image_rect, to);
            let next = quantize_pair(ctx.inputs, idx, space.from_unit(ctx.image_dims, unit));
            if next != values {
                values = next;
                out.changed.push((idx[0], write_scalar(&ctx.inputs[idx[0]].value, values[0])));
                out.changed.push((idx[1], write_scalar(&ctx.inputs[idx[1]].value, values[1])));
            }
        }
        // A click both moves and finishes in one frame; a drag finishes on
        // release, when `drag_to` is None and `values` is already accumulated.
        if grab.commit || catcher.clicked_at.is_some() {
            out.commit = true;
            out.commit_inputs.extend_from_slice(&idx);
        }

        draw_crosshair(ui, image_rect, ctx, spec, values, space, grab.active, true, None, theme);
    } else {
        let note = driven_note(ctx, &idx);
        draw_crosshair(ui, image_rect, ctx, spec, values, space, false, false, note, theme);
    }
}

/// Paint the crosshair, its centre dot, and the readout chip.
#[allow(clippy::too_many_arguments)]
fn draw_crosshair(
    ui: &egui::Ui,
    image_rect: Rect,
    ctx: &GizmoContext<'_>,
    spec: &GizmoSpec,
    values: [f32; 2],
    space: SpatialSpace,
    active: bool,
    editable: bool,
    note: Option<String>,
    theme: &Theme,
) {
    let painter = ui.painter().with_clip_rect(image_rect);
    let center = norm_to_screen(image_rect, space.to_unit(ctx.image_dims, values));

    handle::draw_guide(&painter, image_rect, center, true, theme);
    handle::draw_guide(&painter, image_rect, center, false, theme);

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

    let text = with_note(point_readout(ctx, spec, values), note);
    draw_readout(ui, image_rect, center + Vec2::new(10.0, 10.0), &text, theme);
}

/// The crosshair's readout: pixel coordinates, plus the sampled colour when the
/// backdrop really is this node's source image.
fn point_readout(ctx: &GizmoContext<'_>, spec: &GizmoSpec, values: [f32; 2]) -> String {
    let Some((w, h)) = ctx.image_dims else {
        return format!("{} {:.3}, {:.3}", spec.label, values[0], values[1]);
    };
    // Mirror the operation's own addressing so the numbers name the pixel it
    // will actually read.
    let basis = match spec.kind.space() {
        SpatialSpace::Norm01 { basis } => basis,
    };
    let (px, py) = match basis {
        PixelBasis::Centres => (
            values[0].clamp(0.0, 1.0) * w.saturating_sub(1) as f32,
            values[1].clamp(0.0, 1.0) * h.saturating_sub(1) as f32,
        ),
        PixelBasis::Extent => (values[0] * w as f32, values[1] * h as f32),
    };

    let mut text = format!("{} x {:.0}  y {:.0}", spec.label, px, py);
    if let Some(img) = ctx.sample_source {
        let ch = img.channels() as usize;
        let mut buf = [0.0f32; 4];
        img.bilinear_sample(px, py, &mut buf[..ch.min(4)]);
        let (r, g, b, a) = match ch {
            1 => (buf[0], buf[0], buf[0], 1.0),
            2 => (buf[0], buf[0], buf[0], buf[1]),
            3 => (buf[0], buf[1], buf[2], 1.0),
            _ => (buf[0], buf[1], buf[2], buf[3]),
        };
        text.push_str(&format!("\n{r:.3} {g:.3} {b:.3} {a:.3}"));
    }
    text
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
    spec: &GizmoSpec,
    idx: [usize; 4],
    space: SpatialSpace,
    extent: RectExtent,
    editable: bool,
    theme: &Theme,
    out: &mut SpatialOverlayResponse,
) {
    let Some(mut values) = read_quad(ctx.inputs, idx) else { return };

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
                resize_corners(corners, h, screen_to_norm(image_rect, to), min)
            } else {
                corners
            };
            let next = quantize_quad(ctx.inputs, idx, corners_to_spec(extent, corners));
            if next != values {
                values = next;
                for (slot, &i) in idx.iter().enumerate() {
                    out.changed.push((i, write_scalar(&ctx.inputs[i].value, values[slot])));
                }
            }
        }
        if let Some(h) = released {
            out.commit = true;
            for (slot, touched) in spec_inputs_touched(h, extent).iter().enumerate() {
                if *touched {
                    out.commit_inputs.push(idx[slot]);
                }
            }
        }

        let active = moved.is_some();
        draw_rect(ui, image_rect, ctx, spec, values, space, extent, active, true, None, theme);
    } else {
        let note = driven_note(ctx, &idx);
        draw_rect(ui, image_rect, ctx, spec, values, space, extent, false, false, note, theme);
    }
}

/// Paint the box: an outside scrim so the kept region reads instantly, the
/// outline, rule-of-thirds guides, the grips, and the pixel readout.
#[allow(clippy::too_many_arguments)]
fn draw_rect(
    ui: &egui::Ui,
    image_rect: Rect,
    ctx: &GizmoContext<'_>,
    spec: &GizmoSpec,
    values: [f32; 4],
    _space: SpatialSpace,
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

    let text = with_note(rect_readout(ctx, spec, values, extent), note);
    draw_readout(ui, image_rect, r.left_top() + Vec2::new(4.0, 4.0), &text, theme);
}

/// The box's readout. When the convention allows it, this reproduces the crop
/// operation's own rounding so the numbers match the pixel dimensions the node
/// reports on its `width`/`height` outputs.
fn rect_readout(
    ctx: &GizmoContext<'_>,
    spec: &GizmoSpec,
    values: [f32; 4],
    extent: RectExtent,
) -> String {
    let basis = match spec.kind.space() {
        SpatialSpace::Norm01 { basis } => basis,
    };
    match (ctx.image_dims, extent, basis) {
        (Some(dims), RectExtent::OriginSize, PixelBasis::Extent) => {
            let (x, y, w, h) = crop_pixels(values, dims);
            format!("{} x {x}  y {y}  ·  {w} × {h} px", spec.label)
        }
        _ => format!(
            "{} {:.3}, {:.3}  ·  {:.3} × {:.3}",
            spec.label, values[0], values[1], values[2], values[3]
        ),
    }
}

/// Resolve an origin/size box to pixels exactly as `crop`'s `run()` does.
///
/// Mirrors the operation deliberately: the far edge is rounded from
/// `origin + size` rather than the size alone (so abutting crops share an edge),
/// and the region always keeps at least one pixel. Keep in sync with
/// `operations::images::transform::crop`.
pub fn crop_pixels(values: [f32; 4], dims: (u32, u32)) -> (i64, i64, i64, i64) {
    let iw = dims.0.max(1) as i64;
    let ih = dims.1.max(1) as i64;
    let x0 = ((values[0] * iw as f32).round() as i64).clamp(0, iw - 1);
    let y0 = ((values[1] * ih as f32).round() as i64).clamp(0, ih - 1);
    let x1 = (((values[0] + values[2]) * iw as f32).round() as i64).clamp(x0 + 1, iw);
    let y1 = (((values[1] + values[3]) * ih as f32).round() as i64).clamp(y0 + 1, ih);
    (x0, y0, x1 - x0, y1 - y0)
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
pub fn spec_inputs_touched(h: RectHandle, extent: RectExtent) -> [bool; 4] {
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

/// Append an explanatory note to a readout, on its own line.
fn with_note(readout: String, note: Option<String>) -> String {
    match note {
        Some(n) => format!("{readout}
{n}"),
        None => readout,
    }
}

/// Why a gizmo has no handles: which of its inputs are driven from upstream.
///
/// Appended to the readout rather than shown as a hover tooltip, because a
/// read-only gizmo deliberately registers no interactive widget at all — there
/// would be nothing to hover, and adding one purely for the tooltip would put a
/// dead zone over the image's drag-to-pan background.
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
