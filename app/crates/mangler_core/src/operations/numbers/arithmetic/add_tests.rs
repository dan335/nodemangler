use super::*;

use crate::float_image::FloatImage;
use std::sync::Arc;

macro_rules! assert_value {
    ($val:expr, Integer($expected:expr)) => {
        match &$val { Value::Integer(v) => assert_eq!(*v, $expected), other => panic!("Expected Integer({}), got {:?}", $expected, other) }
    };
    ($val:expr, Decimal($expected:expr)) => {
        match &$val { Value::Decimal(v) => assert!((*v - $expected).abs() < 1e-6, "Expected Decimal({}), got Decimal({})", $expected, v), other => panic!("Expected Decimal({}), got {:?}", $expected, other) }
    };
    ($val:expr, Bool($expected:expr)) => {
        match &$val { Value::Bool(v) => assert_eq!(*v, $expected), other => panic!("Expected Bool({}), got {:?}", $expected, other) }
    };
    ($val:expr, Text($expected:expr)) => {
        match &$val { Value::Text(v) => assert_eq!(v, $expected), other => panic!("Expected Text, got {:?}", other) }
    };
}

fn make_inputs(a: Value, b: Value) -> Vec<Input> {
    vec![
        Input::new("a".to_string(), a, None, None),
        Input::new("b".to_string(), b, None, None),
    ]
}

