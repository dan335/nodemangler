//! Lightning noise image generator.
//!
//! Produces a grayscale image of branching bright filaments: lightning bolts,
//! electrical discharges, dendrites, and root systems. Unlike the closed-cell
//! networks of Voronoi crack noise, this is an open branching tree grown from
//! a seed-driven random walk.
//!
//! A CPU pre-pass grows each bolt as a jagged polyline from the top edge to
//! the bottom edge, recursively forking child branches with reduced length,
//! width, and intensity. The collected segments are then rendered per pixel
//! as a distance field: a hard bright core inside the segment width plus an
//! exponential glow halo, MAX-combined across segments so crossings stay crisp.
//!
//! This node does NOT tile seamlessly — bolts start and end at the image edges.

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
use std::time::Instant;

/// Hard cap on the total number of generated segments, so extreme branch
/// settings cannot explode the pre-pass or the per-pixel loop.
const MAX_SEGMENTS: usize = 4000;

/// One rendered bolt segment in UV space, with its core half-width and
/// peak intensity.
#[derive(Debug, Clone, Copy)]
struct Segment {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    half_width: f64,
    intensity: f64,
}

/// Deterministic sequential random number generator.
///
/// A PCG-style stream: each call advances a wrapping LCG state, then mixes it
/// with the same XOR-shift finalizer used by the cell hashes. All lightning
/// randomness derives from the node seed through this — no system entropy.
struct SeqRng {
    state: u32,
}

impl SeqRng {
    /// Creates a generator whose stream is determined entirely by `seed`.
    fn new(seed: u32) -> Self {
        Self { state: seed.wrapping_mul(747796405).wrapping_add(2891336453) }
    }

    /// Returns the next pseudo-random f64 in [0, 1).
    fn next(&mut self) -> f64 {
        self.state = self.state.wrapping_mul(747796405).wrapping_add(2891336453);
        let mut h = self.state;
        h = h.wrapping_mul(h ^ (h >> 16));
        h = h.wrapping_mul(h ^ (h >> 16));
        (h & 0x00FFFFFF) as f64 / 0x01000000 as f64
    }
}

/// Smoothstep interpolation between two edges.
#[inline(always)]
fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Distance from a point to a line segment, in UV units.
#[inline(always)]
fn dist_to_segment(px: f64, py: f64, seg: &Segment) -> f64 {
    let vx = seg.x1 - seg.x0;
    let vy = seg.y1 - seg.y0;
    let wx = px - seg.x0;
    let wy = py - seg.y0;
    let len_sq = vx * vx + vy * vy;
    let t = if len_sq > 0.0 { ((wx * vx + wy * vy) / len_sq).clamp(0.0, 1.0) } else { 0.0 };
    let dx = wx - t * vx;
    let dy = wy - t * vy;
    (dx * dx + dy * dy).sqrt()
}

/// Operation that generates a branching lightning noise image.
///
/// The pre-pass grows each bolt as a polyline random walk: the heading is
/// perturbed at every vertex by jaggedness and pulled back toward the base
/// direction so the bolt stays coherent. At random vertices (probability tied
/// to the branches input) a child polyline forks off at a 20-60 degree offset
/// with 40-60% of the remaining length and 0.6x width and intensity, down to
/// `depth` levels. Per pixel, the distance to the nearest segment drives a
/// bright smoothstep core plus an exponential glow, MAX-combined.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseLightning {}

