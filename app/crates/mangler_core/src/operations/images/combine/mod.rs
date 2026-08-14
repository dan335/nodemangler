//! Image compositing operations.
//!
//! Provides nodes for combining two images into one: `blit` for simple
//! pixel-copy overlay, and `blend` for blend-mode-aware compositing with
//! alpha masking and color-space-aware blending.

use crate::input::{Input, InputSettings};
use crate::value::Value;

/// Simple pixel overlay of a foreground image onto a background at a position.
pub mod blit;
/// Blend-mode compositing with alpha mask, amount control, and color space selection.
pub mod blend;
/// Pixel-by-pixel image comparison producing a greyscale difference map.
pub mod compare;
/// Shared scale/rotation placement of a foreground within a background.
pub mod placement;

/// The `scale x` / `scale y` / `rotation` inputs both compositing nodes append
/// after their `position x` / `position y` pair.
///
/// Declared once because the two nodes must agree: `mangler_core::gizmo` pins
/// these names, and the 2D preview's placement box drives whichever of the two
/// is selected through the same code.
///
/// Ranges match the `transform` node (unclamped sliders) so the same value
/// means the same thing whichever node the user reaches for. `placement::place`
/// carries the allocation guard that the missing clamp implies.
pub fn placement_inputs() -> Vec<Input> {
    vec![
        Input::new(
            "scale x".to_string(),
            Value::Decimal(1.0),
            Some(InputSettings::Slider { range: (0.01, 4.0), step_by: Some(0.01), clamp_to_range: false }),
            None,
        )
        .with_description("Horizontal scale of the foreground before it is placed; 1 = unchanged."),
        Input::new(
            "scale y".to_string(),
            Value::Decimal(1.0),
            Some(InputSettings::Slider { range: (0.01, 4.0), step_by: Some(0.01), clamp_to_range: false }),
            None,
        )
        .with_description("Vertical scale of the foreground before it is placed; 1 = unchanged."),
        Input::new(
            "rotation".to_string(),
            Value::Decimal(0.0),
            Some(InputSettings::Slider { range: (-360.0, 360.0), step_by: Some(0.1), clamp_to_range: false }),
            None,
        )
        .with_description(
            "Rotation of the foreground in degrees about its own centre; positive is clockwise.",
        ),
    ]
}

/// The shared help paragraph describing the placement controls.
pub const PLACEMENT_HELP: &str = "\n\nPlacement: the foreground's top-left lands at (position x, position y) in background pixels. `scale x` / `scale y` resize it first (1 = unchanged), and `rotation` turns it about its own centre, so scaling and rotating never move where the middle of the image sits. Drag all five directly on the 2D preview — the box's corners scale, its edges scale one axis, the knob above it rotates (hold Shift to snap to 15°), and dragging its inside moves it.\n\nScaling resamples through the same filter the resize node uses (area averaging for real downscales, bilinear otherwise) and rotation is bilinear with a one-pixel antialiased edge, so a rotated foreground has clean edges even without an alpha channel. At scale 1 and rotation 0 the foreground is copied with no resampling at all.";
