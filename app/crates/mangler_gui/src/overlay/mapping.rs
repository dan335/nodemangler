//! Normalized `[0,1]²` ↔ screen coordinate mapping for overlay editors.
//!
//! Every overlay works in a unit square that is drawn into some screen `Rect` —
//! the displayed image for the 2D preview's curve overlay and spatial gizmos,
//! the square editing box for the settings panel's tone curve. The mapping is a
//! plain affine box transform, **y-down** (normalized `y = 0` is the rect's top
//! edge), matching `Curve`'s native coordinate convention.
//!
//! Pan and zoom never appear here: they are already baked into the `Rect` the
//! caller passes in (see `ImageViewer::get_rect`), which is what keeps these
//! functions pure and testable.

use eframe::egui::{Pos2, Rect, Vec2};

/// Map a normalized `[0,1]²` point to a screen position within `rect`.
pub fn norm_to_screen(rect: Rect, p: [f32; 2]) -> Pos2 {
    Pos2::new(rect.left() + p[0] * rect.width(), rect.top() + p[1] * rect.height())
}

/// Map a screen position to a normalized `[0,1]²` point within `rect`, clamped
/// to the unit square so a dragged point can't leave the canvas.
///
/// A degenerate (zero-size) rect maps everything to the origin rather than
/// dividing by zero.
pub fn screen_to_norm(rect: Rect, pos: Pos2) -> [f32; 2] {
    let p = screen_to_norm_unclamped(rect, pos);
    [p[0].clamp(0.0, 1.0), p[1].clamp(0.0, 1.0)]
}

/// Like [`screen_to_norm`] but without the unit-square clamp, for values that
/// legitimately live outside the canvas — bezier tangent tips, and pixel-space
/// gizmo handles whose input has no clamp range.
pub fn screen_to_norm_unclamped(rect: Rect, pos: Pos2) -> [f32; 2] {
    let x = if rect.width() > 0.0 { (pos.x - rect.left()) / rect.width() } else { 0.0 };
    let y = if rect.height() > 0.0 { (pos.y - rect.top()) / rect.height() } else { 0.0 };
    [x, y]
}

/// Convert a screen-space *displacement* into a normalized one.
///
/// Used for incremental drags (moving a whole crop box, rewriting a tangent
/// offset) where the absolute position would teleport the value to the pointer.
/// Being a displacement it is never clamped; a degenerate rect yields no motion.
pub fn screen_delta_to_norm(rect: Rect, delta: Vec2) -> [f32; 2] {
    let x = if rect.width() > 0.0 { delta.x / rect.width() } else { 0.0 };
    let y = if rect.height() > 0.0 { delta.y / rect.height() } else { 0.0 };
    [x, y]
}

/// A letterboxed square canvas centered in `view_rect`, used when there is no
/// image to draw over (a curve being edited or viewed on its own).
pub fn fallback_canvas_rect(view_rect: Rect) -> Rect {
    let size = view_rect.width().min(view_rect.height()) * 0.9;
    Rect::from_center_size(view_rect.center(), Vec2::splat(size))
}

#[cfg(test)]
#[path = "mapping_tests.rs"]
mod tests;
