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
async fn test_image_info_settings() {
    let s = OpTextImageInfo::settings();
    assert_eq!(s.name, "image info");
    assert_eq!(OpTextImageInfo::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_image_info_basic() {
    let mut inputs = image_input(640, 480, 3);
    let r = OpTextImageInfo::run(&mut inputs).await.unwrap();
    match &r.responses[0].value {
        Value::Text(t) => {
            assert!(t.contains("640×480"), "got {t}");
            assert!(t.contains("3 channels"), "got {t}");
            assert!(t.contains("1.33:1"), "got {t}");
        }
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_image_info_singular_channel() {
    let mut inputs = image_input(10, 10, 1);
    let r = OpTextImageInfo::run(&mut inputs).await.unwrap();
    match &r.responses[0].value {
        Value::Text(t) => assert!(t.contains("1 channel,"), "got {t}"),
        other => panic!("Expected Text, got {:?}", other),
    }
}
