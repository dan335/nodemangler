//! Edge-preserving smoothing and denoising filters.

/// Median filter for cartoon/blocky edge-preserving smoothing.
pub mod median;
/// Bilateral edge-preserving smoothing using spatial + color similarity weights.
pub mod bilateral;
/// Guided filter (He et al.): edge-preserving smoothing with cost independent of radius.
pub mod guided;
/// Non-Local Means denoising (Buades, Coll & Morel 2005).
pub mod non_local_means;
/// Perona–Malik anisotropic diffusion: iterative edge-preserving smoothing.
pub mod anisotropic_diffusion;
/// Symmetric Nearest Neighbor edge-preserving smoothing.
pub mod snn;
