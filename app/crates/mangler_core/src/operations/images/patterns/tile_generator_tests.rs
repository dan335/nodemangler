//! Tests for the deterministic tile generator.

use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn square_pattern() -> Arc<FloatImage> {
    // Fully white small pattern so max-composite with black background creates
    // visible stamps.
    Arc::new(FloatImage::from_pixel(8, 8, 4, &[1.0, 1.0, 1.0, 1.0]))
}

#[tokio::test]
async fn output_matches_requested_dimensions() {
    let mut inputs = vec![
        Input::new("pattern".into(), Value::Image { data: square_pattern(), change_id: get_id() }, None, None),
        Input::new("width".into(), Value::Integer(64), None, None),
        Input::new("height".into(), Value::Integer(48), None, None),
        Input::new("count_x".into(), Value::Integer(4), None, None),
        Input::new("count_y".into(), Value::Integer(3), None, None),
        Input::new("scale".into(), Value::Decimal(1.0), None, None),
        Input::new("rotation".into(), Value::Decimal(0.0), None, None),
        Input::new("row offset".into(), Value::Decimal(0.0), None, None),
        Input::new("col offset".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImagePatternTileGenerator::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert_eq!(data.dimensions(), (64, 48));
    assert_eq!(data.channels(), 4);
}

#[tokio::test]
async fn determinism_two_identical_runs_match() {
    // No RNG, so two runs with the same inputs must be bit-identical.
    fn run_inputs() -> Vec<Input> {
        vec![
            Input::new("pattern".into(), Value::Image { data: square_pattern(), change_id: get_id() }, None, None),
            Input::new("width".into(), Value::Integer(32), None, None),
            Input::new("height".into(), Value::Integer(32), None, None),
            Input::new("count_x".into(), Value::Integer(3), None, None),
            Input::new("count_y".into(), Value::Integer(3), None, None),
            Input::new("scale".into(), Value::Decimal(0.8), None, None),
            Input::new("rotation".into(), Value::Decimal(15.0), None, None),
            Input::new("row offset".into(), Value::Decimal(0.25), None, None),
            Input::new("col offset".into(), Value::Decimal(0.0), None, None),
        ]
    }
    let mut a = run_inputs();
    let ra = OpImagePatternTileGenerator::run(&mut a).await.unwrap();
    let mut b = run_inputs();
    let rb = OpImagePatternTileGenerator::run(&mut b).await.unwrap();
    let Value::Image { data: da, .. } = &ra.responses[0].value else { panic!() };
    let Value::Image { data: db, .. } = &rb.responses[0].value else { panic!() };
    for y in 0..32 {
        for x in 0..32 {
            let a = da.get_pixel(x, y);
            let b = db.get_pixel(x, y);
            for c in 0..4 {
                assert!((a[c] - b[c]).abs() < 1e-7);
            }
        }
    }
}

#[tokio::test]
async fn stamps_appear_on_every_cell() {
    // Grid of 2×2 stamps with scale 1 should leave no fully-black quadrant.
    let mut inputs = vec![
        Input::new("pattern".into(), Value::Image { data: square_pattern(), change_id: get_id() }, None, None),
        Input::new("width".into(), Value::Integer(32), None, None),
        Input::new("height".into(), Value::Integer(32), None, None),
        Input::new("count_x".into(), Value::Integer(2), None, None),
        Input::new("count_y".into(), Value::Integer(2), None, None),
        Input::new("scale".into(), Value::Decimal(1.0), None, None),
        Input::new("rotation".into(), Value::Decimal(0.0), None, None),
        Input::new("row offset".into(), Value::Decimal(0.0), None, None),
        Input::new("col offset".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImagePatternTileGenerator::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    // Check a pixel in each quadrant.
    for (qx, qy) in &[(8u32, 8u32), (24, 8), (8, 24), (24, 24)] {
        let px = data.get_pixel(*qx, *qy);
        assert!(px[0] > 0.1, "quadrant ({qx},{qy}) missing stamp");
    }
}
