use super::*;
use crate::value::Value;

/// Node name is "ai edit".
#[test]
fn test_settings() {
    let settings = OpAiEdit::settings();
    assert_eq!(settings.name, "ai edit");
    assert!(!settings.description.is_empty());
}

/// Has 4 inputs with correct names.
#[test]
fn test_create_inputs() {
    let inputs = OpAiEdit::create_inputs();
    assert_eq!(inputs.len(), 4);
    assert_eq!(inputs[0].name, "image");
    assert_eq!(inputs[1].name, "mask");
    assert_eq!(inputs[2].name, "prompt");
    assert_eq!(inputs[3].name, "size");
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
    assert!(matches!(inputs[1].value, Value::Image { .. }), "mask input should be Image");
    assert!(matches!(inputs[2].value, Value::Text(_)), "prompt should be Text");
    assert!(matches!(inputs[3].value, Value::Text(_)), "size should be Text");
}

/// Output types are correct.
#[test]
fn test_output_types() {
    let outputs = OpAiEdit::create_outputs();
    assert!(matches!(outputs[0].value, Value::Image { .. }));
    assert!(matches!(outputs[1].value, Value::Integer(_)));
    assert!(matches!(outputs[2].value, Value::Integer(_)));
}

/// Default size is "1024x1024".
#[test]
fn test_default_size() {
    let inputs = OpAiEdit::create_inputs();
    let Value::Text(size) = &inputs[3].value else { panic!("Expected Text") };
    assert_eq!(size, "1024x1024");
}

/// Empty prompt produces input error on index 2.
#[tokio::test]
async fn test_empty_prompt_error() {
    let mut inputs = OpAiEdit::create_inputs();
    // prompt is empty by default
    // Set env var so we don't get key error first.
    std::env::set_var("OPENAI_API_KEY", "sk-test");

    let result = OpAiEdit::run(&mut inputs).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(!err.input_errors.is_empty());
    assert_eq!(err.input_errors[0].0, 2); // index 2 = prompt
}

/// Mask conversion: a black (0.0) pixel produces alpha=0 (transparent).
#[test]
fn test_mask_black_becomes_transparent() {
    use crate::float_image::FloatImage;
    // 1x1 black image (3-channel).
    let mask = FloatImage::from_pixel(1, 1, 3, &[0.0, 0.0, 0.0]);
    let png_bytes = OpAiEdit::mask_to_png_bytes(&mask).unwrap();
    // Decode and check alpha.
    let img = image::load_from_memory(&png_bytes).unwrap().into_rgba8();
    let pixel = img.get_pixel(0, 0);
    assert_eq!(pixel[3], 0, "Black mask pixel should have alpha=0 (transparent)");
}

/// Mask conversion: a white (1.0) pixel produces alpha=255 (opaque).
#[test]
fn test_mask_white_becomes_opaque() {
    use crate::float_image::FloatImage;
    // 1x1 white image (3-channel).
    let mask = FloatImage::from_pixel(1, 1, 3, &[1.0, 1.0, 1.0]);
    let png_bytes = OpAiEdit::mask_to_png_bytes(&mask).unwrap();
    // Decode and check alpha.
    let img = image::load_from_memory(&png_bytes).unwrap().into_rgba8();
    let pixel = img.get_pixel(0, 0);
    assert_eq!(pixel[3], 255, "White mask pixel should have alpha=255 (opaque)");
}
