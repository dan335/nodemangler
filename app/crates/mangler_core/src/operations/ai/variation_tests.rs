use super::*;
use crate::value::Value;

/// Node name is "ai variation".
#[test]
fn test_settings() {
    let settings = OpAiVariation::settings();
    assert_eq!(settings.name, "ai variation");
    assert!(!settings.description.is_empty());
}

/// Has 4 inputs with correct names.
#[test]
fn test_create_inputs() {
    let inputs = OpAiVariation::create_inputs();
    assert_eq!(inputs.len(), 4);
    assert_eq!(inputs[0].name, "image");
    assert_eq!(inputs[1].name, "model");
    assert_eq!(inputs[2].name, "size");
    assert_eq!(inputs[3].name, "api key");
}

/// Has 3 outputs: image, width, height.
#[test]
fn test_create_outputs() {
    let outputs = OpAiVariation::create_outputs();
    assert_eq!(outputs.len(), 3);
    assert_eq!(outputs[0].name, "image");
    assert_eq!(outputs[1].name, "width");
    assert_eq!(outputs[2].name, "height");
}

/// Input types are correct.
#[test]
fn test_input_types() {
    let inputs = OpAiVariation::create_inputs();
    assert!(matches!(inputs[0].value, Value::Image { .. }), "image input should be Image");
    assert!(matches!(inputs[1].value, Value::Text(_)), "model should be Text");
    assert!(matches!(inputs[2].value, Value::Text(_)), "size should be Text");
    assert!(matches!(inputs[3].value, Value::Text(_)), "api key should be Text");
}

/// Output types are correct.
#[test]
fn test_output_types() {
    let outputs = OpAiVariation::create_outputs();
    assert!(matches!(outputs[0].value, Value::Image { .. }));
    assert!(matches!(outputs[1].value, Value::Integer(_)));
    assert!(matches!(outputs[2].value, Value::Integer(_)));
}

/// Default model is "dall-e-2".
#[test]
fn test_default_model() {
    let inputs = OpAiVariation::create_inputs();
    let Value::Text(model) = &inputs[1].value else { panic!("Expected Text") };
    assert_eq!(model, "dall-e-2");
}

/// Default size is "1024x1024".
#[test]
fn test_default_size() {
    let inputs = OpAiVariation::create_inputs();
    let Value::Text(size) = &inputs[2].value else { panic!("Expected Text") };
    assert_eq!(size, "1024x1024");
}

/// No API key returns descriptive node error.
#[tokio::test]
async fn test_no_api_key_error() {
    let mut inputs = OpAiVariation::create_inputs();
    // api key is empty

    let result = OpAiVariation::run(&mut inputs).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.node_error.is_some());
    assert!(err.node_error.unwrap().contains("API key required"));
}

/// API key default is empty.
#[test]
fn test_api_key_default_empty() {
    let inputs = OpAiVariation::create_inputs();
    let Value::Text(key) = &inputs[3].value else { panic!("Expected Text") };
    assert!(key.is_empty());
}
