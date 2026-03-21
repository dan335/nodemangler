use super::*;

use crate::input::Input;
use crate::value::Value;


#[tokio::test]
async fn test_opimagenoisehybridmultifractalnoise_settings() {
    let s = OpImageNoiseHybridMultifractalNoise::settings();
    assert_eq!(s.name, "hybrid multifractal noise");
    assert_eq!(OpImageNoiseHybridMultifractalNoise::create_inputs().len(), 7);
    assert_eq!(OpImageNoiseHybridMultifractalNoise::create_outputs().len(), 1);
}


#[tokio::test]
async fn test_opimagenoisehybridmultifractalnoise_run() {
    let mut inputs = vec![
        Input::new("i0".to_string(), Value::Integer(4), None, None),
        Input::new("i1".to_string(), Value::Integer(4), None, None),
        Input::new("i2".to_string(), Value::Integer(4), None, None),
        Input::new("i3".to_string(), Value::Integer(4), None, None),
        Input::new("i4".to_string(), Value::Integer(4), None, None),
        Input::new("i5".to_string(), Value::Integer(4), None, None),
        Input::new("i6".to_string(), Value::Integer(4), None, None),

    ];
    let result = OpImageNoiseHybridMultifractalNoise::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagenoisehybridmultifractalnoise_correct_dimensions() {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("octaves".to_string(), Value::Integer(4), None, None),
        Input::new("frequency".to_string(), Value::Integer(5), None, None),
        Input::new("lacunarity".to_string(), Value::Decimal(2.0), None, None),
        Input::new("persistence".to_string(), Value::Decimal(0.5), None, None),

    ];
    let result = OpImageNoiseHybridMultifractalNoise::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
