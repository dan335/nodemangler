//! Image operations for the node graph engine.
//!
//! This module organizes all image-related operations into subcategories:
//! inputs (loading/creating images), outputs (saving/exporting), transform
//! (geometric modifications), adjustments (color/tone corrections), blur
//! (various blur algorithms), filter (convolution-based filters), noise
//! (procedural noise generators), combine (compositing), channels (RGBA
//! manipulation), shapes (vector shape generation), patterns (procedural
//! patterns), and PBR (physically-based rendering maps).

/// Shared tone-curve helpers: LUT building/sampling for `InputSettings::ToneCurve` inputs.
pub mod tone_curve;
/// Image source operations: file, URL, clipboard, color fill, and gradient.
pub mod inputs;
/// Image export operations: saving to file or clipboard.
pub mod outputs;
/// Geometric transform operations: crop, resize, flip, rotate, warp, etc.
pub mod transform;
/// Image adjustment operations: contrast, levels, curves, etc.
pub mod adjustments;
/// Blur operations: Gaussian, directional, radial, slope, non-uniform.
pub mod blur;
/// Filter operations: edge detect, emboss, sharpen, unsharpen.
pub mod filter;
/// Procedural noise generation operations.
pub mod noise;
/// Physical-process simulation generators (cracks, erosion, weathering, ...).
pub mod simulation;
/// Image compositing operations: blit and blend.
pub mod combine;
/// RGBA channel manipulation: split, merge, and shuffle.
pub mod channels;
/// Vector shape generation operations.
pub mod shapes;
/// Procedural pattern generation operations.
pub mod patterns;
/// Physically-based rendering map operations.
pub mod pbr;
/// Mask-driven effect layers (drop shadow, glows).
pub mod fx;
/// Cast operations for converting values to images.
pub mod cast;
