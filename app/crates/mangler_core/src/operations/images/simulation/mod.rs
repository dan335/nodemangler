//! Physical-process simulation generators.
//!
//! Nodes in this category generate content by simulating the real-world
//! process that creates a material's look (crack propagation, hydraulic
//! erosion, diffusion-limited aggregation, percolation, ...) rather than by
//! layering random noise. The caustics noise node's refraction simulation is
//! the reference for the approach.
//!
//! Category convention: guidance-map image inputs (weakness, fuel, moisture,
//! height, ...) are OPTIONAL — when unconnected, the node generates an
//! internal fallback map from its seed, so every simulation node also works
//! standalone like a noise generator. Connecting a map makes the simulation
//! context-aware (e.g. cracks concentrate where the supplied weakness map is
//! dark).

/// Droplet-based hydraulic erosion: gullies, ridges, sediment fans.
pub mod hydraulic_erosion;

/// Returns true when an image-typed input still holds the 1x1 placeholder,
/// i.e. nothing is connected. Simulation nodes use this to decide between a
/// supplied guidance map and their internal seed-derived fallback.
pub(crate) fn is_unconnected(image: &crate::float_image::FloatImage) -> bool {
    image.width() <= 1 && image.height() <= 1
}

/// Resamples a guidance-map image to `w` x `h` and reduces it to a
/// single-channel [0, 1] luminance grid (Rec. 709 for RGB inputs, channel 0
/// for grayscale). Simulation nodes call this on connected guidance maps so
/// the sim always runs on a grid matching the output resolution.
pub(crate) fn guidance_map_to_grid(image: &crate::float_image::FloatImage, w: usize, h: usize) -> Vec<f64> {
    let resized = if image.width() as usize == w && image.height() as usize == h {
        None
    } else {
        Some(image.resize(w as u32, h as u32))
    };
    let source = resized.as_ref().unwrap_or(image);
    let channels = source.channels() as usize;
    let mut grid = vec![0.0_f64; w * h];
    for (i, pixel) in source.pixels().enumerate() {
        let v = if channels >= 3 {
            0.2126 * pixel[0] as f64 + 0.7152 * pixel[1] as f64 + 0.0722 * pixel[2] as f64
        } else {
            pixel[0] as f64
        };
        grid[i] = v.clamp(0.0, 1.0);
    }
    grid
}
