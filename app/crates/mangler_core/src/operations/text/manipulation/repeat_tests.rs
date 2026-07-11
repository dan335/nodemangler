use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_repeat_settings() {
    let s = OpTextRepeat::settings();
    assert_eq!(s.name, "repeat");
    assert_eq!(OpTextRepeat::create_inputs().len(), 2);
    assert_eq!(OpTextRepeat::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_repeat_basic() {
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text("ab".to_string()), None, None),
        Input::new("count".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpTextRepeat::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "ababab"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_repeat_negative_clamps_to_zero() {
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text("x".to_string()), None, None),
        Input::new("count".to_string(), Value::Integer(-5), None, None),
    ];
    let result = OpTextRepeat::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, ""),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_repeat_huge_count_from_wired_input_is_capped() {
    // The DragValue clamp (0..10000) only bounds manual entry; a wired
    // upstream node can send i32::MAX. Without a cap, `"x".repeat(i32::MAX)`
    // would try to allocate ~2GB. The output should be capped to a sane
    // total length instead of panicking or exhausting memory.
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text("x".to_string()), None, None),
        Input::new("count".to_string(), Value::Integer(i32::MAX), None, None),
    ];
    let result = OpTextRepeat::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert!(v.len() <= 100_000, "Expected capped output, got length {}", v.len()),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_repeat_huge_count_with_long_text_still_capped() {
    // A longer input text should proportionally reduce the effective repeat
    // count, keeping the total output bounded regardless of `text` length.
    let long_text = "a".repeat(1000);
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text(long_text), None, None),
        Input::new("count".to_string(), Value::Integer(i32::MAX), None, None),
    ];
    let result = OpTextRepeat::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert!(v.len() <= 100_000, "Expected capped output, got length {}", v.len()),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_repeat_count_one_always_allowed_even_for_long_text() {
    // A count of 1 should always return a single copy, even if the text is
    // already longer than the output cap — capping must never suppress the
    // single-copy case.
    let long_text = "a".repeat(200_000);
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text(long_text.clone()), None, None),
        Input::new("count".to_string(), Value::Integer(1), None, None),
    ];
    let result = OpTextRepeat::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, &long_text),
        other => panic!("Expected Text, got {:?}", other),
    }
}
