//! Painterly and graphic stylization filters.

/// Emboss effect using directional convolution.
pub mod emboss;
/// Kuwahara edge-preserving smoothing filter for a painterly look.
pub mod kuwahara;
/// Anisotropic Kuwahara (Kyprianidis 2009): structure-tensor-driven elliptical sampling.
pub mod anisotropic_kuwahara;
/// Toon / cel-shade filter: posterize plus edge overlay.
pub mod toon;
/// Oil paint stylization via intensity-histogram quantization.
pub mod oil_paint;
/// Halftone dot-screen stylization with rotated grid.
pub mod halftone;
/// Cross-hatch pen-and-ink stylization with layered hatch angles.
pub mod cross_hatch;
/// ASCII-style glyph stylization using 10 density-ordered 8×8 bitmaps.
pub mod ascii;
/// Mosaic / pixelate: block-average each cell to a single colour.
pub mod pixelate;