/// Creates a test image with a gradient pattern as a 4-channel FloatImage.
fn test_image(w: u32, h: u32) -> Arc<FloatImage> {
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

/// Creates a Value::Image from a test gradient image.
fn image_input(w: u32, h: u32) -> Value {
    Value::Image { data: test_image(w, h), change_id: get_id() }
}

/// Unwraps a Value::Image, panicking with a helpful message otherwise.
fn expect_image(value: &Value) -> &FloatImage {
    match value {
        Value::Image { data, .. } => data,
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_add_decimal_decimal() {
    let mut inputs = make_inputs(
        Value::Decimal(5.0),
        Value::Decimal(10.0),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Decimal(15.0));
}

#[tokio::test]
async fn test_add_integer_integer() {
    let mut inputs = make_inputs(
        Value::Integer(5),
        Value::Integer(10),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Integer(15));
}

#[tokio::test]
async fn test_add_integer_decimal() {
    let mut inputs = make_inputs(
        Value::Integer(5),
        Value::Decimal(2.5),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Decimal(7.5));
}

#[tokio::test]
async fn test_add_decimal_integer() {
    let mut inputs = make_inputs(
        Value::Decimal(2.5),
        Value::Integer(5),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Decimal(7.5));
}

#[tokio::test]
async fn test_add_bool_true_integer() {
    let mut inputs = make_inputs(
        Value::Bool(true),
        Value::Integer(5),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Integer(6));
}

#[tokio::test]
async fn test_add_bool_false_integer() {
    let mut inputs = make_inputs(
        Value::Bool(false),
        Value::Integer(5),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Integer(5));
}

#[tokio::test]
async fn test_add_bool_bool() {
    let mut inputs = make_inputs(
        Value::Bool(true),
        Value::Bool(false),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Integer(1));
}

#[tokio::test]
async fn test_add_bool_bool_true_true() {
    let mut inputs = make_inputs(
        Value::Bool(true),
        Value::Bool(true),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Integer(2));
}

#[tokio::test]
async fn test_add_bool_bool_false_false() {
    let mut inputs = make_inputs(
        Value::Bool(false),
        Value::Bool(false),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Integer(0));
}

#[tokio::test]
async fn test_add_bool_decimal() {
    let mut inputs = make_inputs(
        Value::Bool(true),
        Value::Decimal(5.5),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Decimal(6.5));
}

#[tokio::test]
async fn test_add_integer_bool_true() {
    let mut inputs = make_inputs(
        Value::Integer(10),
        Value::Bool(true),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Integer(11));
}

#[tokio::test]
async fn test_add_decimal_bool_true() {
    let mut inputs = make_inputs(
        Value::Decimal(10.0),
        Value::Bool(true),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Decimal(11.0));
}

#[tokio::test]
async fn test_add_decimal_zero() {
    let mut inputs = make_inputs(
        Value::Decimal(0.0),
        Value::Decimal(0.0),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Decimal(0.0));
}

#[tokio::test]
async fn test_add_negative_numbers() {
    let mut inputs = make_inputs(
        Value::Integer(-5),
        Value::Integer(-10),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Integer(-15));
}

#[tokio::test]
async fn test_add_text_concat() {
    let mut inputs = make_inputs(
        Value::Bool(true),
        Value::Text("hello".to_string()),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Text("truehello"));
}

#[tokio::test]
async fn test_add_integer_text_concat() {
    let mut inputs = make_inputs(
        Value::Integer(42),
        Value::Text("hello".to_string()),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Text("42hello"));
}

#[tokio::test]
async fn test_add_settings() {
    let settings = OpNumberMathAdd::settings();
    assert_eq!(settings.name, "add");
}

#[tokio::test]
async fn test_add_create_inputs_count() {
    let inputs = OpNumberMathAdd::create_inputs();
    assert_eq!(inputs.len(), 2);
}

#[tokio::test]
async fn test_add_create_outputs_count() {
    let outputs = OpNumberMathAdd::create_outputs();
    assert_eq!(outputs.len(), 1);
}

#[tokio::test]
async fn test_add_large_integers() {
    let mut inputs = make_inputs(
        Value::Integer(i32::MAX / 2),
        Value::Integer(i32::MAX / 2),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Integer(i32::MAX - 1));
}

#[tokio::test]
async fn test_add_large_decimals() {
    let mut inputs = make_inputs(
        Value::Decimal(1e15_f32),
        Value::Decimal(1e15_f32),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(*v > 0.0),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_add_tiny_decimals() {
    let mut inputs = make_inputs(
        Value::Decimal(0.0001),
        Value::Decimal(0.0001),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Decimal(0.0002));
}

#[tokio::test]
async fn test_add_mixed_sign() {
    let mut inputs = make_inputs(
        Value::Integer(100),
        Value::Integer(-100),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Integer(0));
}

#[tokio::test]
async fn test_add_decimal_negative() {
    let mut inputs = make_inputs(
        Value::Decimal(-3.5),
        Value::Decimal(-1.5),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Decimal(-5.0));
}

#[tokio::test]
async fn test_add_integer_zero() {
    let mut inputs = make_inputs(
        Value::Integer(0),
        Value::Integer(0),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Integer(0));
}

#[tokio::test]
async fn test_add_invalid_type_returns_error() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Bool(true), None, None),
        Input::new("b".to_string(), Value::Trigger, None, None),
    ];
    let result = OpNumberMathAdd::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for unsupported type combination");
}

#[tokio::test]
async fn test_add_bool_false_decimal() {
    let mut inputs = make_inputs(
        Value::Bool(false),
        Value::Decimal(5.5),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Decimal(5.5));
}

#[tokio::test]
async fn test_add_integer_decimal_fractional_result() {
    let mut inputs = make_inputs(
        Value::Integer(3),
        Value::Decimal(0.25),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 3.25).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_add_image_decimal_changes_pixels() {
    let mut inputs = make_inputs(image_input(4, 4), Value::Decimal(0.25));
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    let img = expect_image(&result.responses[0].value);
    assert_eq!(img.dimensions(), (4, 4));
    let original = test_image(4, 4);
    for (x, y, pixel) in img.enumerate_pixels() {
        let orig = original.get_pixel(x, y);
        for c in 0..pixel.len() {
            assert!((pixel[c] - (orig[c] + 0.25)).abs() < 1e-6,
                "Pixel ({},{}) channel {}: expected {}, got {}", x, y, c, orig[c] + 0.25, pixel[c]);
        }
    }
}

#[tokio::test]
async fn test_add_decimal_image_changes_pixels() {
    let mut inputs = make_inputs(Value::Decimal(0.25), image_input(4, 4));
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    let img = expect_image(&result.responses[0].value);
    let original = test_image(4, 4);
    for (x, y, pixel) in img.enumerate_pixels() {
        let orig = original.get_pixel(x, y);
        for c in 0..pixel.len() {
            assert!((pixel[c] - (orig[c] + 0.25)).abs() < 1e-6);
        }
    }
}

#[tokio::test]
async fn test_add_image_integer_changes_pixels() {
    let mut inputs = make_inputs(image_input(2, 2), Value::Integer(2));
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    let img = expect_image(&result.responses[0].value);
    let original = test_image(2, 2);
    for (x, y, pixel) in img.enumerate_pixels() {
        let orig = original.get_pixel(x, y);
        for c in 0..pixel.len() {
            assert!((pixel[c] - (orig[c] + 2.0)).abs() < 1e-6);
        }
    }
}

#[tokio::test]
async fn test_add_image_bool_true_changes_pixels() {
    let mut inputs = make_inputs(image_input(2, 2), Value::Bool(true));
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    let img = expect_image(&result.responses[0].value);
    let original = test_image(2, 2);
    for (x, y, pixel) in img.enumerate_pixels() {
        let orig = original.get_pixel(x, y);
        for c in 0..pixel.len() {
            assert!((pixel[c] - (orig[c] + 1.0)).abs() < 1e-6);
        }
    }
}

#[tokio::test]
async fn test_add_image_image_same_size() {
    let mut inputs = make_inputs(image_input(3, 3), image_input(3, 3));
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    let img = expect_image(&result.responses[0].value);
    assert_eq!(img.dimensions(), (3, 3));
    assert_eq!(img.channels(), 4);
    let original = test_image(3, 3);
    for (x, y, pixel) in img.enumerate_pixels() {
        let orig = original.get_pixel(x, y);
        for c in 0..pixel.len() {
            assert!((pixel[c] - orig[c] * 2.0).abs() < 1e-6,
                "Pixel ({},{}) channel {}: expected {}, got {}", x, y, c, orig[c] * 2.0, pixel[c]);
        }
    }
}

#[tokio::test]
async fn test_add_image_image_mismatched_size_errors() {
    let mut inputs = make_inputs(image_input(4, 4), image_input(2, 2));
    let result = OpNumberMathAdd::run(&mut inputs).await;
    let err = result.expect_err("Expected error for mismatched image dimensions");
    assert_eq!(err.input_errors[0].0, 1);
    assert!(err.input_errors[0].1.contains("dimensions"), "Unexpected message: {}", err.input_errors[0].1);
}

#[tokio::test]
async fn test_add_image_image_mismatched_channels_errors() {
    let gray = {
        let mut img = FloatImage::new(2, 2, 1);
        for y in 0..2 { for x in 0..2 { img.put_pixel(x, y, &[0.5]); } }
        Value::Image { data: Arc::new(img), change_id: get_id() }
    };
    let mut inputs = make_inputs(image_input(2, 2), gray);
    let result = OpNumberMathAdd::run(&mut inputs).await;
    let err = result.expect_err("Expected error for mismatched channel counts");
    assert_eq!(err.input_errors[0].0, 1);
    assert!(err.input_errors[0].1.contains("channel"), "Unexpected message: {}", err.input_errors[0].1);
}

#[tokio::test]
async fn test_add_image_color_changes_pixels() {
    let color = Color::from_srgb_float(0.1, 0.2, 0.3, 0.0);
    let mut inputs = make_inputs(image_input(2, 2), Value::Color(color));
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    let img = expect_image(&result.responses[0].value);
    let original = test_image(2, 2);
    let (r, g, b, a) = color.to_srgb_float();
    let addend = [r, g, b, a];
    for (x, y, pixel) in img.enumerate_pixels() {
        let orig = original.get_pixel(x, y);
        for c in 0..pixel.len() {
            assert!((pixel[c] - (orig[c] + addend[c])).abs() < 1e-6,
                "Pixel ({},{}) channel {}: expected {}, got {}", x, y, c, orig[c] + addend[c], pixel[c]);
        }
    }
}

#[tokio::test]
async fn test_add_image_fresh_change_id() {
    let source = image_input(2, 2);
    let source_id = match &source { Value::Image { change_id, .. } => change_id.clone(), _ => unreachable!() };
    let mut inputs = make_inputs(source, Value::Decimal(0.5));
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { change_id, .. } => assert_ne!(*change_id, source_id, "Output image should carry a fresh change_id"),
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_add_text_text_errors() {
    let mut inputs = make_inputs(
        Value::Text("foo".to_string()),
        Value::Text("bar".to_string()),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await;
    let err = result.expect_err("Expected error for text as input 'a'");
    assert_eq!(err.input_errors[0].0, 0);
}

#[tokio::test]
async fn test_add_text_integer_errors() {
    let mut inputs = make_inputs(
        Value::Text("foo".to_string()),
        Value::Integer(1),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for text as input 'a'");
}

#[tokio::test]
async fn test_add_path_integer_errors() {
    let mut inputs = make_inputs(
        Value::Path(std::path::PathBuf::from("/tmp/foo.png")),
        Value::Integer(1),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await;
    let err = result.expect_err("Expected error for path as input 'a'");
    assert_eq!(err.input_errors[0].0, 0);
}

#[tokio::test]
async fn test_add_image_trigger_errors() {
    let mut inputs = make_inputs(image_input(2, 2), Value::Trigger);
    let result = OpNumberMathAdd::run(&mut inputs).await;
    let err = result.expect_err("Expected error for image + trigger");
    assert_eq!(err.input_errors[0].0, 1);
}
