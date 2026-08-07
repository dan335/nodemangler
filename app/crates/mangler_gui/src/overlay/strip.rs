//! A small floating control strip pinned inside an overlay panel.
//!
//! Overlays draw over an image that fills the whole panel, so their controls
//! can't sit in normal layout flow — they are painted into a fixed rect in a
//! corner, on an opaque `panel_fill` plate so they stay legible against any
//! image. Used for the curve overlay's closed/interpolation controls and for
//! the gizmo overlay's "what am I looking at" caption.

use eframe::egui::{self, Rect, Vec2};

use crate::themes::theme::Theme;

/// Inset of the strip from the panel corner, in screen pixels.
const MARGIN: f32 = 8.0;
/// Inner padding of the strip's content area (horizontal, vertical).
const PADDING: Vec2 = Vec2::new(6.0, 2.0);

/// Draw a control strip of `size` pinned to the top-left of `view_rect`, and run
/// `add` inside it with a horizontally-centered layout.
///
/// The strip is painted with an opaque plate first, so whatever `add` draws sits
/// on a readable background regardless of the image underneath.
pub fn top_left<R>(
    ui: &mut egui::Ui,
    view_rect: Rect,
    size: Vec2,
    theme: &Theme,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let strip_rect = Rect::from_min_size(view_rect.left_top() + Vec2::splat(MARGIN), size);
    at(ui, strip_rect, theme, add)
}

/// Draw a control strip at an explicit rect. Prefer [`top_left`] unless the
/// caller needs a different anchor.
pub fn at<R>(
    ui: &mut egui::Ui,
    strip_rect: Rect,
    theme: &Theme,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    ui.painter().rect_filled(strip_rect, 4.0, theme.get().panel_fill);
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(strip_rect.shrink2(PADDING)),
        |ui| ui.horizontal_centered(add).inner,
    )
    .inner
}
