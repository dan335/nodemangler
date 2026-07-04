//! Caustics noise image generator.
//!
//! Produces a seamlessly tiling grayscale image of real refractive caustics:
//! the thin bright filament webs that sunlight forms on a pool floor after
//! passing through a rippled water surface.
//!
//! Unlike pattern-based approximations, this is a physical simulation. A
//! periodic Perlin-fBm heightfield models the water surface. A dense
//! supersampled grid of parallel vertical photons is cast down through the
//! surface; each photon is refracted by Snell's law (IOR 1.33) using the
//! surface normal from the heightfield gradient, travels to the floor at a
//! configurable depth, and is splatted into an accumulation buffer with
//! bilinear weights. Where the curved surface focuses photons, density piles
//! up into the characteristic connected filaments and cusps; where it
//! defocuses, the floor stays dark. The density map is normalized, softened by
//! a tiny blur, and tone-mapped with an exponential curve to tame the huge
//! dynamic range of real caustics.
//!
//! Tiles seamlessly: the heightfield uses integer Perlin periods and photon
//! landing positions wrap with `rem_euclid`. All randomness derives from the
//! seed via permutation tables, so output is fully deterministic.

use rayon::prelude::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use noise::permutationtable::NoiseHasher;
use crate::operations::images::noise::build_perm_tables;

/// Unclamped periodic 2D Perlin noise for the water surface.
///
/// Identical to the shared `periodic_perlin_2d` except that the final value
/// is not saturated to [-1, 1]. The shared version's clamp flat-tops the
/// strongest peaks, and a flat plateau on a refracting surface has a sharp
/// gradient crease along its boundary that scatters photons into visible
/// oval ring artifacts. A heightfield needs the smooth, unclipped value; the
/// slight overshoot past +/-1 is harmless because the amplitude is rescaled
/// anyway.
#[inline(always)]
fn periodic_perlin_2d_smooth(x: f64, y: f64, period: isize, hasher: &impl NoiseHasher) -> f64 {
    const SCALE_FACTOR: f64 = std::f64::consts::SQRT_2;

    let x0 = x.floor() as isize;
    let y0 = y.floor() as isize;
    let dx = x - x0 as f64;
    let dy = y - y0 as f64;

    // Wrap lattice corners with the period before hashing so the noise tiles.
    let wx0 = x0.rem_euclid(period);
    let wy0 = y0.rem_euclid(period);
    let wx1 = (x0 + 1).rem_euclid(period);
    let wy1 = (y0 + 1).rem_euclid(period);

    // Gradient dot product with the noise crate's 4-gradient set.
    let gradient = |hx: isize, hy: isize, px: f64, py: f64| -> f64 {
        match hasher.hash(&[hx, hy]) & 0b11 {
            0 => px + py,
            1 => -px + py,
            2 => px - py,
            3 => -px - py,
            _ => unreachable!(),
        }
    };

    let g00 = gradient(wx0, wy0, dx, dy);
    let g10 = gradient(wx1, wy0, dx - 1.0, dy);
    let g01 = gradient(wx0, wy1, dx, dy - 1.0);
    let g11 = gradient(wx1, wy1, dx - 1.0, dy - 1.0);

    // Quintic s-curve interpolation, matching the shared implementation.
    let quintic = |t: f64| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    let sx = quintic(dx);
    let sy = quintic(dy);

    let bottom = g00 + sy * (g01 - g00);
    let top = g10 + sy * (g11 - g10);
    (bottom + sx * (top - bottom)) * SCALE_FACTOR
}

/// Number of fBm octaves in the water-surface heightfield. Real water
/// surfaces are smooth, so a few octaves are enough.
const OCTAVES: usize = 3;
/// Per-octave amplitude falloff for the surface heightfield.
const PERSISTENCE: f64 = 0.35;
/// Index of refraction of water.
const IOR: f64 = 1.33;
/// Fixed-point scale for the atomic photon accumulator. Integer atomic adds
/// are associative, so the parallel splat is bit-exact deterministic.
const FIXED_POINT: f64 = 65536.0;

