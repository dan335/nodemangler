use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::value::Value;
use std::sync::Arc;

/// Builds an RGBA image input plus a `max size` input.
fn inputs_rgba(w: u32, h: u32, max_size: i32) -> Vec<Input> {
    let img = FloatImage::from_pixel(w, h, 4, &[0.25, 0.5, 0.75, 1.0]);
    vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("max size".to_string(), Value::Integer(max_size), Some(InputSettings::DragValue { clamp: Some((16.0, 4096.0)), speed: None }), None),
    ]
}

#[tokio::test]
async fn test_data_uri_settings() {
    let s = OpTextImageDataUri::settings();
    assert_eq!(s.name, "data uri");
    assert_eq!(OpTextImageDataUri::create_inputs().len(), 2);
    assert_eq!(OpTextImageDataUri::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_data_uri_prefix() {
    let mut inputs = inputs_rgba(8, 8, 512);
    let r = OpTextImageDataUri::run(&mut inputs).await.unwrap();
    match &r.responses[0].value {
        Value::Text(t) => {
            assert!(t.starts_with("data:image/png;base64,"), "got prefix {:.32}", t);
            assert!(t.len() > "data:image/png;base64,".len(), "should carry encoded bytes");
        }
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_data_uri_downscales() {
    // A large image with a tiny max size should still produce a valid URI.
    let mut inputs = inputs_rgba(200, 100, 16);
    let r = OpTextImageDataUri::run(&mut inputs).await.unwrap();
    let Value::Text(t) = &r.responses[0].value else { panic!("expected Text") };
    assert!(t.starts_with("data:image/png;base64,"), "got {:.32}", t);
}
