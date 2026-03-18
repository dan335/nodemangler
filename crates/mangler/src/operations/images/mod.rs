//! Image operations for the node graph engine.
//!
//! This module organizes all image-related operations into subcategories:
//! inputs (loading/creating images), outputs (saving/exporting), transform
//! (geometric modifications), adjustments (color/tone corrections), noise
//! (procedural noise generators), combine (compositing), channels (RGBA
//! manipulation), shapes (vector shape generation), patterns (procedural
//! patterns), and PBR (physically-based rendering maps).

/// Image source operations: file, URL, clipboard, color fill, and gradient.
pub mod inputs;
/// Image export operations: saving to file or clipboard.
pub mod outputs;
/// Geometric transform operations: crop, resize, flip, rotate, warp, etc.
pub mod transform;
/// Image adjustment operations: blur, contrast, levels, curves, etc.
pub mod adjustments;
/// Procedural noise generation operations.
pub mod noise;
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