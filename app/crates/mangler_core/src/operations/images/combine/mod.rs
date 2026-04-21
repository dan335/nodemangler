//! Image compositing operations.
//!
//! Provides nodes for combining two images into one: `blit` for simple
//! pixel-copy overlay, and `blend` for blend-mode-aware compositing with
//! alpha masking and color-space-aware blending.

/// Simple pixel overlay of a foreground image onto a background at a position.
pub mod blit;
/// Blend-mode compositing with alpha mask, amount control, and color space selection.
pub mod blend;
/// Pixel-by-pixel image comparison producing a greyscale difference map.
pub mod compare;