impl OpImageNoiseLightning {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "lightning noise".to_string(),
            description: "Branching bright filaments: lightning bolts, electrical discharges, dendrites, and root systems. Does not tile.".to_string(),
            help: "Grows an open branching tree, unlike the closed cells of voronoi crack noise. Each bolt starts at a random point on the top edge and random-walks down to the bottom edge in a few dozen segments; jaggedness scales how far the heading wanders at each step. At random vertices the bolt forks a child branch at a 20-60 degree offset with reduced length, and each level of forking shrinks width and brightness by 0.6x, down to the depth limit. The branches input sets how eagerly forks are spawned.\n\nAll segments are rendered as a distance field: pixels within the bolt width get a hard bright core, and a soft exponential halo controlled by glow extends about 8x further. Contributions MAX-combine so crossings stay crisp. Bolts spreads several independent strikes across the width.\n\nBest for lightning, electric arcs, Tesla-coil discharges, dendrites, cracks in glowing material, and root or vein systems (flip vertically for roots). Note: this node does NOT tile seamlessly — bolts begin and end at the image edges.".to_string(),
        }
    }

    /// Creates the default inputs for the lightning noise operation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for bolt paths and branching; change to strike somewhere else."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("bolts".to_string(), Value::Integer(1), Some(InputSettings::Slider { range: (1.0, 8.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of independent bolts spread across the image width."),
            Input::new("depth".to_string(), Value::Integer(3), Some(InputSettings::Slider { range: (1.0, 5.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Maximum branching levels; 1 is a bare bolt, 5 gives dense dendritic trees."),
            Input::new("branches".to_string(), Value::Decimal(0.6), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How eagerly the bolt forks child branches; 0 spawns none, 1 forks constantly."),
            Input::new("jaggedness".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How far the bolt heading wanders at each step; 0 is nearly straight, 1 is erratic."),
            Input::new("bolt_width".to_string(), Value::Decimal(0.15), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Core filament width; the full range maps from hairline to thick plasma channels."),
            Input::new("glow".to_string(), Value::Decimal(0.4), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Strength of the soft halo around each filament; 0 leaves only the hard core."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Grayscale image of branching bright filaments with soft glow. Does not tile seamlessly."),
        ]
    }

    /// Grows one polyline (and, recursively, its child branches) into `segments`.
    ///
    /// Walks `length` UV units from (`x`, `y`) along `angle` (0 = straight
    /// down; dx = sin, dy = cos), perturbing the heading each step by
    /// jaggedness and pulling it back toward the base angle so the bolt stays
    /// coherent. At each vertex a child branch may fork with probability tied
    /// to `branches`, at a 20-60 degree offset, with 40-60% of the remaining
    /// length and 0.6x width and intensity.
    #[allow(clippy::too_many_arguments)]
    fn grow(
        rng: &mut SeqRng,
        mut x: f64,
        mut y: f64,
        angle: f64,
        length: f64,
        half_width: f64,
        intensity: f64,
        depth_left: i32,
        branches: f64,
        jaggedness: f64,
        segments: &mut Vec<Segment>,
    ) {
        if segments.len() >= MAX_SEGMENTS {
            return;
        }

        // Segment count scales with length: the full-height main bolt walks
        // in roughly 24-48 steps, shorter branches in proportionally fewer.
        let steps = (12.0 + length * 30.0).round().clamp(4.0, 48.0) as usize;
        let step_len = length / steps as f64;
        let mut heading = angle;

        for i in 0..steps {
            if segments.len() >= MAX_SEGMENTS {
                return;
            }

            // Random-walk the heading, then relax it toward the base angle so
            // the bolt wanders without curling away entirely.
            heading += (rng.next() - 0.5) * jaggedness * 1.6;
            heading = angle + (heading - angle) * 0.85;

            let nx = x + heading.sin() * step_len;
            let ny = y + heading.cos() * step_len;
            segments.push(Segment { x0: x, y0: y, x1: nx, y1: ny, half_width, intensity });
            x = nx;
            y = ny;

            // Fork a child branch at this vertex with probability tied to `branches`.
            if depth_left > 1 && rng.next() < branches * 0.15 {
                let remaining = length - (i + 1) as f64 * step_len;
                let child_len = remaining * (0.4 + rng.next() * 0.2);
                if child_len > 0.02 {
                    let side = if rng.next() < 0.5 { 1.0 } else { -1.0 };
                    let child_angle = heading + side * (20.0 + rng.next() * 40.0).to_radians();
                    Self::grow(
                        rng,
                        x,
                        y,
                        child_angle,
                        child_len,
                        half_width * 0.6,
                        intensity * 0.6,
                        depth_left - 1,
                        branches,
                        jaggedness,
                        segments,
                    );
                }
            }
        }
    }

    /// Generates a lightning noise image from the given inputs.
    ///
    /// Pre-pass: grows `bolts` independent bolts from the top edge to the
    /// bottom edge, collecting all segments (with per-segment width and
    /// intensity) into a Vec. Per pixel (parallelized per row): computes the
    /// distance to every segment and MAX-combines a smoothstep core inside
    /// the segment width with an exponential glow halo around it.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let bolts_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let depth_converted = convert_input(inputs, 4, ValueType::Integer, &mut input_errors);
        let branches_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let jaggedness_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let bolt_width_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let glow_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(bolts) = bolts_converted.unwrap() else { unreachable!() };
        let Value::Integer(depth) = depth_converted.unwrap() else { unreachable!() };
        let Value::Decimal(branches) = branches_converted.unwrap() else { unreachable!() };
        let Value::Decimal(jaggedness) = jaggedness_converted.unwrap() else { unreachable!() };
        let Value::Decimal(bolt_width) = bolt_width_converted.unwrap() else { unreachable!() };
        let Value::Decimal(glow) = glow_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let bolts = bolts.clamp(1, 8);
        let depth = depth.clamp(1, 5);
        let branches = (branches as f64).clamp(0.0, 1.0);
        let jaggedness = (jaggedness as f64).clamp(0.0, 1.0);
        let bolt_width = (bolt_width as f64).clamp(0.0, 1.0);
        let glow = (glow as f64).clamp(0.0, 1.0);

        let w = width as usize;
        let h = height as usize;

        // Map the 0-1 width slider to a core half-width in UV units:
        // hairline (0.002) up to thick plasma channels (0.02).
        let core_half_width = 0.002 + bolt_width * 0.018;

        // CPU pre-pass: grow all bolts into a flat segment list.
        let mut rng = SeqRng::new(seed as u32);
        let mut segments: Vec<Segment> = Vec::new();
        for bolt in 0..bolts {
            // Spread bolt start points across the width, with a little jitter.
            let start_x = (bolt as f64 + 0.5) / bolts as f64 + (rng.next() - 0.5) * 0.6 / bolts as f64;
            // Long enough to reach the bottom edge despite the wandering path.
            let length = 1.1 + rng.next() * 0.15;
            Self::grow(
                &mut rng,
                start_x,
                0.0,
                0.0,
                length,
                core_half_width,
                1.0,
                depth,
                branches,
                jaggedness,
                &mut segments,
            );
        }

        let segments_ref = &segments;

        // Per-pixel distance field (parallelized per row): MAX-combine a hard
        // smoothstep core with an exponential glow halo across all segments.
        let buffer: Vec<f64> = (0..h).into_par_iter().flat_map_iter(move |py| {
            (0..w).map(move |px| {
                let u = px as f64 / w as f64;
                let v = py as f64 / h as f64;

                let mut max_val = 0.0_f64;

                for seg in segments_ref {
                    let hw = seg.half_width;
                    let glow_radius = hw * 8.0;
                    // Beyond ~6 glow radii the halo is negligible; a cheap
                    // bounding-box check prunes far segments before the exact
                    // distance is computed.
                    let cutoff = glow_radius * 6.0;
                    let min_x = seg.x0.min(seg.x1) - cutoff;
                    let max_x = seg.x0.max(seg.x1) + cutoff;
                    let min_y = seg.y0.min(seg.y1) - cutoff;
                    let max_y = seg.y0.max(seg.y1) + cutoff;
                    if u < min_x || u > max_x || v < min_y || v > max_y {
                        continue;
                    }

                    let dist = dist_to_segment(u, v, seg);
                    if dist > cutoff {
                        continue;
                    }

                    // Hard bright core inside the segment width.
                    let core = seg.intensity * (1.0 - smoothstep(hw * 0.5, hw, dist));
                    // Soft exponential halo around it.
                    let halo = glow * seg.intensity * (-dist / glow_radius).exp();

                    let contribution = core.max(halo);
                    if contribution > max_val {
                        max_val = contribution;
                    }
                }

                max_val.clamp(0.0, 1.0)
            })
        }).collect();

        // No normalization — segment intensity directly controls brightness
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
#[path = "lightning_tests.rs"]
mod tests;
