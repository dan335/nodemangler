//! Tests for the white balance operation (Planckian illuminant + Bradford CAT).

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn gray_arc() -> Arc<FloatImage> {
    Arc::new(FloatImage::from_pixel(4, 4, 4, &[0.5, 0.5, 0.5, 0.6]))
}

async fn run_arc(image: Arc<FloatImage>, temperature: f32, tint: f32) -> Arc<FloatImage> {
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: image, change_id: get_id() }, None, None),
        Input::new("temperature".to_string(), Value::Decimal(temperature), None, None),
        Input::new("tint".to_string(), Value::Decimal(tint), None, None),
    ];
    let out = OpImageAdjustmentWhiteBalance::run(&mut inputs).await.unwrap().responses[0].value.clone();
    let Value::Image { data, .. } = out else { panic!("expected image output") };
    data
}

/// Mid-grey run through the node, returned as an `[r, g, b, a]` pixel.
async fn balanced_gray(temperature: f32, tint: f32) -> [f32; 4] {
    let data = run_arc(gray_arc(), temperature, tint).await;
    let p = data.get_pixel(0, 0);
    [p[0], p[1], p[2], p[3]]
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageAdjustmentWhiteBalance::settings().name, "white balance");
    assert_eq!(OpImageAdjustmentWhiteBalance::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentWhiteBalance::create_outputs().len(), 1);
}

#[tokio::test]
async fn default_temperature_is_the_neutral() {
    let inputs = OpImageAdjustmentWhiteBalance::create_inputs();
    let Value::Decimal(t) = inputs[1].value else { panic!("expected decimal temperature") };
    assert_eq!(t, 6500.0);
    let Value::Decimal(tint) = inputs[2].value else { panic!("expected decimal tint") };
    assert_eq!(tint, 0.0);
}

#[tokio::test]
async fn neutral_settings_pass_the_buffer_through() {
    let src = gray_arc();
    let out = run_arc(src.clone(), 6500.0, 0.0).await;
    assert!(Arc::ptr_eq(&src, &out), "6500 K / 0 tint must short-circuit to the original Arc");
}

#[tokio::test]
async fn neutral_temperature_keeps_grey_grey() {
    // A hair off the neutral still has to be visually neutral: no jump when
    // the slider leaves its default.
    let p = balanced_gray(6501.0, 0.0).await;
    assert!((p[0] - p[2]).abs() < 0.005, "6501 K should stay neutral, got {:?}", p);
    assert!((p[0] - p[1]).abs() < 0.005, "6501 K should stay neutral, got {:?}", p);
}

#[tokio::test]
async fn high_kelvin_warms_the_image() {
    // Lightroom sense: declaring a bluer (higher Kelvin) light warms the result.
    let src = gray_arc();
    let out = run_arc(src.clone(), 8000.0, 0.0).await;
    assert!(!Arc::ptr_eq(&src, &out), "8000 K must actually change the image");
    let p = out.get_pixel(0, 0);
    assert!(p[0] > p[1] && p[1] > p[2], "8000 K should order r > g > b, got {:?}", p);
    assert!(p[0] > 0.5, "8000 K should raise red above the 0.5 input, got {}", p[0]);
    assert!(p[2] < 0.5, "8000 K should lower blue below the 0.5 input, got {}", p[2]);
}

#[tokio::test]
async fn low_kelvin_cools_the_image() {
    let p = balanced_gray(4000.0, 0.0).await;
    assert!(p[2] > p[1] && p[1] > p[0], "4000 K should order b > g > r, got {:?}", p);
    assert!(p[2] > 0.5, "4000 K should raise blue, got {}", p[2]);
    assert!(p[0] < 0.5, "4000 K should lower red, got {}", p[0]);
}

#[tokio::test]
async fn warmth_is_monotonic_in_temperature() {
    let mut previous = f32::NEG_INFINITY;
    for kelvin in [3000.0, 4000.0, 5000.0, 7000.0, 9000.0, 12000.0] {
        let p = balanced_gray(kelvin, 0.0).await;
        let warmth = p[0] - p[2];
        assert!(warmth > previous, "warmth must rise with Kelvin; {} K gave {}", kelvin, warmth);
        previous = warmth;
    }
}

#[tokio::test]
async fn tint_pushes_magenta_and_green_in_opposite_directions() {
    let magenta = balanced_gray(6500.0, 1.0).await;
    let green = balanced_gray(6500.0, -1.0).await;
    let magenta_bias = magenta[1] - 0.5 * (magenta[0] + magenta[2]);
    let green_bias = green[1] - 0.5 * (green[0] + green[2]);
    assert!(magenta_bias < 0.0, "positive tint should drop green below r/b, got {:?}", magenta);
    assert!(green_bias > 0.0, "negative tint should raise green above r/b, got {:?}", green);
}

#[tokio::test]
async fn alpha_is_preserved() {
    let p = balanced_gray(9000.0, 0.5).await;
    assert_eq!(p[3], 0.6, "alpha must pass through untouched");
}

#[tokio::test]
async fn grayscale_passthrough() {
    let src = Arc::new(FloatImage::from_pixel(2, 2, 1, &[0.3]));
    let out = run_arc(src.clone(), 3000.0, 1.0).await;
    assert!(Arc::ptr_eq(&src, &out), "1-channel images have no chroma to balance");

    let src2 = Arc::new(FloatImage::from_pixel(2, 2, 2, &[0.3, 1.0]));
    let out2 = run_arc(src2.clone(), 3000.0, 1.0).await;
    assert!(Arc::ptr_eq(&src2, &out2), "2-channel images have no chroma to balance");
}

#[tokio::test]
async fn output_stays_in_range_and_keeps_dimensions() {
    let out = run_arc(gray_arc(), 2000.0, -1.0).await;
    assert_eq!((out.width(), out.height()), (4, 4));
    for pixel in out.pixels() {
        for &v in pixel.iter().take(3) {
            assert!((0.0..=1.0).contains(&v), "channel out of range: {}", v);
        }
    }
}

#[test]
fn neutral_matrix_is_identity() {
    let m = white_balance_matrix(6500.0, 0.0);
    let identity = Mat3::IDENTITY;
    for c in 0..3 {
        for r in 0..3 {
            assert!(
                (m.col(c)[r] - identity.col(c)[r]).abs() < 1e-4,
                "neutral matrix should be identity, got {:?}",
                m
            );
        }
    }
}

#[test]
fn planckian_locus_matches_published_values() {
    // Spot checks against the Kang et al. (2002) locus: warmer light sits at
    // larger x, and 6500 K lands next to D65 (0.3127, 0.3290).
    let (x_warm, _) = planckian_xy(3000.0);
    let (x_neutral, y_neutral) = planckian_xy(6500.0);
    let (x_cool, _) = planckian_xy(12000.0);
    assert!(x_warm > x_neutral && x_neutral > x_cool, "x must fall as Kelvin rises");
    assert!((x_neutral - 0.3135).abs() < 0.005, "6500 K x was {}", x_neutral);
    assert!((y_neutral - 0.3237).abs() < 0.005, "6500 K y was {}", y_neutral);
}
