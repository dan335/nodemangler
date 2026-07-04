use super::*;

use crate::input::Input;
use crate::value::Value;


#[tokio::test]
async fn test_opimagenoiseopensimplex_settings() {
    let s = OpImageNoiseOpenSimplex::settings();
    assert_eq!(s.name, "open simplex noise");
    assert_eq!(OpImageNoiseOpenSimplex::create_inputs().len(), 4);
    assert_eq!(OpImageNoiseOpenSimplex::create_outputs().len(), 1);
}


#[tokio::test]
async fn test_opimagenoiseopensimplex_run() {
    let mut inputs = vec![
        Input::new("i0".to_string(), Value::Integer(4), None, None),
        Input::new("i1".to_string(), Value::Integer(4), None, None),
        Input::new("i2".to_string(), Value::Integer(4), None, None),
        Input::new("i3".to_string(), Value::Integer(4), None, None),

    ];
    let result = OpImageNoiseOpenSimplex::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagenoiseopensimplex_correct_dimensions() {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("scale".to_string(), Value::Integer(10), None, None),

    ];
    let result = OpImageNoiseOpenSimplex::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagenoiseopensimplex_deterministic() {
    let make_inputs = || vec![
        Input::new("seed".to_string(), Value::Integer(3), None, None),
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("scale".to_string(), Value::Integer(5), None, None),

    ];
    let r1 = OpImageNoiseOpenSimplex::run(&mut make_inputs()).await.unwrap();
    let r2 = OpImageNoiseOpenSimplex::run(&mut make_inputs()).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_eq!(d1.pixels().collect::<Vec<_>>(),
                       d2.pixels().collect::<Vec<_>>(),
                       "open simplex noise is not deterministic");
        }
        _ => panic!("Expected Image"),
    }
}
