//! Tests for the chromatic aberration operation.

use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::{EdgeMode, Value};
use std::sync::Arc;

async fn run(image: Arc<FloatImage>, red_cyan: f32, blue_yellow: f32, edge: EdgeMode) -> Arc<FloatImage> {
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: image, change_id: get_id() }, None, None),
        Input::new("red cyan".into(), Value::Decimal(red_cyan), None, None),
        Input::new("blue yellow".into(), Value::Decimal(blue_yellow), None, None),
        Input::new("edge mode".into(), Value::EdgeMode(edge), None, None),
    ];
    let r = OpImageTransformChromaticAberration::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    data.clone()
}

fn white_square_on_black(size: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(size, size, 4);
    let q = size / 4;
    for y in 0..size {
        for x in 0..size {
            let v = if x >= q && x < size - q && y >= q && y < size - q { 1.0 } else { 0.0 };
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    Arc::new(img)
}

#[tokio::test]
async fn both_zero_is_a_passthrough() {
    let src = white_square_on_black(16);
    let ptr_before = Arc::as_ptr(&src);
    let out = run(src.clone(), 0.0, 0.0, EdgeMode::Extend).await;
    assert_eq!(Arc::as_ptr(&out), ptr_before);
}

#[tokio::test]
async fn one_channel_is_a_passthrough() {
    let mut img = FloatImage::new(8, 8, 1);
    for y in 0..8 {
        for x in 0..8 {
            img.put_pixel(x, y, &[(x + y) as f32 / 16.0]);
        }
    }
    let src = Arc::new(img);
    let ptr_before = Arc::as_ptr(&src);
    let out = run(src.clone(), 0.8, -0.8, EdgeMode::Extend).await;
    assert_eq!(Arc::as_ptr(&out), ptr_before, "1-channel images have no chroma to separate");
}

#[tokio::test]
async fn two_channel_is_a_passthrough() {
    let mut img = FloatImage::new(8, 8, 2);
    for y in 0..8 {
        for x in 0..8 {
            img.put_pixel(x, y, &[(x + y) as f32 / 16.0, 1.0]);
        }
    }
    let src = Arc::new(img);
    let ptr_before = Arc::as_ptr(&src);
    let out = run(src.clone(), 0.8, -0.8, EdgeMode::Extend).await;
    assert_eq!(Arc::as_ptr(&out), ptr_before);
}

#[tokio::test]
async fn green_channel_is_identical_to_source() {
    let src = white_square_on_black(20);
    let out = run(src.clone(), 0.9, -0.9, EdgeMode::Extend).await;
    for (x, y, px) in out.enumerate_pixels() {
        let sg = src.get_pixel(x, y)[1];
        assert!((px[1] - sg).abs() < 1e-4, "green moved at ({x},{y}): {} vs {}", px[1], sg);
    }
}

#[tokio::test]
async fn alpha_is_preserved() {
    let mut img = FloatImage::new(12, 12, 4);
    for y in 0..12 {
        for x in 0..12 {
            img.put_pixel(x, y, &[0.5, 0.5, 0.5, if x < 6 { 1.0 } else { 0.4 }]);
        }
    }
    let src = Arc::new(img);
    let out = run(src.clone(), 0.9, 0.9, EdgeMode::Extend).await;
    for (x, y, px) in out.enumerate_pixels() {
        let sa = src.get_pixel(x, y)[3];
        assert!((px[3] - sa).abs() < 1e-4, "alpha moved at ({x},{y}): {} vs {}", px[3], sa);
    }
}

#[tokio::test]
async fn center_pixel_is_unchanged() {
    let src = white_square_on_black(17); // odd size: exact centre pixel
    let center_src = src.get_pixel(8, 8).to_vec();
    let out = run(src, 0.9, -0.9, EdgeMode::Extend).await;
    let center_out = out.get_pixel(8, 8);
    for c in 0..4 {
        assert!((center_out[c] - center_src[c]).abs() < 1e-3, "channel {c} moved at centre");
    }
}

#[tokio::test]
async fn alpha_gradient_keeps_colours_in_range() {
    // White (straight) RGB behind a horizontal alpha ramp. Each channel is
    // interpolated in premultiplied space at its *own* radial offset, so it
    // must be divided by the alpha at that same offset: dividing every channel
    // by the unshifted (green) alpha instead reads red where alpha is high and
    // divides it where alpha is low, pushing colours well past 1.
    let (w, h) = (32u32, 32u32);
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            img.put_pixel(x, y, &[1.0, 1.0, 1.0, x as f32 / (w - 1) as f32]);
        }
    }
    let out = run(Arc::new(img), 1.0, -1.0, EdgeMode::Extend).await;
    for (x, y, px) in out.enumerate_pixels() {
        if px[3] <= 0.05 {
            continue;
        }
        for c in 0..3 {
            assert!((px[c] - 1.0).abs() < 2e-3,
                "channel {c} at ({x},{y}) should stay at the source's straight white, got {px:?}");
        }
    }
}

#[tokio::test]
async fn transparent_hidden_colour_does_not_bleed() {
    // Left half opaque black, right half fully transparent white; the shifted
    // red/blue taps straddle the boundary. Premultiplied resampling must keep
    // the hidden white out of the visible pixels.
    let (w, h) = (48u32, 48u32);
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            if x < w / 2 {
                img.put_pixel(x, y, &[0.0, 0.0, 0.0, 1.0]);
            } else {
                img.put_pixel(x, y, &[1.0, 1.0, 1.0, 0.0]);
            }
        }
    }
    let out = run(Arc::new(img), 1.0, -1.0, EdgeMode::Extend).await;
    for (x, y, px) in out.enumerate_pixels() {
        if px[3] > 0.01 {
            assert!(px[0] < 0.05 && px[1] < 0.05 && px[2] < 0.05,
                "hidden colour bled into a visible pixel at ({x},{y}): {px:?}");
        }
    }
}

#[tokio::test]
async fn positive_red_cyan_separates_red_and_blue_at_an_edge() {
    // A white square on black, off-centre from the frame: positive red_cyan
    // and negative blue_yellow should pull red and blue apart measurably at
    // the square's edge (they start equal, coming from a grayscale square).
    let src = white_square_on_black(64);
    let out = run(src, 1.0, -1.0, EdgeMode::Extend).await;
    let mut max_rb_diff = 0.0f32;
    for (_, _, px) in out.enumerate_pixels() {
        max_rb_diff = max_rb_diff.max((px[0] - px[2]).abs());
    }
    assert!(max_rb_diff > 0.02, "expected measurable red/blue separation, got {max_rb_diff}");
}
