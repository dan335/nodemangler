//! Filter operations for the node graph.
//!
//! Provides convolution-based image filters for detecting edges, creating
//! emboss effects, and sharpening images.

/// Anisotropic Kuwahara (Kyprianidis 2009): structure-tensor-driven elliptical sampling.
pub mod anisotropic_kuwahara;
/// Bilateral edge-preserving smoothing using spatial + color similarity weights.
pub mod bilateral;
/// Edge detection using Sobel-based convolution.
pub mod edge_detect;
/// Emboss effect using directional convolution.
pub mod emboss;
/// Guided filter (He et al.): edge-preserving smoothing with cost independent of radius.
pub mod guided;
/// Kuwahara edge-preserving smoothing filter for a painterly look.
pub mod kuwahara;
/// Median filter for cartoon/blocky edge-preserving smoothing.
pub mod median;
/// Sharpening filter to enhance image detail.
pub mod sharpen;
/// Symmetric Nearest Neighbor edge-preserving smoothing.
pub mod snn;
/// Toon / cel-shade filter: posterize plus edge overlay.
pub mod toon;
/// Unsharp mask sharpening with configurable radius and threshold.
pub mod unsharpen;
