use super::*;

#[test]
fn test_settings() {
    let settings = OpImageInputRaw::settings();
    assert_eq!(settings.name, "from raw");
    assert!(!settings.description.is_empty());
    assert!(!settings.help.is_empty());
}

#[test]
fn test_input_and_output_shape() {
    let inputs = OpImageInputRaw::create_inputs();
    assert_eq!(inputs.len(), 6);
    assert_eq!(inputs[PATH].name, "path");
    assert_eq!(inputs[WHITE_BALANCE].name, "white balance");
    assert_eq!(inputs[OUTPUT_ENCODING].name, "output");
    assert_eq!(inputs[DEMOSAIC].name, "demosaic");
    assert_eq!(inputs[EXPOSURE].name, "exposure");
    assert_eq!(inputs[MAX_SIZE].name, "max size");

    let outputs = OpImageInputRaw::create_outputs();
    assert_eq!(outputs.len(), 3);
    assert_eq!(outputs[0].name, "output");
    assert_eq!(outputs[1].name, "width");
    assert_eq!(outputs[2].name, "height");
}

#[test]
fn test_defaults_are_camera_faithful_and_memory_safe() {
    let inputs = OpImageInputRaw::create_inputs();
    assert!(matches!(&inputs[WHITE_BALANCE].value, Value::Text(t) if t == "as shot"));
    assert!(matches!(&inputs[OUTPUT_ENCODING].value, Value::Text(t) if t == "srgb"));
    assert!(matches!(inputs[DEMOSAIC].value, Value::Bool(true)));
    assert!(matches!(inputs[EXPOSURE].value, Value::Decimal(v) if v == 0.0));
    // A full-resolution default would make a modest edit chain allocate
    // gigabytes; the cap is the safety valve.
    assert!(matches!(inputs[MAX_SIZE].value, Value::Integer(DEFAULT_MAX_SIZE)));
}

/// The picker must offer raw files only — this node cannot open a PNG.
#[test]
fn test_path_filter_is_raw_only() {
    let inputs = OpImageInputRaw::create_inputs();
    let Some(InputSettings::Path { extension_filter, .. }) = &inputs[PATH].settings else {
        panic!("path input must use a Path picker");
    };
    assert!(!extension_filter.iter().any(|e| e == "png"));
    assert!(!extension_filter.iter().any(|e| e == "jpg"));

    #[cfg(feature = "raw")]
    {
        assert!(extension_filter.iter().any(|e| e == "cr3"));
        assert!(extension_filter.iter().any(|e| e == "nef"));
    }
}

#[test]
fn test_dropdown_options_match_the_parsed_labels() {
    let inputs = OpImageInputRaw::create_inputs();

    let Some(InputSettings::Dropdown { options }) = &inputs[WHITE_BALANCE].settings else {
        panic!("white balance must be a dropdown");
    };
    // Every offered label must parse to a distinct mode, or the UI would show
    // an option that silently does nothing.
    let parsed: Vec<_> = options.iter().map(|o| RawWhiteBalance::from_label(o)).collect();
    assert_eq!(parsed, vec![
        RawWhiteBalance::AsShot,
        RawWhiteBalance::CameraNeutral,
        RawWhiteBalance::None,
    ]);

    let Some(InputSettings::Dropdown { options }) = &inputs[OUTPUT_ENCODING].settings else {
        panic!("output must be a dropdown");
    };
    assert_eq!(options, &vec!["srgb".to_string(), "linear".to_string()]);
}

#[tokio::test]
async fn test_empty_path_errors_without_panicking() {
    let mut inputs = OpImageInputRaw::create_inputs();
    let result = OpImageInputRaw::run(&mut inputs).await;
    assert!(result.is_err(), "an empty path must produce an error, not a panic");
}

#[tokio::test]
async fn test_missing_file_errors() {
    let mut inputs = OpImageInputRaw::create_inputs();
    inputs[PATH].value = Value::Path(PathBuf::from("/nonexistent/nope.cr3"));
    let result = OpImageInputRaw::run(&mut inputs).await;
    assert!(result.is_err());
}

/// End-to-end run of the node against a real camera file. Skipped unless
/// `NODEMANGLER_RAW_FIXTURE` points at one.
#[cfg(feature = "raw")]
#[tokio::test]
async fn test_run_on_real_raw_fixture() {
    let Ok(path) = std::env::var("NODEMANGLER_RAW_FIXTURE") else { return };

    let mut inputs = OpImageInputRaw::create_inputs();
    inputs[PATH].value = Value::Path(PathBuf::from(path));
    inputs[MAX_SIZE].value = Value::Integer(512);

    let response = OpImageInputRaw::run(&mut inputs).await.expect("real raw must develop");
    assert_eq!(response.responses.len(), 3);

    let Value::Image { data, .. } = &response.responses[0].value else {
        panic!("first output must be an image");
    };
    assert_eq!(data.channels(), 3);
    assert!(data.width().max(data.height()) <= 512, "max size must be honoured");

    // The reported dimensions must describe the image actually emitted.
    let Value::Integer(width) = response.responses[1].value else { panic!("width must be an integer") };
    let Value::Integer(height) = response.responses[2].value else { panic!("height must be an integer") };
    assert_eq!((width as u32, height as u32), (data.width(), data.height()));
}

/// `demosaic` off must yield the single-channel sensor mosaic rather than RGB.
#[cfg(feature = "raw")]
#[tokio::test]
async fn test_demosaic_off_yields_single_channel() {
    let Ok(path) = std::env::var("NODEMANGLER_RAW_FIXTURE") else { return };

    let mut inputs = OpImageInputRaw::create_inputs();
    inputs[PATH].value = Value::Path(PathBuf::from(path));
    inputs[DEMOSAIC].value = Value::Bool(false);
    inputs[MAX_SIZE].value = Value::Integer(512);

    let response = OpImageInputRaw::run(&mut inputs).await.expect("mosaic must develop");
    let Value::Image { data, .. } = &response.responses[0].value else {
        panic!("first output must be an image");
    };
    assert_eq!(data.channels(), 1, "an undemosaiced raw is a single-channel mosaic");
}

/// Linear output must differ from sRGB output, and be darker in the midtones —
/// that is the signature of the gamma step having been skipped.
#[cfg(feature = "raw")]
#[tokio::test]
async fn test_linear_output_is_darker_than_srgb() {
    let Ok(path) = std::env::var("NODEMANGLER_RAW_FIXTURE") else { return };

    async fn mean_of(path: &str, encoding: &str) -> f64 {
        let mut inputs = OpImageInputRaw::create_inputs();
        inputs[PATH].value = Value::Path(PathBuf::from(path));
        inputs[OUTPUT_ENCODING].value = Value::Text(encoding.to_string());
        inputs[MAX_SIZE].value = Value::Integer(512);
        let response = OpImageInputRaw::run(&mut inputs).await.expect("must develop");
        let Value::Image { data, .. } = &response.responses[0].value else { unreachable!() };
        let slice = data.as_slice();
        slice.iter().map(|v| *v as f64).sum::<f64>() / slice.len() as f64
    }

    let srgb = mean_of(&path, "srgb").await;
    let linear = mean_of(&path, "linear").await;
    assert!(
        linear < srgb,
        "linear output should be darker than sRGB-encoded (linear={linear:.4}, srgb={srgb:.4})"
    );
}
