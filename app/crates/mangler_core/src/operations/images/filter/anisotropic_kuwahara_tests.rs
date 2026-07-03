//! Tests for the anisotropic Kuwahara filter.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn gradient_image(w: u32, h: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            let r = x as f32 / w.max(1) as f32;
            let g = y as f32 / h.max(1) as f32;
            img.put_pixel(x, y, &[r, g, 0.5, 1.0]);
        }
    }
    Arc::new(img)
}

fn default_inputs(img: Value) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), img, None, None),
        Input::new("radius".to_string(), Value::Integer(3), None, None),
        Input::new("sharpness".to_string(), Value::Decimal(8.0), None, None),
        Input::new("alpha".to_string(), Value::Decimal(1.0), None, None),
    ]
}

#[tokio::test]
async fn test_anisotropic_kuwahara_settings() {
    let s = OpImageAdjustmentAnisotropicKuwahara::settings();
    assert_eq!(s.name, "anisotropic kuwahara");
    assert_eq!(OpImageAdjustmentAnisotropicKuwahara::create_inputs().len(), 4);
    assert_eq!(OpImageAdjustmentAnisotropicKuwahara::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_anisotropic_kuwahara_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.784, 0.392, 0.196, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentAnisotropicKuwahara::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            // single-pixel image: every sample bilinear-clamps to the same pixel,
            // so the output must equal the input
            assert!((p[0] - 0.784).abs() < 1e-3);
            assert!((p[1] - 0.392).abs() < 1e-3);
            assert!((p[2] - 0.196).abs() < 1e-3);
            assert!((p[3] - 1.0).abs() < 1e-3);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_anisotropic_kuwahara_preserves_dimensions() {
    let mut inputs = default_inputs(Value::Image { data: gradient_image(16, 12), change_id: get_id() });
    let result = OpImageAdjustmentAnisotropicKuwahara::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 12);
            assert_eq!(data.channels(), 4);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_anisotropic_kuwahara_flat_image_is_identity() {
    // Uniform input — every sector has zero luminance variance, the variance
    // weighting averages all sector means together (which all equal the
    // constant), so output equals input.
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.3, 0.6, 0.9, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentAnisotropicKuwahara::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                assert!((pixel[0] - 0.3).abs() < 1e-3, "R drifted: {}", pixel[0]);
                assert!((pixel[1] - 0.6).abs() < 1e-3, "G drifted: {}", pixel[1]);
                assert!((pixel[2] - 0.9).abs() < 1e-3, "B drifted: {}", pixel[2]);
                assert!((pixel[3] - 1.0).abs() < 1e-3, "A drifted: {}", pixel[3]);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_anisotropic_kuwahara_edge_preserving() {
    // Sharp vertical black/white edge — well away from the boundary, the filter
    // must keep pixels near their original values (the low-variance sectors are
    // entirely on the matching side of the edge).
    let mut img = FloatImage::new(32, 32, 4);
    for y in 0..32 {
        for x in 0..32 {
            let v = if x < 16 { 0.0 } else { 1.0 };
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    let mut inputs = default_inputs(Value::Image { data: Arc::new(img), change_id: get_id() });
    let result = OpImageAdjustmentAnisotropicKuwahara::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let left = data.get_pixel(2, 16);
            let right = data.get_pixel(29, 16);
            assert!(left[0] < 0.05, "left leaked white: {}", left[0]);
            assert!(right[0] > 0.95, "right leaked black: {}", right[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_anisotropic_kuwahara_output_range() {
    let mut inputs = default_inputs(Value::Image { data: gradient_image(8, 8), change_id: get_id() });
    let result = OpImageAdjustmentAnisotropicKuwahara::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for &val in pixel {
                    assert!(val >= 0.0 && val <= 1.0, "out of range: {}", val);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

/// Brute-force reference: sequential per-pixel filter with a sector-major LUT
/// scanning ALL sectors per sampled offset (the pre-optimization structure).
/// Mirrors the op's math exactly: same structure tensor, same wedge weights,
/// f64 accumulation.
fn reference_filter(data: &FloatImage, radius: i32, q: f32, alpha: f32) -> Vec<f32> {
    let (width, height) = data.dimensions();
    let w = width as usize;
    let h = height as usize;
    let n = w * h;
    let ch = data.channels() as usize;
    let has_alpha = ch == 2 || ch == 4;
    let color_ch = if has_alpha { ch - 1 } else { ch };

    // luminance
    let luminance: Vec<f32> = (0..n).map(|i| {
        let p = data.get_pixel((i % w) as u32, (i / w) as u32);
        if color_ch >= 3 { 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2] } else { p[0] }
    }).collect();

    // structure tensor via Sobel on luminance
    let mut jxx = vec![0.0f32; n];
    let mut jyy = vec![0.0f32; n];
    let mut jxy = vec![0.0f32; n];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let at = |cx: i32, cy: i32| -> f32 {
                luminance[cy.clamp(0, h as i32 - 1) as usize * w + cx.clamp(0, w as i32 - 1) as usize]
            };
            let gx = -at(x - 1, y - 1) - 2.0 * at(x - 1, y) - at(x - 1, y + 1)
                + at(x + 1, y - 1) + 2.0 * at(x + 1, y) + at(x + 1, y + 1);
            let gy = -at(x - 1, y - 1) - 2.0 * at(x, y - 1) - at(x + 1, y - 1)
                + at(x - 1, y + 1) + 2.0 * at(x, y + 1) + at(x + 1, y + 1);
            let i = y as usize * w + x as usize;
            jxx[i] = gx * gx;
            jyy[i] = gy * gy;
            jxy[i] = gx * gy;
        }
    }
    let jxx_s = box_blur_2d(&jxx, w, h, 2);
    let jyy_s = box_blur_2d(&jyy, w, h, 2);
    let jxy_s = box_blur_2d(&jxy, w, h, 2);

    // orientation + anisotropy
    let mut phi = vec![0.0f32; n];
    let mut anis = vec![0.0f32; n];
    for i in 0..n {
        let (a, b, c) = (jxx_s[i], jxy_s[i], jyy_s[i]);
        let trace = a + c;
        let disc = ((a - c) * (a - c) + 4.0 * b * b).sqrt();
        let l1 = 0.5 * (trace + disc);
        let l2 = 0.5 * (trace - disc);
        phi[i] = 0.5 * (2.0 * b).atan2(a - c) + std::f32::consts::FRAC_PI_2;
        let denom = l1 + l2;
        anis[i] = if denom > 1e-8 { ((l1 - l2) / denom).clamp(0.0, 1.0) } else { 0.0 };
    }

    // sector-major LUT: weights[sector][offset]
    let diameter = (2 * radius + 1) as usize;
    let kernel_n = diameter * diameter;
    let sigma = radius as f32 * 0.5;
    let two_sigma_sq = 2.0 * sigma * sigma;
    let mut weights = vec![vec![0.0f32; kernel_n]; SECTORS];
    for (s, plane) in weights.iter_mut().enumerate() {
        let center_angle = (s as f32 + 0.5) * (std::f32::consts::TAU / SECTORS as f32);
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let off = ((dy + radius) as usize) * diameter + (dx + radius) as usize;
                let dist2 = (dx * dx + dy * dy) as f32;
                let radial = (-dist2 / two_sigma_sq).exp();
                if dx == 0 && dy == 0 {
                    plane[off] = radial / SECTORS as f32;
                    continue;
                }
                let theta = (dy as f32).atan2(dx as f32);
                let mut delta = theta - center_angle;
                while delta > std::f32::consts::PI { delta -= std::f32::consts::TAU; }
                while delta < -std::f32::consts::PI { delta += std::f32::consts::TAU; }
                let arg = (SECTORS as f32) * 0.25 * delta;
                plane[off] = if arg.abs() < std::f32::consts::FRAC_PI_2 {
                    let cc = arg.cos();
                    radial * cc * cc
                } else {
                    0.0
                };
            }
        }
    }

    // per-pixel filter, all sectors scanned per offset
    let mut out = Vec::with_capacity(n * ch);
    let mut sample = vec![0.0f32; ch];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let i = y as usize * w + x as usize;
            let a = anis[i];
            let scale_along = (alpha + a) / alpha;
            let scale_perp = alpha / (alpha + a);
            let cos_p = phi[i].cos();
            let sin_p = phi[i].sin();

            let mut sums = vec![0.0f64; SECTORS * ch];
            let mut sum_lum = [0.0f64; SECTORS];
            let mut sumsq_lum = [0.0f64; SECTORS];
            let mut wsum = [0.0f64; SECTORS];

            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let off = ((dy + radius) as usize) * diameter + (dx + radius) as usize;
                    let along_x = dx as f32 * scale_along;
                    let perp_y = dy as f32 * scale_perp;
                    let sx = x as f32 + along_x * cos_p - perp_y * sin_p;
                    let sy = y as f32 + along_x * sin_p + perp_y * cos_p;
                    data.bilinear_sample(sx, sy, &mut sample);
                    let s_lum = if color_ch >= 3 {
                        0.2126 * sample[0] + 0.7152 * sample[1] + 0.0722 * sample[2]
                    } else {
                        sample[0]
                    };
                    for s in 0..SECTORS {
                        let kw = weights[s][off] as f64;
                        if kw == 0.0 { continue; }
                        for c in 0..ch {
                            sums[s * ch + c] += sample[c] as f64 * kw;
                        }
                        sum_lum[s] += s_lum as f64 * kw;
                        sumsq_lum[s] += (s_lum as f64).powi(2) * kw;
                        wsum[s] += kw;
                    }
                }
            }

            let eps = 1e-8f64;
            let mut numer = vec![0.0f64; ch];
            let mut denom = 0.0f64;
            for s in 0..SECTORS {
                if wsum[s] < 1e-12 { continue; }
                let inv_w = 1.0 / wsum[s];
                let mean_lum = sum_lum[s] * inv_w;
                let var_lum = (sumsq_lum[s] * inv_w - mean_lum * mean_lum).max(0.0);
                let blend_w = 1.0 / (var_lum.powf(q as f64) + eps);
                for c in 0..ch {
                    numer[c] += sums[s * ch + c] * inv_w * blend_w;
                }
                denom += blend_w;
            }
            let inv_d = if denom > 0.0 { 1.0 / denom } else { 0.0 };
            for val in numer.iter().take(ch) {
                out.push((val * inv_d).clamp(0.0, 1.0) as f32);
            }
        }
    }
    out
}

#[tokio::test]
async fn test_anisotropic_kuwahara_matches_bruteforce_reference() {
    // Golden test: the offset-major LUT restructure must match a brute-force
    // per-pixel all-sector reference to well below visual precision.
    let (w, h) = (14u32, 11u32);
    let mut img = FloatImage::new(w, h, 4);
    let mut state: u32 = 0xCAFE_F00D;
    for y in 0..h {
        for x in 0..w {
            let mut px = [0.0f32; 4];
            for v in px.iter_mut().take(3) {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                *v = (state >> 8) as f32 / (1u32 << 24) as f32;
            }
            px[3] = 1.0;
            img.put_pixel(x, y, &px);
        }
    }
    let img = Arc::new(img);

    let (radius, q, alpha) = (3i32, 8.0f32, 1.0f32);
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img.clone(), change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(radius), None, None),
        Input::new("sharpness".to_string(), Value::Decimal(q), None, None),
        Input::new("alpha".to_string(), Value::Decimal(alpha), None, None),
    ];
    let result = OpImageAdjustmentAnisotropicKuwahara::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!("expected image") };

    let expected = reference_filter(&img, radius, q, alpha);
    assert_eq!(data.as_raw().len(), expected.len());
    let mut max_diff = 0.0f32;
    for (got, want) in data.as_raw().iter().zip(expected.iter()) {
        max_diff = max_diff.max((got - want).abs());
    }
    assert!(max_diff < 1e-3, "max abs diff vs brute-force reference: {}", max_diff);
}

#[tokio::test]
async fn test_anisotropic_kuwahara_radius_clamped() {
    // radius of 0 or 1 should be auto-clamped to the supported minimum (2)
    // and produce valid output rather than panicking.
    let mut inputs = default_inputs(Value::Image { data: gradient_image(4, 4), change_id: get_id() });
    inputs[1] = Input::new("radius".to_string(), Value::Integer(0), None, None);
    let result = OpImageAdjustmentAnisotropicKuwahara::run(&mut inputs).await;
    assert!(result.is_ok(), "radius=0 failed: {:?}", result.err());
}
