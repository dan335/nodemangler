//! Shared foreground placement for the compositing nodes.
//!
//! `blend` and `composite` both paste a foreground image into a background's
//! space at an integer pixel offset. This module adds scale and rotation to
//! that without either op's compositing loop having to know about them: it
//! **normalises** any (offset, scale, rotation) back into the one shape those
//! loops already understand — *an image plus an integer offset*.
//!
//! The point of that framing is the identity case. With scale 1 and rotation 0
//! [`place`] hands back the caller's own `Arc` and its own offset, so the
//! pre-transform behaviour is preserved byte for byte, with no resampling, no
//! allocation and no code path to keep in sync.
//!
//! ## Why scale runs through `resize`, not through the sampler
//! Folding the scale into the rotation's inverse map would make every downscale
//! a point-sample of a fraction of the source pixels — the classic sparkling
//! aliasing when a big photo is placed small. [`FloatImage::resize`] already
//! picks area averaging for real downscales and bilinear otherwise, so scaling
//! first and rotating a 1:1 image second gets that for free. It also makes the
//! rotation step exactly 1 source pixel per destination pixel, which is what
//! lets the edge coverage below be a plain one-pixel ramp.
//!
//! ## Alpha
//! Rotation interpolates between pixels, so it follows the house rule and
//! resamples in **premultiplied** space (see `float_image.rs`); `resize_premultiplied`
//! does the same for the scale step. A foreground without an alpha channel gains
//! one (1→2, 3→4 channels): the rotated quad no longer fills its bounding box,
//! and alpha is how the corners outside it are reported as absent.

use crate::float_image::FloatImage;
use rayon::prelude::*;
use std::sync::Arc;

/// Largest scaled foreground we will materialise, in pixels.
///
/// The scale inputs are deliberately not clamped (matching the `transform`
/// node), so a 26 MP raw at scale 16 would ask for a 6.6 gigapixel buffer. An
/// error naming the size beats an allocation failure.
const MAX_PLACED_PIXELS: u64 = 64 * 1024 * 1024;

/// Scale distance from 1.0 below which no resize is performed.
const SCALE_EPS: f32 = 1e-6;
/// Rotation, in degrees, below which no rotation pass is performed.
const ANGLE_EPS: f32 = 1e-4;

/// A foreground reduced to "this image, at this integer offset".
#[derive(Debug)]
pub struct Placed {
    /// The foreground as it should be composited, already scaled and rotated.
    pub image: Arc<FloatImage>,
    /// Where `image`'s top-left lands in the background, in background pixels.
    pub x: i32,
    pub y: i32,
    /// Per-pixel share of `image` that lies inside the placed quad, as a
    /// 1-channel image the same size as `image`. `None` when the placement was
    /// a pure offset/scale and every pixel is therefore inside.
    ///
    /// Only blend modes that ignore the foreground's alpha (Lerp) need this:
    /// for every other mode a zero alpha already means "leave the background
    /// alone", so the corners a rotation leaves outside the quad are
    /// self-cancelling. Lerp would otherwise fade the background towards
    /// transparent black in those corners.
    pub coverage: Option<Arc<FloatImage>>,
}

