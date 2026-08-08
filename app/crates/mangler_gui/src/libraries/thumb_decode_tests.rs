//! Pure dispatch tests for library thumbnail decode (no heavy fixtures).

use std::path::Path;

use super::thumb_dispatch_kind;

#[test]
fn dispatch_jpeg_uses_scaled_path() {
    assert_eq!(thumb_dispatch_kind(Path::new("photo.JPG")), "jpeg_scaled");
    assert_eq!(thumb_dispatch_kind(Path::new("photo.jpeg")), "jpeg_scaled");
}

#[test]
fn dispatch_png_uses_image_crate() {
    assert_eq!(thumb_dispatch_kind(Path::new("a.png")), "image_crate");
    assert_eq!(thumb_dispatch_kind(Path::new("a.tif")), "image_crate");
}

#[test]
fn dispatch_special_formats_use_float_path() {
    assert_eq!(thumb_dispatch_kind(Path::new("a.jxl")), "float_image");
    assert_eq!(thumb_dispatch_kind(Path::new("a.psd")), "float_image");
    assert_eq!(thumb_dispatch_kind(Path::new("a.heic")), "float_image");
}

#[test]
fn decode_tiny_png_roundtrip() {
    let dir = std::env::temp_dir().join(format!(
        "mangler_thumb_decode_{}",
        std::process::id()
    ));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("tiny.png");
    let img = image::RgbImage::from_pixel(64, 48, image::Rgb([10u8, 20, 30]));
    img.save(&path).unwrap();

    let (pixels, size) = super::decode_thumb(&path).expect("png thumb");
    assert_eq!(size[0] * size[1] * 4, pixels.len());
    assert!(size[0] <= crate::libraries::thumb_decode::LIBRARY_THUMB_MAX as usize);
    assert!(size[1] <= crate::libraries::thumb_decode::LIBRARY_THUMB_MAX as usize);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn decode_jpeg_scaled_roundtrip() {
    let dir = std::env::temp_dir().join(format!(
        "mangler_thumb_jpeg_{}",
        std::process::id()
    ));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("big.jpg");
    // 512×512 is enough to exercise the scaled path without a multi-MP fixture.
    let img = image::RgbImage::from_fn(512, 512, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
    });
    img.save_with_format(&path, image::ImageFormat::Jpeg).unwrap();

    let (pixels, size) = super::decode_thumb(&path).expect("jpeg thumb");
    assert_eq!(size[0] * size[1] * 4, pixels.len());
    assert!(size[0] <= crate::libraries::thumb_decode::LIBRARY_THUMB_MAX as usize);
    assert!(size[1] <= crate::libraries::thumb_decode::LIBRARY_THUMB_MAX as usize);
    assert!(size[0] >= 8 && size[1] >= 8);

    let _ = std::fs::remove_dir_all(&dir);
}
