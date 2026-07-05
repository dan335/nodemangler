use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn inputs(img: FloatImage, threshold: f32) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("threshold".to_string(), Value::Decimal(threshold), None, None),
    ]
}

fn int(v: &Value) -> i32 {
    match v { Value::Integer(i) => *i, other => panic!("expected Integer, got {:?}", other) }
}

#[tokio::test]
async fn test_bounding_box_settings() {
    let s = OpNumberImageBoundingBox::settings();
    assert_eq!(s.name, "bounding box");
    assert_eq!(OpNumberImageBoundingBox::create_inputs().len(), 2);
    assert_eq!(OpNumberImageBoundingBox::create_outputs().len(), 4);
}

#[tokio::test]
async fn test_bounding_box_bright_square() {
    // 6x6 black grayscale with a bright 2x2 block at (2,3)..(3,4)
    let mut img = FloatImage::new(6, 6, 1);
    for y in 3..=4 {
        for x in 2..=3 {
            img.put_pixel(x, y, &[1.0]);
        }
    }
    let mut inp = inputs(img, 0.5);
    let r = OpNumberImageBoundingBox::run(&mut inp).await.unwrap();
    assert_eq!(int(&r.responses[0].value), 2); // x
    assert_eq!(int(&r.responses[1].value), 3); // y
    assert_eq!(int(&r.responses[2].value), 2); // width
    assert_eq!(int(&r.responses[3].value), 2); // height
}

#[tokio::test]
async fn test_bounding_box_uses_alpha() {
    // 4x4 rgba, fully opaque single pixel at (1,1), rest transparent
    let mut img = FloatImage::from_pixel(4, 4, 4, &[1.0, 1.0, 1.0, 0.0]);
    img.put_pixel(1, 1, &[1.0, 1.0, 1.0, 1.0]);
    let mut inp = inputs(img, 0.5);
    let r = OpNumberImageBoundingBox::run(&mut inp).await.unwrap();
    assert_eq!(int(&r.responses[0].value), 1);
    assert_eq!(int(&r.responses[1].value), 1);
    assert_eq!(int(&r.responses[2].value), 1);
    assert_eq!(int(&r.responses[3].value), 1);
}

#[tokio::test]
async fn test_bounding_box_empty_all_zero() {
    let img = FloatImage::from_pixel(5, 5, 1, &[0.0]);
    let mut inp = inputs(img, 0.5);
    let r = OpNumberImageBoundingBox::run(&mut inp).await.unwrap();
    for i in 0..4 {
        assert_eq!(int(&r.responses[i].value), 0);
    }
}
