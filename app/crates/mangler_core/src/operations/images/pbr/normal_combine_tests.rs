//! Tests for the normal combine operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Build a solid normal-map image where every pixel holds the same packed normal.
fn solid_normal(w: u32, h: u32, n: [f32; 3]) -> Arc<FloatImage> {
    let px = pack_normal(n);
    Arc::new(FloatImage::from_pixel(w, h, 4, &px))
}

#[tokio::test]
async fn settings_and_shape() {
    let s = OpImagePbrNormalCombine::settings();
    assert_eq!(s.name, "normal combine");
    assert_eq!(OpImagePbrNormalCombine::create_inputs().len(), 3);
    assert_eq!(OpImagePbrNormalCombine::create_outputs().len(), 1);
}

#[tokio::test]
async fn flat_plus_flat_is_flat() {
    // Two flat-up normals combined — result must still be flat-up regardless of mode.
    let flat = [0.0f32, 0.0, 1.0];
    for mode in 0..=3 {
        let mut inputs = vec![
            Input::new("base".into(), Value::Image { data: solid_normal(4, 4, flat), change_id: get_id() }, None, None),
            Input::new("detail".into(), Value::Image { data: solid_normal(4, 4, flat), change_id: get_id() }, None, None),
            Input::new("mode".into(), Value::Integer(mode), None, None),
        ];
        let r = OpImagePbrNormalCombine::run(&mut inputs).await.unwrap();
        let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
        let px = data.get_pixel(0, 0);
        assert!((px[0] - 0.5).abs() < 1e-3, "mode {} r={}", mode, px[0]);
        assert!((px[1] - 0.5).abs() < 1e-3, "mode {} g={}", mode, px[1]);
        assert!(px[2] > 0.99, "mode {} b={}", mode, px[2]);
    }
}

#[tokio::test]
async fn flat_detail_leaves_base_unchanged() {
    // A flat detail map over a tilted base normal should leave the base
    // (approximately) unchanged across every blend mode.
    let tilted = normalize([0.3, 0.0, 1.0]);
    let flat = [0.0f32, 0.0, 1.0];
    for mode in 0..=3 {
        let mut inputs = vec![
            Input::new("base".into(), Value::Image { data: solid_normal(4, 4, tilted), change_id: get_id() }, None, None),
            Input::new("detail".into(), Value::Image { data: solid_normal(4, 4, flat), change_id: get_id() }, None, None),
            Input::new("mode".into(), Value::Integer(mode), None, None),
        ];
        let r = OpImagePbrNormalCombine::run(&mut inputs).await.unwrap();
        let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
        let px = data.get_pixel(0, 0);
        let unpacked = unpack_normal(px);
        // X component should remain tilted the same direction (positive).
        assert!(unpacked[0] > 0.1, "mode {} x drifted: {}", mode, unpacked[0]);
        // Z stays positive (no sign flip).
        assert!(unpacked[2] > 0.5, "mode {} z collapsed: {}", mode, unpacked[2]);
    }
}
