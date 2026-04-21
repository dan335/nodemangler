use super::*;
use crate::value::Value;

/// Node name is "ai variation".
#[test]
fn test_settings() {
    let settings = OpAiVariation::settings();
    assert_eq!(settings.name, "ai variation");
    assert!(!settings.description.is_empty());
}

/// Has 3 inputs with correct names.
#[test]
fn test_create_inputs() {
    let inputs = OpAiVariation::create_inputs();
    assert_eq!(inputs.len(), 3);
    assert_eq!(inputs[0].name, "image");
    assert_eq!(inputs[1].name, "model");
    assert_eq!(inputs[2].name, "size");
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

