//! Morphological operators (erode/dilate family).
//!
//! `erode` hosts the shared `separable_morphology` helper used by the rest of
//! the family and by the fx glow nodes.

/// Morphological erosion: per-channel min in a square window.
pub mod erode;
/// Morphological dilation: per-channel max in a square window.
pub mod dilate;
/// Morphological opening: erode then dilate.
pub mod open;
/// Morphological closing: dilate then erode.
pub mod close;
/// Morphological gradient: dilation minus erosion (edge band).
pub mod morphological_gradient;
/// White top-hat: image minus its morphological opening (small bright details).
pub mod top_hat;
/// Black top-hat: morphological closing minus the image (small dark details).
pub mod black_hat;
/// Vector erode/dilate that picks a coherent neighbouring vector on normal-map-like fields.
pub mod vector_morphology;
/// Mask outline / stroke via dilate/erode difference.
pub mod outline;
