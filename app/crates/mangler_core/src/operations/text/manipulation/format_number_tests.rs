use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_format_number_settings() {
    let s = OpTextFormatNumber::settings();
    assert_eq!(s.name, "format number");
    assert_eq!(OpTextFormatNumber::create_inputs().len(), 4);
    assert_eq!(OpTextFormatNumber::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_format_number_decimals() {
    let mut inputs = vec![
        Input::new("value".to_string(), Value::Decimal(3.14159), None, None),
        Input::new("decimals".to_string(), Value::Integer(2), None, None),
        Input::new("min width".to_string(), Value::Integer(0), None, None),
        Input::new("pad zeros".to_string(), Value::Bool(false), None, None),
    ];
    let result = OpTextFormatNumber::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "3.14"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_format_number_zero_pad() {
    let mut inputs = vec![
        Input::new("value".to_string(), Value::Decimal(5.0), None, None),
        Input::new("decimals".to_string(), Value::Integer(1), None, None),
        Input::new("min width".to_string(), Value::Integer(6), None, None),
        Input::new("pad zeros".to_string(), Value::Bool(true), None, None),
    ];
    let result = OpTextFormatNumber::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "0005.0"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_format_number_space_pad() {
    let mut inputs = vec![
        Input::new("value".to_string(), Value::Decimal(5.0), None, None),
        Input::new("decimals".to_string(), Value::Integer(1), None, None),
        Input::new("min width".to_string(), Value::Integer(6), None, None),
        Input::new("pad zeros".to_string(), Value::Bool(false), None, None),
    ];
    let result = OpTextFormatNumber::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "   5.0"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_format_number_huge_decimals_from_wired_input_is_capped() {
    // decimals' DragValue clamp (0..15) only bounds manual entry; a wired
    // upstream node can send i32::MAX. Without a cap, `format!("{:.*}", ...)`
    // would try to pad millions of meaningless digits. Decimals should be
    // capped to a sane maximum (17) instead of exhausting memory.
    let mut inputs = vec![
        Input::new("value".to_string(), Value::Decimal(5.0), None, None),
        Input::new("decimals".to_string(), Value::Integer(i32::MAX), None, None),
        Input::new("min width".to_string(), Value::Integer(0), None, None),
        Input::new("pad zeros".to_string(), Value::Bool(false), None, None),
    ];
    let result = OpTextFormatNumber::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => {
            // "5." + at most 17 fractional digits
            assert!(v.len() <= 20, "Expected capped decimals, got length {} ({})", v.len(), v);
        }
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_format_number_huge_min_width_from_wired_input_is_capped() {
    // min width's DragValue clamp (0..64) only bounds manual entry; a wired
    // upstream node can send i32::MAX. Without a cap, the left-pad loop
    // would try to build a ~2GB string. Width should be capped to a sane
    // maximum instead of exhausting memory.
    let mut inputs = vec![
        Input::new("value".to_string(), Value::Decimal(5.0), None, None),
        Input::new("decimals".to_string(), Value::Integer(1), None, None),
        Input::new("min width".to_string(), Value::Integer(i32::MAX), None, None),
        Input::new("pad zeros".to_string(), Value::Bool(false), None, None),
    ];
    let result = OpTextFormatNumber::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert!(v.len() <= 100_000, "Expected capped output, got length {}", v.len()),
        other => panic!("Expected Text, got {:?}", other),
    }
}
