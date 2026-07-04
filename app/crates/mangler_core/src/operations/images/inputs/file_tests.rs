use super::*;

#[tokio::test]
async fn test_file_input_settings() {
    let s = OpImageInputFile::settings();
    assert!(!s.name.is_empty());
    assert!(!OpImageInputFile::create_inputs().is_empty());
    assert!(!OpImageInputFile::create_outputs().is_empty());
}

#[tokio::test]
async fn test_file_input_exact_settings() {
    let s = OpImageInputFile::settings();
    assert_eq!(s.name, "from file");
    assert_eq!(OpImageInputFile::create_inputs().len(), 1);
    assert_eq!(OpImageInputFile::create_outputs().len(), 3);
}

#[tokio::test]
async fn test_file_input_nonexistent_path_returns_error() {
    use crate::input::Input;
    let mut inputs = vec![
        Input::new("path".to_string(), Value::Path(PathBuf::from("/this/does/not/exist.png")), None, None),
    ];
    let result = OpImageInputFile::run(&mut inputs).await;
    assert!(result.is_err(), "loading from nonexistent path should fail");
}

/// Runs the operation on `path` and returns (image, width, height).
async fn load(path: PathBuf) -> (std::sync::Arc<crate::float_image::FloatImage>, i32, i32) {
    use crate::input::Input;
    let mut inputs = vec![Input::new("path".to_string(), Value::Path(path), None, None)];
    let result = OpImageInputFile::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!("expected image output") };
    let Value::Integer(w) = result.responses[1].value else { panic!("expected width output") };
    let Value::Integer(h) = result.responses[2].value else { panic!("expected height output") };
    (data.clone(), w, h)
}

#[tokio::test]
async fn test_file_input_jxl() {
    // Losslessly encoded 4x4 RGBA fixture (cjxl -d 0): pixel (x, y) is
    // (x*60, y*60, 100, 255 - x*40) in 8-bit sRGB.
    let path = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/gradient_4x4.jxl"));
    let (img, w, h) = load(path).await;
    assert_eq!((w, h), (4, 4));
    assert_eq!(img.channels(), 4, "alpha-carrying JXL should decode to 4 channels");
    for y in 0..4u32 {
        for x in 0..4u32 {
            let expected = [
                (x * 60) as f32 / 255.0,
                (y * 60) as f32 / 255.0,
                100.0 / 255.0,
                (255 - x * 40) as f32 / 255.0,
            ];
            let px = img.get_pixel(x, y);
            for c in 0..4 {
                assert!(
                    (px[c] - expected[c]).abs() < 0.002,
                    "pixel ({}, {}) channel {}: got {}, expected {}",
                    x, y, c, px[c], expected[c]
                );
            }
        }
    }
}

/// Builds a minimal flat PSD file: RGB, 8-bit, raw (uncompressed) image data.
fn minimal_psd(width: u32, height: u32, rgb_pixels: &[[u8; 3]]) -> Vec<u8> {
    assert_eq!(rgb_pixels.len(), (width * height) as usize);
    let mut out = vec![];
    out.extend_from_slice(b"8BPS"); // signature
    out.extend_from_slice(&1u16.to_be_bytes()); // version
    out.extend_from_slice(&[0u8; 6]); // reserved
    out.extend_from_slice(&3u16.to_be_bytes()); // channels
    out.extend_from_slice(&height.to_be_bytes());
    out.extend_from_slice(&width.to_be_bytes());
    out.extend_from_slice(&8u16.to_be_bytes()); // bit depth
    out.extend_from_slice(&3u16.to_be_bytes()); // color mode: RGB
    out.extend_from_slice(&0u32.to_be_bytes()); // color mode data: empty
    out.extend_from_slice(&0u32.to_be_bytes()); // image resources: empty
    out.extend_from_slice(&0u32.to_be_bytes()); // layer & mask info: empty
    out.extend_from_slice(&0u16.to_be_bytes()); // compression: raw
    for channel in 0..3 {
        for px in rgb_pixels {
            out.push(px[channel]); // planar channel data
        }
    }
    out
}

#[tokio::test]
async fn test_file_input_psd() {
    let pixels = [[255, 0, 0], [0, 255, 0], [0, 0, 255], [40, 80, 120]];
    let tmp = std::env::temp_dir().join("nodemangler_test_input_psd");
    std::fs::create_dir_all(&tmp).unwrap();
    let path = tmp.join("sample.psd");
    std::fs::write(&path, minimal_psd(2, 2, &pixels)).unwrap();

    let (img, w, h) = load(path.clone()).await;
    assert_eq!((w, h), (2, 2));
    assert_eq!(img.channels(), 4, "psd composite decodes as RGBA");
    for (i, expected) in pixels.iter().enumerate() {
        let px = img.get_pixel(i as u32 % 2, i as u32 / 2);
        for c in 0..3 {
            let want = expected[c] as f32 / 255.0;
            assert!((px[c] - want).abs() < 0.002, "pixel {} channel {}: got {}, expected {}", i, c, px[c], want);
        }
        assert!((px[3] - 1.0).abs() < 0.002, "flat RGB psd should be opaque");
    }

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&tmp).ok();
}