/// Resolve a foreground plus its placement into an image and an integer offset.
///
/// `Ok(None)` means nothing of the foreground lands on the background — a
/// degenerate scale, an empty image, or a placement entirely off-canvas. The
/// caller emits the background untouched.
///
/// `Err` is reserved for a scaled size too large to allocate; it carries a
/// message naming the size so the user can see which scale caused it.
pub fn place(
    fg: &Arc<FloatImage>,
    bg_dims: (u32, u32),
    x: i32,
    y: i32,
    scale_x: f32,
    scale_y: f32,
    rotation_deg: f32,
) -> Result<Option<Placed>, String> {
    let (fg_w, fg_h) = fg.dimensions();
    if fg_w == 0 || fg_h == 0 {
        return Ok(None);
    }

    // A non-finite parameter has no meaning as a placement, and every path
    // below would carry the NaN into a size or an angle. Falling back to the
    // neutral value keeps a stray value from blanking the node.
    let scale_x = if scale_x.is_finite() { scale_x } else { 1.0 };
    let scale_y = if scale_y.is_finite() { scale_y } else { 1.0 };
    // `rem_euclid` folds -90 to 270 and -0.0 to 0, so both ends of the range
    // are recognised as "no rotation".
    let turn = if rotation_deg.is_finite() { rotation_deg.rem_euclid(360.0) } else { 0.0 };
    let rotates = turn > ANGLE_EPS && turn < 360.0 - ANGLE_EPS;
    let scales = (scale_x - 1.0).abs() > SCALE_EPS || (scale_y - 1.0).abs() > SCALE_EPS;

    // The identity: hand back the caller's own Arc untouched.
    if !rotates && !scales {
        return Ok(Some(Placed { image: fg.clone(), x, y, coverage: None }));
    }

    let scaled = scale_image(fg, (fg_w, fg_h), scale_x, scale_y)?;
    let Some(scaled) = scaled else { return Ok(None) };

    if !rotates {
        return Ok(Some(Placed { image: scaled, x, y, coverage: None }));
    }

    Ok(rotate_into_background(&scaled, bg_dims, x, y, turn))
}

/// Apply the scale step, returning the caller's `Arc` when the rounded size is
/// unchanged. `Ok(None)` when the result would be empty.
fn scale_image(
    fg: &Arc<FloatImage>,
    fg_dims: (u32, u32),
    scale_x: f32,
    scale_y: f32,
) -> Result<Option<Arc<FloatImage>>, String> {
    let (fg_w, fg_h) = fg_dims;
    let sw = (fg_w as f32 * scale_x).round();
    let sh = (fg_h as f32 * scale_y).round();
    if !sw.is_finite() || !sh.is_finite() || sw < 1.0 || sh < 1.0 {
        return Ok(None);
    }
    if sw as u64 * sh as u64 > MAX_PLACED_PIXELS {
        return Err(format!(
            "scaled foreground would be {} x {} pixels, past the {} megapixel limit",
            sw as u64,
            sh as u64,
            MAX_PLACED_PIXELS / (1024 * 1024)
        ));
    }
    let (sw, sh) = (sw as u32, sh as u32);
    Ok(Some(if (sw, sh) == (fg_w, fg_h) {
        fg.clone()
    } else {
        Arc::new(fg.resize_premultiplied(sw, sh))
    }))
}

