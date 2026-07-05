use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn image_input(w: u32, h: u32, ch: u32) -> Vec<Input> {
    let img = FloatImage::from_pixel(w, h, ch, &vec![0.5; ch as usize]);
    vec![Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None)]
}

#[tokio::test]
async fn test_dimensions_settings() {
    let s = OpNumberImageDimensions::settings();
    assert_eq!(s.name, "dimensions");
    assert_eq!(OpNumberImageDimensions::create_inputs().len(), 1);
    assert_eq!(OpNumberImageDimensions::create_outputs().len(), 4);
}

#[tokio::test]
async fn test_dimensions_basic() {
    let mut inputs = image_input(640, 480, 3);
    let result = OpNumberImageDimensions::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Integer(640)));
    assert!(matches!(result.responses[1].value, Value::Integer(480)));
    match result.responses[2].value {
        Value::Decimal(a) => assert!((a - 640.0 / 480.0).abs() < 1e-4),
        ref other => panic!("expected Decimal, got {:?}", other),
    }
    assert!(matches!(result.responses[3].value, Value::Integer(3)));
}

#[tokio::test]
async fn test_dimensions_square_aspect_one() {
    let mut inputs = image_input(256, 256, 4);
    let result = OpNumberImageDimensions::run(&mut inputs).await.unwrap();
    match result.responses[2].value {
        Value::Decimal(a) => assert!((a - 1.0).abs() < 1e-6),
        ref other => panic!("expected Decimal, got {:?}", other),
    }
    assert!(matches!(result.responses[3].value, Value::Integer(4)));
}
