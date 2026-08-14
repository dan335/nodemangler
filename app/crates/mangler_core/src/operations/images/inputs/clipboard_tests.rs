//! Tests for the clipboard input node.
//!
//! The *capture* path needs a real OS clipboard, so it is only exercised for
//! the property that holds either way (the pulse is consumed). The *restore*
//! path — the one that fixes losing your image on reload — needs no clipboard
//! at all and is tested directly.

use super::*;
use crate::operations::images::inputs::encode_png_base64;

/// A recognisable image: a red/green/blue/white quadrant grid.
fn quadrants() -> FloatImage {
    let mut img = FloatImage::new(4, 4, 4);
    for y in 0..4 {
        for x in 0..4 {
            let px = match (x < 2, y < 2) {
                (true, true) => [1.0, 0.0, 0.0, 1.0],
                (false, true) => [0.0, 1.0, 0.0, 1.0],
                (true, false) => [0.0, 0.0, 1.0, 1.0],
                (false, false) => [1.0, 1.0, 1.0, 1.0],
            };
            img.put_pixel(x, y, &px);
        }
    }
    img
}

/// The node's own inputs, with `embedded_image` pre-loaded as a previous
/// capture would have left it.
fn inputs_with_stored(image: &FloatImage) -> Vec<Input> {
    let mut inputs = OpImageInputClipboard::create_inputs();
    inputs[CAPTURE].embedded_image = Some(encode_png_base64(image).unwrap());
    inputs
}

#[tokio::test]
async fn test_clipboard_input_settings() {
    let s = OpImageInputClipboard::settings();
    assert!(!s.name.is_empty());
    assert!(!OpImageInputClipboard::create_inputs().is_empty());
    assert!(!OpImageInputClipboard::create_outputs().is_empty());
}

#[tokio::test]
async fn the_capture_control_is_a_momentary_button() {
    // Not a Trigger: a trigger's fingerprint is constant, so `run` could not
    // tell a fresh press from "the graph just loaded", and would re-sample the
    // clipboard on every open — the behaviour that lost the image.
    let inputs = OpImageInputClipboard::create_inputs();
    assert!(matches!(inputs[CAPTURE].value, Value::Bool(false)));
    assert!(matches!(inputs[CAPTURE].settings, Some(InputSettings::Button)));
}

#[tokio::test]
async fn a_stored_image_is_restored_without_touching_the_clipboard() {
    // The bug this fixes: reopening a saved graph used to re-read the OS
    // clipboard, which by then holds something else or nothing.
    let source = quadrants();
    let mut inputs = inputs_with_stored(&source);

    let result = OpImageInputClipboard::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.dimensions(), (4, 4));
            assert_eq!(data.as_raw(), source.as_raw(), "restored pixels should be exact");
        }
        other => panic!("Expected Image, got {other:?}"),
    }
    assert!(matches!(result.responses[1].value, Value::Integer(4)), "width output");
    assert!(matches!(result.responses[2].value, Value::Integer(4)), "height output");
}

#[tokio::test]
async fn restoring_leaves_the_stored_image_alone() {
    // A restore must not re-encode: the graph file's bytes should be identical
    // after opening a graph and doing nothing, or every open would dirty it.
    let source = quadrants();
    let mut inputs = inputs_with_stored(&source);
    let before = inputs[CAPTURE].embedded_image.clone();

    OpImageInputClipboard::run(&mut inputs).await.unwrap();
    assert_eq!(inputs[CAPTURE].embedded_image, before);
}

#[tokio::test]
async fn repeated_restores_do_not_degrade_the_image() {
    // Three opens in a row must be pixel-identical to one.
    let source = quadrants();
    let mut inputs = inputs_with_stored(&source);
    for _ in 0..3 {
        let result = OpImageInputClipboard::run(&mut inputs).await.unwrap();
        let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
        assert_eq!(data.as_raw(), source.as_raw());
    }
}

#[tokio::test]
async fn a_corrupt_stored_image_reports_an_error() {
    // Better than a blank frame the user would then save over the real data.
    let mut inputs = OpImageInputClipboard::create_inputs();
    inputs[CAPTURE].embedded_image = Some("not a png".to_string());

    let err = OpImageInputClipboard::run(&mut inputs).await.unwrap_err();
    assert!(err.node_error.is_some());
    assert!(err.input_errors.is_empty());
}

#[tokio::test]
async fn pressing_consumes_the_pulse_whatever_the_clipboard_holds() {
    // The press must not survive the run, or the node would re-capture on every
    // subsequent tick. Asserted regardless of the outcome, because whether this
    // machine's clipboard holds an image is not something a test can control.
    let mut inputs = inputs_with_stored(&quadrants());
    inputs[CAPTURE].value = Value::Bool(true);

    let _ = OpImageInputClipboard::run(&mut inputs).await;
    assert!(
        matches!(inputs[CAPTURE].value, Value::Bool(false)),
        "the capture pulse should be consumed"
    );
}

#[tokio::test]
async fn a_driven_capture_input_is_not_reset() {
    // When wired upstream the value is not ours to rewrite — same rule the
    // output nodes' save button follows.
    let mut inputs = inputs_with_stored(&quadrants());
    inputs[CAPTURE].value = Value::Bool(true);
    inputs[CAPTURE].connection = Some(("upstream".to_string(), 0));

    let _ = OpImageInputClipboard::run(&mut inputs).await;
    assert!(matches!(inputs[CAPTURE].value, Value::Bool(true)));
}
