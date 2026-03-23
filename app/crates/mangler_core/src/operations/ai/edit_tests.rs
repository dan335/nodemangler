use super::*;
use crate::value::Value;

/// Node name is "ai edit".
#[test]
fn test_settings() {
    let settings = OpAiEdit::settings();
    assert_eq!(settings.name, "ai edit");
    assert!(!settings.description.is_empty());
}

/// Has 5 inputs with correct names.
#[test]
fn test_create_inputs() {
    let inputs = OpAiEdit::create_inputs();
    assert_eq!(inputs.len(), 5);
    assert_eq!(inputs[0].name, "image");
    assert_eq!(inputs[1].name, "prompt");
    assert_eq!(inputs[2].name, "model");
    assert_eq!(inputs[3].name, "size");
    assert_eq!(inputs[4].name, "api key");
}

/// Has 3 outputs: image, width, height.
#[test]
fn test_create_outputs() {
    let outputs = OpAiEdit::create_outputs();
    assert_eq!(outputs.len(), 3);
    assert_eq!(outputs[0].name, "image");
    assert_eq!(outputs[1].name, "width");
    assert_eq!(outputs[2].name, "height");
}

/// Input types are correct.
#[test]
fn test_input_types() {
    let inputs = OpAiEdit::create_inputs();
    assert!(matches!(inputs[0].value, Value::Image { .. }), "image input should be Image");
    assert!(matches!(inputs[1].value, Value::Text(_)), "prompt should be Text");
    assert!(matches!(inputs[2].value, Value::Text(_)), "model should be Text");
    assert!(matches!(inputs[3].value, Value::Text(_)), "size should be Text");
    assert!(matches!(inputs[4].value, Value::Text(_)), "api key should be Text");
}

/// Output types are correct.
#[test]
fn test_output_types() {
    let outputs = OpAiEdit::create_outputs();
    assert!(matches!(outputs[0].value, Value::Image { .. }));
    assert!(matches!(outputs[1].value, Value::Integer(_)));
    assert!(matches!(outputs[2].value, Value::Integer(_)));
}

/// Default model is "dall-e-2".
#[test]
fn test_default_model() {
    let inputs = OpAiEdit::create_inputs();
    let Value::Text(model) = &inputs[2].value else { panic!("Expected Text") };
    assert_eq!(model, "dall-e-2");
}

/// Default size is "1024x1024".
#[test]
fn test_default_size() {
    let inputs = OpAiEdit::create_inputs();
    let Value::Text(size) = &inputs[3].value else { panic!("Expected Text") };
    assert_eq!(size, "1024x1024");
}

/// Empty prompt produces input error on index 1.
#[tokio::test]
async fn test_empty_prompt_error() {
    let mut inputs = OpAiEdit::create_inputs();
    // prompt is empty by default
    inputs[4].value = Value::Text("sk-test".to_string());

    let result = OpAiEdit::run(&mut inputs).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(!err.input_errors.is_empty());
    assert_eq!(err.input_errors[0].0, 1); // index 1 = prompt
}

/// No API key returns descriptive node error.
#[tokio::test]
async fn test_no_api_key_error() {
    let mut inputs = OpAiEdit::create_inputs();
    inputs[1].value = Value::Text("make it blue".to_string());
    // api key is empty

    let result = OpAiEdit::run(&mut inputs).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.node_error.is_some());
    assert!(err.node_error.unwrap().contains("API key required"));
}
