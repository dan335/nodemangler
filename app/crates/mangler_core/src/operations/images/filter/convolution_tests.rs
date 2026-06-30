//! Tests for the custom convolution filter.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn gradient(w: u32, h: u32) -> Value {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            img.put_pixel(x, y, &[x as f32 / w as f32, y as f32 / h as f32, 0.5, 1.0]);
        }
    }
    Value::Image { data: Arc::new(img), change_id: get_id() }
}

async fn run(image: Value, kernel: [f32; 9], divisor: f32, bias: f32) -> Value {
    let names = ["k00", "k01", "k02", "k10", "k11", "k12", "k20", "k21", "k22"];
    let mut inputs = vec![Input::new("image".to_string(), image, None, None)];
    for (n, v) in names.iter().zip(kernel.iter()) {
        inputs.push(Input::new(n.to_string(), Value::Decimal(*v), None, None));
    }
    inputs.push(Input::new("divisor".to_string(), Value::Decimal(divisor), None, None));
    inputs.push(Input::new("bias".to_string(), Value::Decimal(bias), None, None));
    OpImageAdjustmentConvolution::run(&mut inputs).await.unwrap().responses[0].value.clone()
}

const IDENTITY: [f32; 9] = [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageAdjustmentConvolution::settings().name, "convolution");
    assert_eq!(OpImageAdjustmentConvolution::create_inputs().len(), 12);
    assert_eq!(OpImageAdjustmentConvolution::create_outputs().len(), 1);
}

#[tokio::test]
async fn identity_kernel_is_identity() {
    let src = gradient(8, 8);
    let Value::Image { data: src_data, .. } = &src else { panic!() };
    let src_data = src_data.clone();
    let Value::Image { data, .. } = run(src, IDENTITY, 1.0, 0.0).await else { panic!() };
    assert_eq!(data.as_raw(), src_data.as_raw());
}

#[tokio::test]
async fn box_blur_uniform_stays_uniform() {
    let img = FloatImage::from_pixel(8, 8, 4, &[0.4, 0.6, 0.2, 1.0]);
    let out = run(Value::Image { data: Arc::new(img), change_id: get_id() }, [1.0; 9], 9.0, 0.0).await;
    let Value::Image { data, .. } = out else { panic!() };
    assert!(data.pixels().all(|p| (p[0] - 0.4).abs() < 1e-5 && (p[1] - 0.6).abs() < 1e-5 && (p[2] - 0.2).abs() < 1e-5));
}

#[tokio::test]
async fn bias_adds_constant_with_zero_kernel() {
    let img = FloatImage::from_pixel(4, 4, 4, &[0.1, 0.2, 0.3, 0.7]);
    let out = run(Value::Image { data: Arc::new(img), change_id: get_id() }, [0.0; 9], 1.0, 0.25).await;
    let Value::Image { data, .. } = out else { panic!() };
    for p in data.pixels() {
        assert!((p[0] - 0.25).abs() < 1e-6 && (p[1] - 0.25).abs() < 1e-6 && (p[2] - 0.25).abs() < 1e-6);
        assert!((p[3] - 0.7).abs() < 1e-6, "alpha should be preserved");
    }
}

#[tokio::test]
async fn zero_divisor_treated_as_one() {
    let src = gradient(4, 4);
    let Value::Image { data: src_data, .. } = &src else { panic!() };
    let src_data = src_data.clone();
    // Identity kernel with divisor 0 should behave like divisor 1, not NaN.
    let Value::Image { data, .. } = run(src, IDENTITY, 0.0, 0.0).await else { panic!() };
    assert_eq!(data.as_raw(), src_data.as_raw());
}

#[tokio::test]
async fn preserves_dimensions() {
    let Value::Image { data, .. } = run(gradient(11, 5), IDENTITY, 1.0, 0.0).await else { panic!() };
    assert_eq!(data.dimensions(), (11, 5));
}
