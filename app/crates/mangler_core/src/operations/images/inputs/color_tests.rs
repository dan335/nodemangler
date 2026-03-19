use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_from_color() {
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
    ];
    let result = OpImageInputColor::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_from_color_settings() {
    let s = OpImageInputColor::settings();
    assert_eq!(s.name, "from color");
    assert_eq!(OpImageInputColor::create_inputs().len(), 3);
    assert_eq!(OpImageInputColor::create_outputs().len(), 4);
}

#[tokio::test]
async fn test_from_color_pixel_values() {
    // All pixels should be the input color
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::from_srgb_u8(255, 0, 128, 200)), None, None),
        Input::new("width".to_string(), Value::Integer(4), None, None),
        Input::new("height".to_string(), Value::Integer(4), None, None),
    ];
    let result = OpImageInputColor::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let rgba = data.to_rgba8();
            // Every pixel should match (within u8 rounding of sRGB)
            for y in 0..4 {
                for x in 0..4 {
                    let p = rgba.get_pixel(x, y).0;
                    assert_eq!(p[0], 255, "red channel mismatch at ({x},{y})");
                    assert_eq!(p[1], 0, "green channel mismatch at ({x},{y})");
                }
            }
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_from_color_outputs_color_passthrough() {
    // The second output should be the same color that was input
    let color = Color::from_srgb_float(0.5, 0.25, 0.75, 1.0);
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(color), None, None),
        Input::new("width".to_string(), Value::Integer(4), None, None),
        Input::new("height".to_string(), Value::Integer(4), None, None),
    ];
    let result = OpImageInputColor::run(&mut inputs).await.unwrap();
    match &result.responses[1].value {
        Value::Color(_) => {}
        other => panic!("Expected Color output, got {:?}", other),
    }
}

#[tokio::test]
async fn test_from_color_outputs_width_height() {
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::default()), None, None),
        Input::new("width".to_string(), Value::Integer(10), None, None),
        Input::new("height".to_string(), Value::Integer(7), None, None),
    ];
    let result = OpImageInputColor::run(&mut inputs).await.unwrap();
    match &result.responses[2].value {
        Value::Integer(w) => assert_eq!(*w, 10),
        other => panic!("Expected Integer width, got {:?}", other),
    }
    match &result.responses[3].value {
        Value::Integer(h) => assert_eq!(*h, 7),
        other => panic!("Expected Integer height, got {:?}", other),
    }
}

#[tokio::test]
async fn test_from_color_1x1() {
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
        Input::new("width".to_string(), Value::Integer(1), None, None),
        Input::new("height".to_string(), Value::Integer(1), None, None),
    ];
    let result = OpImageInputColor::run(&mut inputs).await;
    assert!(result.is_ok(), "from_color 1x1 failed: {:?}", result.err());
}