/// Operation that generates an underwater caustics image by photon splatting.
///
/// Builds a periodic Perlin-fBm heightfield as the water surface, then casts a
/// supersampled grid of vertical photons through it. Each photon is refracted
/// at the surface with Snell's law (IOR 1.33) using the normal from the
/// heightfield gradient (finite differences with wrap-around, scaled by
/// choppiness), lands on the floor `depth` tile-units below, and is
/// accumulated with bilinear splatting into a fixed-point atomic buffer.
/// The resulting photon density is normalized by its mean, lightly blurred to
/// remove splat grain, and tone-mapped with `1 - exp(-k * density)` followed
/// by a contrast gamma.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseCaustics {}

impl OpImageNoiseCaustics {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "caustics noise".to_string(),
            description: "Physically simulated water caustics: photons refracted through a rippled surface form thin bright filament webs on a dark floor.".to_string(),
            help: "Real refraction simulation, not a pattern lookup. A periodic Perlin-fBm heightfield models the rippled water surface. A dense supersampled grid of parallel vertical photons is cast down through it; each photon bends at the surface by Snell's law (IOR 1.33) using the normal from the heightfield gradient, then lands on the floor at the chosen depth. Landing positions are accumulated with bilinear splatting, and where the wavy surface focuses the light, photons pile up into the connected filament webs and bright cusps of real pool-floor caustics; defocused regions stay dark. The density map is normalized, lightly blurred to remove splat grain, and tone-mapped exponentially to compress the huge dynamic range.\n\nScale sets how many surface ripples fit across the tile (and therefore the size of the caustic web cells). Choppiness is the wave steepness: higher values bend light harder, giving denser, more tangled webs. Depth is the distance from surface to floor and acts as the focus control - the web sharpens toward thin cusped filaments as the floor approaches the focal distance of the ripples, then doubles and softens past it. Intensity scales the exponential tone map (higher burns the filaments brighter), and contrast applies a gamma that darkens the floor relative to the web.\n\nBest for pool floors, riverbeds, underwater lighting, shallow-sea ground, and light-through-glass effects. The output tiles seamlessly and is fully determined by the seed.".to_string(),
        }
    }

    /// Creates the default inputs for the caustics noise operation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for the water-surface heightfield; change to rearrange the light web."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("scale".to_string(), Value::Decimal(4.0), Some(InputSettings::DragValue { clamp: Some((1.0, 32.0)), speed: Some(0.1) }), None)
                .with_description("Number of surface ripples across the tile; higher values give smaller, denser caustic cells."),
            Input::new("choppiness".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { clamp: Some((0.0, 4.0)), speed: Some(0.01) }), None)
                .with_description("Wave steepness of the water surface; higher values refract light harder for denser, more tangled webs."),
            Input::new("depth".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { clamp: Some((0.05, 3.0)), speed: Some(0.01) }), None)
                .with_description("Floor distance below the surface in tile units; the focus control - filaments sharpen into cusps near the focal depth."),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { clamp: Some((0.05, 10.0)), speed: Some(0.05) }), None)
                .with_description("Exposure of the exponential tone map; higher values brighten the floor and burn the filaments toward white."),
            Input::new("contrast".to_string(), Value::Decimal(2.0), Some(InputSettings::DragValue { clamp: Some((0.1, 4.0)), speed: Some(0.01) }), None)
                .with_description("Gamma applied after tone mapping; higher values darken the floor relative to the bright web."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale caustic image: bright refracted filament webs on a dark floor."),
        ]
    }

    /// One pass of a wrapped separable [1, 2, 1]/4 blur; kills the residual
    /// grain of the photon splats without softening the filaments.
    fn blur_121_wrap(buf: &[f64], w: usize, h: usize) -> Vec<f64> {
        let mut horizontal = vec![0.0_f64; w * h];
        for y in 0..h {
            let row = y * w;
            for x in 0..w {
                let xm = (x + w - 1) % w;
                let xp = (x + 1) % w;
                horizontal[row + x] = 0.25 * buf[row + xm] + 0.5 * buf[row + x] + 0.25 * buf[row + xp];
            }
        }
        let mut vertical = vec![0.0_f64; w * h];
        for y in 0..h {
            let ym = ((y + h - 1) % h) * w;
            let yp = ((y + 1) % h) * w;
            let row = y * w;
            for x in 0..w {
                vertical[row + x] = 0.25 * horizontal[ym + x] + 0.5 * horizontal[row + x] + 0.25 * horizontal[yp + x];
            }
        }
        vertical
    }

    /// Generates a caustics image from the given inputs.
    ///
    /// 1. Evaluates a periodic Perlin-fBm heightfield on a supersampled grid
    /// 2. Refracts one vertical photon per grid point through the surface
    ///    (Snell's law, IOR 1.33) and lands it on the floor `depth` below
    /// 3. Splats photons bilinearly into a fixed-point atomic buffer
    ///    (integer adds keep the parallel accumulation deterministic)
    /// 4. Normalizes by mean density, blurs one pixel, and tone-maps with
    ///    `1 - exp(-k * density)` plus a contrast gamma
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let scale_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let choppiness_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let depth_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let intensity_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let contrast_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale) = scale_converted.unwrap() else { unreachable!() };
        let Value::Decimal(choppiness) = choppiness_converted.unwrap() else { unreachable!() };
        let Value::Decimal(depth) = depth_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };
        let Value::Decimal(contrast) = contrast_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let scale = (scale as f64).clamp(1.0, 32.0);
        let choppiness = (choppiness as f64).clamp(0.0, 4.0);
        let depth = (depth as f64).clamp(0.05, 3.0);
        let intensity = (intensity as f64).clamp(0.05, 10.0);
        let contrast = (contrast as f64).clamp(0.1, 4.0);

        let w = width as usize;
        let h = height as usize;
        let seed_u32 = seed as u32;

        // Supersample factor: photons per output pixel edge. 4x (16 photons
        // per pixel) up to 1 Mpx, 3x above that to keep large renders quick.
        let ss: usize = if w * h <= 1_048_576 { 4 } else { 3 };
        let sw = w * ss;
        let sh = h * ss;

        // Wave height amplitude in tile units. Scaling by 1/scale keeps the
        // wave *steepness* (and thus the character of the web) constant as
        // scale changes; the 0.03 base matches real amp/wavelength ratios.
        let amp = choppiness * 0.05 / scale;
        let eta = 1.0 / IOR;
        let base_period = scale.round().max(1.0) as isize;

        let perm_tables = build_perm_tables(seed_u32, OCTAVES);
        let perm_ref = &perm_tables;

        // 1. Water-surface heightfield on the photon grid: periodic Perlin
        // fBm with integer periods so the surface (and everything downstream)
        // tiles seamlessly. Stored un-amplified; the gradient is scaled later.
        let norm: f64 = (0..OCTAVES).map(|o| PERSISTENCE.powi(o as i32)).sum();
        let surface: Vec<f64> = (0..sh).into_par_iter().flat_map_iter(move |gy| {
            (0..sw).map(move |gx| {
                let u = (gx as f64 + 0.5) / sw as f64;
                let v = (gy as f64 + 0.5) / sh as f64;
                let mut sum = 0.0;
                let mut octave_amp = 1.0;
                let mut period = base_period;
                for hasher in perm_ref.iter() {
                    sum += octave_amp * periodic_perlin_2d_smooth(u * period as f64, v * period as f64, period, hasher);
                    octave_amp *= PERSISTENCE;
                    period *= 2;
                }
                sum / norm
            })
        }).collect();

        // 2 & 3. Refract one photon per grid point and splat it. Fixed-point
        // atomic adds are order-independent, so the parallel loop stays
        // bit-exact deterministic.
        let accum: Vec<AtomicU64> = (0..w * h).map(|_| AtomicU64::new(0)).collect();
        let surface_ref = &surface;
        let accum_ref = &accum;

        (0..sh).into_par_iter().for_each(move |gy| {
            let ym = ((gy + sh - 1) % sh) * sw;
            let yp = ((gy + 1) % sh) * sw;
            let row = gy * sw;
            let v = (gy as f64 + 0.5) / sh as f64;
            for gx in 0..sw {
                let xm = (gx + sw - 1) % sw;
                let xp = (gx + 1) % sw;

                // Surface gradient (per tile unit) via wrapped central
                // differences, scaled to real wave height by `amp`.
                let gx_h = (surface_ref[row + xp] - surface_ref[row + xm]) * sw as f64 * 0.5 * amp;
                let gy_h = (surface_ref[yp + gx] - surface_ref[ym + gx]) * sh as f64 * 0.5 * amp;

                // Unit surface normal from the gradient.
                let inv_len = 1.0 / (gx_h * gx_h + gy_h * gy_h + 1.0).sqrt();
                let nx = -gx_h * inv_len;
                let ny = -gy_h * inv_len;
                let nz = inv_len;

                // Snell refraction of the vertical ray d = (0, 0, -1).
                let cos_i = nz;
                let k = 1.0 - eta * eta * (1.0 - cos_i * cos_i);
                let m = eta * cos_i - k.sqrt();
                let tx = m * nx;
                let ty = m * ny;
                let tz = m * nz - eta; // always negative: ray continues down

                // Land on the floor `depth` below; wrap for seamless tiling.
                let u = (gx as f64 + 0.5) / sw as f64;
                let lu = (u + depth * tx / -tz).rem_euclid(1.0);
                let lv = (v + depth * ty / -tz).rem_euclid(1.0);

                // Bilinear splat over the four surrounding pixel centers.
                let fx = lu * w as f64 - 0.5;
                let fy = lv * h as f64 - 0.5;
                let x0f = fx.floor();
                let y0f = fy.floor();
                let sx = fx - x0f;
                let sy = fy - y0f;
                let x0 = (x0f as i64).rem_euclid(w as i64) as usize;
                let y0 = (y0f as i64).rem_euclid(h as i64) as usize;
                let x1 = (x0 + 1) % w;
                let y1 = (y0 + 1) % h;

                let w00 = (1.0 - sx) * (1.0 - sy);
                let w10 = sx * (1.0 - sy);
                let w01 = (1.0 - sx) * sy;
                let w11 = sx * sy;
                accum_ref[y0 * w + x0].fetch_add((w00 * FIXED_POINT + 0.5) as u64, Ordering::Relaxed);
                accum_ref[y0 * w + x1].fetch_add((w10 * FIXED_POINT + 0.5) as u64, Ordering::Relaxed);
                accum_ref[y1 * w + x0].fetch_add((w01 * FIXED_POINT + 0.5) as u64, Ordering::Relaxed);
                accum_ref[y1 * w + x1].fetch_add((w11 * FIXED_POINT + 0.5) as u64, Ordering::Relaxed);
            }
        });

        // 4. Photon density normalized by its mean, then a one-pixel blur to
        // remove splat grain, then exponential tone map plus contrast gamma.
        let density: Vec<f64> = accum.iter().map(|a| a.load(Ordering::Relaxed) as f64 / FIXED_POINT).collect();
        let mean = density.iter().sum::<f64>() / (w * h) as f64;
        let inv_mean = if mean > 0.0 { 1.0 / mean } else { 0.0 };
        let blurred = Self::blur_121_wrap(&density, w, h);

        let tone_k = 0.18 * intensity;
        let buffer: Vec<f64> = blurred.iter().map(|&d| {
            let v = 1.0 - (-tone_k * d * inv_mean).exp();
            v.powf(contrast).clamp(0.0, 1.0)
        }).collect();

        // Build a single-channel FloatImage from the computed pixel values
        let mut float_image = FloatImage::new(width as u32, height as u32, 1);
        for y in 0..h {
            for x in 0..w {
                let linear = buffer[y * w + x] as f32;
                let non_linear = crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb(linear);
                float_image.put_pixel(x as u32, y as u32, &[non_linear]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(float_image), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "caustics_tests.rs"]
mod tests;
