//! Filter operations for the node graph.
//!
//! Provides convolution-based image filters for detecting edges, creating
//! emboss effects, and sharpening images.

/// Perona–Malik anisotropic diffusion: iterative edge-preserving smoothing.
pub mod anisotropic_diffusion;
/// Anisotropic Kuwahara (Kyprianidis 2009): structure-tensor-driven elliptical sampling.
pub mod anisotropic_kuwahara;
/// ASCII-style glyph stylization using 10 density-ordered 8×8 bitmaps.
pub mod ascii;
/// Bilateral edge-preserving smoothing using spatial + color similarity weights.
pub mod bilateral;
/// Canny multi-stage edge detector with non-max suppression and hysteresis.
pub mod canny;
/// Morphological closing: dilate then erode.
pub mod close;
/// Cross-hatch pen-and-ink stylization with layered hatch angles.
pub mod cross_hatch;
/// Morphological dilation: per-channel max in a square window.
pub mod dilate;
/// Difference of Gaussians / XDoG: stylized line-drawing filter.
pub mod dog;
/// Edge detection using Sobel-based convolution.
pub mod edge_detect;
/// Emboss effect using directional convolution.
pub mod emboss;
/// Morphological erosion: per-channel min in a square window.
pub mod erode;
/// Floyd–Steinberg error-diffusion dithering.
pub mod floyd_steinberg;
/// Guided filter (He et al.): edge-preserving smoothing with cost independent of radius.
pub mod guided;
/// Halftone dot-screen stylization with rotated grid.
pub mod halftone;
/// Highpass filter: subtract a blurred copy from the original.
pub mod highpass;
/// Kuwahara edge-preserving smoothing filter for a painterly look.
pub mod kuwahara;
/// Luminance-only highpass filter: preserves chroma.
pub mod luminance_highpass;
/// Median filter for cartoon/blocky edge-preserving smoothing.
pub mod median;
/// Non-Local Means denoising (Buades, Coll & Morel 2005).
pub mod non_local_means;
/// Oil paint stylization via intensity-histogram quantization.
pub mod oil_paint;
/// Morphological opening: erode then dilate.
pub mod open;
/// Ordered (Bayer-matrix) dithering to N quantization levels.
pub mod ordered_dither;
/// Mask outline / stroke via dilate/erode difference.
pub mod outline;
/// Mosaic / pixelate: block-average each cell to a single colour.
pub mod pixelate;
/// Sharpening filter to enhance image detail.
pub mod sharpen;
/// Symmetric Nearest Neighbor edge-preserving smoothing.
pub mod snn;
/// Toon / cel-shade filter: posterize plus edge overlay.
pub mod toon;
/// Unsharp mask sharpening with configurable radius and threshold.
pub mod unsharpen;
/// Vector erode/dilate that picks a coherent neighbouring vector on normal-map-like fields.
pub mod vector_morphology;
