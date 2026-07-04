use super::*;

use crate::input::Input;
use crate::value::Value;


#[tokio::test]
async fn test_opimagenoisecheckerboard_settings() {
    let s = OpImageNoiseCheckerboard::settings();
    assert_eq!(s.name, "checkerboard noise");
    assert_eq!(OpImageNoiseCheckerboard::create_inputs().len(), 3);
    assert_eq!(OpImageNoiseCheckerboard::create_outputs().len(), 1);
}


#[tokio::test]
async fn test_opimagenoisecheckerboard_run() {
    let mut inputs = vec![
        Input::new("i0".to_string(), Value::Integer(4), None, None),
        Input::new("i1".to_string(), Value::Integer(4), None, None),
        Input::new("i2".to_string(), Value::Integer(4), None, None)
    ];
    let result = OpImageNoiseCheckerboard::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagenoisecheckerboard_1x1() {
    let mut inputs = vec![
        Input::new("width".to_string(), Value::Integer(1), None, None),
        Input::new("height".to_string(), Value::Integer(1), None, None),
        Input::new("size".to_string(), Value::Integer(1), None, None),
    ];
    let result = OpImageNoiseCheckerboard::run(&mut inputs).await;
    assert!(result.is_ok(), "checkerboard 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_opimagenoisecheckerboard_correct_dimensions() {
    let mut inputs = vec![
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("size".to_string(), Value::Integer(4), None, None),
    ];
    let result = OpImageNoiseCheckerboard::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagenoisecheckerboard_pattern_alternates() {
    // 8x8 with 2 squares across -> 4px cells: white / black / black / white quadrants
    let mut inputs = vec![
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("size".to_string(), Value::Integer(2), None, None),
    ];
    let result = OpImageNoiseCheckerboard::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.get_pixel(0, 0)[0], 1.0, "top-left square should be white");
            assert_eq!(data.get_pixel(4, 0)[0], 0.0, "top-right square should be black");
            assert_eq!(data.get_pixel(0, 4)[0], 0.0, "bottom-left square should be black");
            assert_eq!(data.get_pixel(4, 4)[0], 1.0, "bottom-right square should be white");
            // Cells are solid within their bounds
            assert_eq!(data.get_pixel(3, 3)[0], 1.0);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagenoisecheckerboard_size_changes_pattern() {
    let make_inputs = |size: i32| vec![
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(16), None, None),
        Input::new("size".to_string(), Value::Integer(size), None, None),
    ];
    let r2 = OpImageNoiseCheckerboard::run(&mut make_inputs(2)).await.unwrap();
    let r4 = OpImageNoiseCheckerboard::run(&mut make_inputs(4)).await.unwrap();
    match (&r2.responses[0].value, &r4.responses[0].value) {
        (Value::Image { data: d2, .. }, Value::Image { data: d4, .. }) => {
            // With 4 squares across a 16px image the cell at x=4 is black;
            // with 2 squares it is still inside the white top-left cell.
            assert_eq!(d2.get_pixel(4, 0)[0], 1.0);
            assert_eq!(d4.get_pixel(4, 0)[0], 0.0);
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_opimagenoisecheckerboard_deterministic() {
    // Same inputs should give identical outputs
    let make_inputs = || vec![
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("size".to_string(), Value::Integer(2), None, None),
    ];
    let r1 = OpImageNoiseCheckerboard::run(&mut make_inputs()).await.unwrap();
    let r2 = OpImageNoiseCheckerboard::run(&mut make_inputs()).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            let p1: Vec<_> = d1.pixels().collect();
            let p2: Vec<_> = d2.pixels().collect();
            assert_eq!(p1, p2, "checkerboard should be deterministic");
        }
        _ => panic!("Expected Image"),
    }
}