/// Rotate `src` about the centre of its placed rect and render the result into
/// a buffer clipped to the background.
///
/// Clipping here rather than in the callers is what bounds the allocation: a
/// foreground far larger than the background costs a buffer the size of the
/// background, not the size of the rotated bounding box.
fn rotate_into_background(
    src: &Arc<FloatImage>,
    bg_dims: (u32, u32),
    x: i32,
    y: i32,
    turn_deg: f32,
) -> Option<Placed> {
    let (sw, sh) = src.dimensions();
    let (bg_w, bg_h) = bg_dims;
    let (sin_t, cos_t) = turn_deg.to_radians().sin_cos();
    let (hw, hh) = (sw as f32 * 0.5, sh as f32 * 0.5);
    // Centre of the placed (unrotated) rect, in background pixels. Rotation is
    // about this point, so a pure rotation never moves the image's centre.
    let (cx, cy) = (x as f32 + hw, y as f32 + hh);

    // Half-extents of the rotated quad's axis-aligned bounding box.
    let bw = hw * cos_t.abs() + hh * sin_t.abs();
    let bh = hw * sin_t.abs() + hh * cos_t.abs();

    let x0 = ((cx - bw).floor() as i64).clamp(0, bg_w as i64);
    let y0 = ((cy - bh).floor() as i64).clamp(0, bg_h as i64);
    let x1 = ((cx + bw).ceil() as i64).clamp(0, bg_w as i64);
    let y1 = ((cy + bh).ceil() as i64).clamp(0, bg_h as i64);
    if x1 <= x0 || y1 <= y0 {
        return None; // entirely off the background
    }
    let (out_w, out_h) = ((x1 - x0) as u32, (y1 - y0) as u32);

    // A foreground with no alpha gains one: the rotated quad does not fill its
    // bounding box, and alpha is how the leftover corners say "not here".
    let src_ch = src.channels() as usize;
    let src_has_alpha = src.has_alpha();
    let out_ch = if src_has_alpha { src_ch } else { src_ch + 1 };
    let colors = out_ch - 1;

    // Interpolating straight alpha would drag the hidden colour of transparent
    // pixels into the rotated edges (see `float_image::premultiply_alpha`).
    let premul_owned;
    let sampled: &FloatImage = if src_has_alpha {
        premul_owned = src.premultiply_alpha();
        &premul_owned
    } else {
        src
    };

    let mut out = FloatImage::new(out_w, out_h, out_ch as u32);
    let mut cov = FloatImage::new(out_w, out_h, 1);
    let row_len = out_w as usize * out_ch;
    let cov_row_len = out_w as usize;

    out.as_raw_mut()
        .par_chunks_exact_mut(row_len)
        .zip(cov.as_raw_mut().par_chunks_exact_mut(cov_row_len))
        .enumerate()
        .for_each(|(row, (out_row, cov_row))| {
            // Background pixel centre, relative to the rotation pivot.
            let dy = y0 as f32 + row as f32 + 0.5 - cy;
            let mut sample = [0.0f32; 4];
            for (col, (out_px, cov_px)) in
                out_row.chunks_exact_mut(out_ch).zip(cov_row.iter_mut()).enumerate()
            {
                let dx = x0 as f32 + col as f32 + 0.5 - cx;
                // R(-θ): rotation matrices are orthonormal, so the inverse is
                // the transpose. Scale is already baked in, so this is the
                // whole inverse map.
                let u = dx * cos_t + dy * sin_t;
                let v = -dx * sin_t + dy * cos_t;
                // Extent coordinates in the scaled foreground: 0 is its left
                // edge, `sw` its right, pixel i's centre at i + 0.5.
                let ex = u + hw;
                let ey = v + hh;

                let c = edge_coverage(ex, sw as f32) * edge_coverage(ey, sh as f32);
                if c <= 0.0 {
                    continue; // stays zero: transparent and uncovered
                }
                // Clamp into the addressable range before sampling.
                // `bilinear_sample` clamps which *taps* it reads but keeps the
                // fraction from the raw coordinate, so a value a hair below 0
                // floors to -1 and blends ~100% of pixel 1 — the outermost
                // column of a rotation would show its neighbour instead of the
                // edge. Coverage above already handles being outside; here we
                // only want edge extension.
                let sx = (ex - 0.5).clamp(0.0, sw as f32 - 1.0);
                let sy = (ey - 0.5).clamp(0.0, sh as f32 - 1.0);
                sampled.bilinear_sample(sx, sy, &mut sample);
                for (i, o) in out_px[..colors].iter_mut().enumerate() {
                    *o = sample[i] * c;
                }
                out_px[colors] = if src_has_alpha { sample[colors] * c } else { c };
                *cov_px = c;
            }
        });

    out.unpremultiply_alpha();
    Some(Placed {
        image: Arc::new(out),
        x: x0 as i32,
        y: y0 as i32,
        coverage: Some(Arc::new(cov)),
    })
}

/// How much of a destination pixel at extent coordinate `e` falls inside an
/// axis spanning `[0, len]`.
///
/// A one-pixel linear ramp centred on each edge — half coverage exactly on the
/// edge, none half a pixel outside it. This is what antialiases a rotated
/// foreground's edges, including opaque ones that have no alpha to soften. It
/// is a one-*pixel* ramp because the scale step already ran, so one destination
/// pixel is one source pixel here.
fn edge_coverage(e: f32, len: f32) -> f32 {
    let near = (e + 0.5).clamp(0.0, 1.0);
    let far = (len - e + 0.5).clamp(0.0, 1.0);
    near.min(far)
}

#[cfg(test)]
#[path = "placement_tests.rs"]
mod tests;
