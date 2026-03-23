use super::*;
use crate::value::Value;

/// Node name is "ai generate".
#[test]
fn test_settings() {
    let settings = OpAiGenerate::settings();
    assert_eq!(settings.name, "ai generate");
    assert!(!settings.description.is_empty());
}

/// Has 5 inputs with correct names.
#[test]
fn test_create_inputs() {
    let inputs = OpAiGenerate::create_inputs();
    assert_eq!(inputs.len(), 5);
    assert_eq!(inputs[0].name, "prompt");
    assert_eq!(inputs[1].name, "model");
    assert_eq!(inputs[2].name, "size");
    assert_eq!(inputs[3].name, "quality");
    assert_eq!(inputs[4].name, "api key");
}

/// Has 4 outputs: image, width, height, revised prompt.
#[test]
fn test_create_outputs() {
    let outputs = OpAiGenerate::create_outputs();
    assert_eq!(outputs.len(), 4);
    assert_eq!(outputs[0].name, "image");
    assert_eq!(outputs[1].name, "width");
    assert_eq!(outputs[2].name, "height");
    assert_eq!(outputs[3].name, "revised prompt");
}

/// All inputs are Text type.
#[test]
fn test_input_types() {
    let inputs = OpAiGenerate::create_inputs();
    for input in &inputs {
        assert!(matches!(input.value, Value::Text(_)), "Input '{}' should be Text", input.name);
    }
}

/// Outputs have correct types.
#[test]
fn test_output_types() {
    let outputs = OpAiGenerate::create_outputs();
    assert!(matches!(outputs[0].value, Value::Image { .. }));
    assert!(matches!(outputs[1].value, Value::Integer(_)));
    assert!(matches!(outputs[2].value, Value::Integer(_)));
    assert!(matches!(outputs[3].value, Value::Text(_)));
}

/// Default model is "dall-e-3".
#[test]
fn test_default_model() {
    let inputs = OpAiGenerate::create_inputs();
    let Value::Text(model) = &inputs[1].value else { panic!("Expected Text") };
    assert_eq!(model, "dall-e-3");
}

/// Default size is "1024x1024".
#[test]
fn test_default_size() {
    let inputs = OpAiGenerate::create_inputs();
    let Value::Text(size) = &inputs[2].value else { panic!("Expected Text") };
    assert_eq!(size, "1024x1024");
}

/// Default quality is "standard".
#[test]
fn test_default_quality() {
    let inputs = OpAiGenerate::create_inputs();
    let Value::Text(quality) = &inputs[3].value else { panic!("Expected Text") };
    assert_eq!(quality, "standard");
}

/// Request body has correct structure.
#[test]
fn test_build_request_body() {
    let body = OpAiGenerate::build_request_body("a sunset", "dall-e-3", "1024x1024", "hd");
    assert_eq!(body["model"], "dall-e-3");
    assert_eq!(body["prompt"], "a sunset");
    assert_eq!(body["size"], "1024x1024");
    assert_eq!(body["quality"], "hd");
    assert_eq!(body["response_format"], "b64_json");
    assert_eq!(body["n"], 1);
}

/// Empty prompt produces input error on index 0.
#[tokio::test]
async fn test_empty_prompt_error() {
    let mut inputs = OpAiGenerate::create_inputs();
    // prompt is already empty by default
    // Set a dummy API key so we don't get key error first.
    inputs[4].value = Value::Text("sk-test".to_string());

    let result = OpAiGenerate::run(&mut inputs).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(!err.input_errors.is_empty());
    assert_eq!(err.input_errors[0].0, 0); // index 0 = prompt
}

/// No API key returns descriptive node error.
#[tokio::test]
async fn test_no_api_key_error() {
    let mut inputs = OpAiGenerate::create_inputs();
    inputs[0].value = Value::Text("a sunset over mountains".to_string());
    // api key is empty, env var should not be set for this test key name

    let result = OpAiGenerate::run(&mut inputs).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.node_error.is_some());
    assert!(err.node_error.unwrap().contains("API key required"));
}

/// API key from input is used (prompt empty check triggers first though).
#[test]
fn test_api_key_input_default_empty() {
    let inputs = OpAiGenerate::create_inputs();
    let Value::Text(key) = &inputs[4].value else { panic!("Expected Text") };
    assert!(key.is_empty());
}
