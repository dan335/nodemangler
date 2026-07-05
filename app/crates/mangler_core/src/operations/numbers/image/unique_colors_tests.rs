use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn inputs(img: FloatImage, levels: i32) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("levels".to_string(), Value::Integer(levels), None, None),
    ]
}

fn int(v: &Value) -> i32 {
    match v { Value::Integer(i) => *i, other => panic!("expected Integer, got {:?}", other) }
}

#[tokio::test]
async fn test_unique_colors_settings() {
    let s = OpNumberImageUniqueColors::settings();
    assert_eq!(s.name, "unique colors");
    assert_eq!(OpNumberImageUniqueColors::create_inputs().len(), 2);
    assert_eq!(OpNumberImageUniqueColors::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_unique_colors_uniform_is_one() {
    let img = FloatImage::from_pixel(4, 4, 3, &[0.2, 0.4, 0.6]);
    let mut inp = inputs(img, 32);
    let r = OpNumberImageUniqueColors::run(&mut inp).await.unwrap();
    assert_eq!(int(&r.responses[0].value), 1);
}

#[tokio::test]
async fn test_unique_colors_two_distinct() {
    // two clearly different colors -> 2 buckets at any reasonable level
    let mut img = FloatImage::new(2, 1, 3);
    img.put_pixel(0, 0, &[1.0, 0.0, 0.0]);
    img.put_pixel(1, 0, &[0.0, 1.0, 0.0]);
    let mut inp = inputs(img, 32);
    let r = OpNumberImageUniqueColors::run(&mut inp).await.unwrap();
    assert_eq!(int(&r.responses[0].value), 2);
}

#[tokio::test]
async fn test_unique_colors_quantization_merges() {
    // two nearly-identical colors collapse into one bucket at coarse levels
    let mut img = FloatImage::new(2, 1, 3);
    img.put_pixel(0, 0, &[0.50, 0.50, 0.50]);
    img.put_pixel(1, 0, &[0.51, 0.50, 0.50]);
    let mut inp = inputs(img, 2);
    let r = OpNumberImageUniqueColors::run(&mut inp).await.unwrap();
    assert_eq!(int(&r.responses[0].value), 1);
}
