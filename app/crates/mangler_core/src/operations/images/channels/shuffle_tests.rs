//! Tests for the channel shuffle operation.
use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

#[tokio::test]
async fn test_shuffle_settings() { assert_eq!(OpImageChannelShuffle::settings().name, "channel shuffle"); assert_eq!(OpImageChannelShuffle::create_inputs().len(), 5); }

#[tokio::test]
async fn test_shuffle_identity() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.04, 0.08, 0.12, 0.16]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("red source".to_string(), Value::Integer(0), None, None),
        Input::new("green source".to_string(), Value::Integer(1), None, None),
        Input::new("blue source".to_string(), Value::Integer(2), None, None),
        Input::new("alpha source".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpImageChannelShuffle::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!((p[0] - 0.04).abs() < 0.001);
            assert!((p[1] - 0.08).abs() < 0.001);
            assert!((p[2] - 0.12).abs() < 0.001);
            assert!((p[3] - 0.16).abs() < 0.001);
        }
        other => panic!("{:?}", other),
    }
}

#[tokio::test]
async fn test_shuffle_swap_red_blue() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.04, 0.08, 0.12, 0.16]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("red source".to_string(), Value::Integer(2), None, None),
        Input::new("green source".to_string(), Value::Integer(1), None, None),
        Input::new("blue source".to_string(), Value::Integer(0), None, None),
        Input::new("alpha source".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpImageChannelShuffle::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!((p[0] - 0.12).abs() < 0.001); // was blue
            assert!((p[2] - 0.04).abs() < 0.001); // was red
        }
        other => panic!("{:?}", other),
    }
}
