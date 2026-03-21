//! Tests for the FloatImage type.

use super::*;
use image::DynamicImage;

#[test]
/// A new FloatImage should be zero-filled with the correct dimensions.
fn test_new_zero_filled() {
    let img = FloatImage::new(4, 3, 2);
    assert_eq!(img.width(), 4);
    assert_eq!(img.height(), 3);
    assert_eq!(img.channels(), 2);
    assert_eq!(img.as_raw().len(), 4 * 3 * 2);
    assert!(img.as_raw().iter().all(|&v| v == 0.0));
}

#[test]
/// from_pixel should fill every pixel with the given value.
fn test_from_pixel() {
    let img = FloatImage::from_pixel(2, 2, 3, &[0.1, 0.2, 0.3]);
    for px in img.pixels() {
        assert_eq!(px, &[0.1, 0.2, 0.3]);
    }
}

#[test]
/// get_pixel and put_pixel should read/write correctly.
fn test_get_put_pixel() {
    let mut img = FloatImage::new(3, 3, 1);
    img.put_pixel(1, 2, &[0.75]);
    assert_eq!(img.get_pixel(1, 2), &[0.75]);
    assert_eq!(img.get_pixel(0, 0), &[0.0]);
}

#[test]
/// enumerate_pixels should yield correct (x, y) coordinates.
fn test_enumerate_pixels() {
    let img = FloatImage::from_pixel(2, 2, 1, &[1.0]);
    let coords: Vec<(u32, u32)> = img.enumerate_pixels().map(|(x, y, _)| (x, y)).collect();
    assert_eq!(coords, vec![(0, 0), (1, 0), (0, 1), (1, 1)]);
}

#[test]
/// from_dynamic should correctly convert a Luma8 image to 1-channel float.
fn test_from_dynamic_luma8() {
    let buf = image::GrayImage::from_raw(2, 1, vec![0, 255]).unwrap();
    let dyn_img = DynamicImage::ImageLuma8(buf);
    let fi = FloatImage::from_dynamic(&dyn_img);
    assert_eq!(fi.channels(), 1);
    assert_eq!(fi.width(), 2);
    assert_eq!(fi.height(), 1);
    assert!((fi.get_pixel(0, 0)[0] - 0.0).abs() < 1e-5);
    assert!((fi.get_pixel(1, 0)[0] - 1.0).abs() < 1e-5);
}

#[test]
/// from_dynamic should correctly convert an Rgba8 image to 4-channel float.
fn test_from_dynamic_rgba8() {
    let buf = image::RgbaImage::from_raw(1, 1, vec![255, 128, 0, 255]).unwrap();
    let dyn_img = DynamicImage::ImageRgba8(buf);
    let fi = FloatImage::from_dynamic(&dyn_img);
    assert_eq!(fi.channels(), 4);
    assert!((fi.get_pixel(0, 0)[0] - 1.0).abs() < 1e-5);
    assert!((fi.get_pixel(0, 0)[1] - 128.0 / 255.0).abs() < 1e-3);
    assert!((fi.get_pixel(0, 0)[2] - 0.0).abs() < 1e-5);
    assert!((fi.get_pixel(0, 0)[3] - 1.0).abs() < 1e-5);
}

#[test]
/// from_dynamic should correctly convert an Rgb32F image to 3-channel float.
fn test_from_dynamic_rgb32f() {
    let buf = image::Rgb32FImage::from_raw(1, 1, vec![0.5, 0.25, 0.75]).unwrap();
    let dyn_img = DynamicImage::ImageRgb32F(buf);
    let fi = FloatImage::from_dynamic(&dyn_img);
    assert_eq!(fi.channels(), 3);
    assert_eq!(fi.get_pixel(0, 0), &[0.5, 0.25, 0.75]);
}

#[test]
/// Round-trip: from_dynamic → to_dynamic should preserve data for 4-channel images.
fn test_round_trip_rgba32f() {
    let original = vec![0.1, 0.2, 0.3, 0.4];
    let buf = image::Rgba32FImage::from_raw(1, 1, original.clone()).unwrap();
    let dyn_img = DynamicImage::ImageRgba32F(buf);
    let fi = FloatImage::from_dynamic(&dyn_img);
    let result = fi.to_dynamic();

    // Should be Rgba32F
    if let DynamicImage::ImageRgba32F(buf) = result {
        let data: Vec<f32> = buf.into_raw();
        for (a, b) in data.iter().zip(original.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    } else {
        panic!("Expected ImageRgba32F, got {:?}", result.color());
    }
}

#[test]
/// Round-trip: 1-channel should go through Luma16 with minimal precision loss.
fn test_round_trip_1ch() {
    let fi = FloatImage::from_pixel(1, 1, 1, &[0.5]);
    let dyn_img = fi.to_dynamic();

    // Should be Luma16
    assert!(matches!(dyn_img, DynamicImage::ImageLuma16(_)));

    let fi2 = FloatImage::from_dynamic(&dyn_img);
    assert_eq!(fi2.channels(), 1);
    // Luma16 round-trip precision: 1/65535 ≈ 0.000015
    assert!((fi2.get_pixel(0, 0)[0] - 0.5).abs() < 0.001);
}

#[test]
/// to_rgba8 should expand 1-channel to grayscale RGBA.
fn test_to_rgba8_1ch() {
    let fi = FloatImage::from_pixel(1, 1, 1, &[0.5]);
    let rgba = fi.to_rgba8();
    let px = rgba.get_pixel(0, 0).0;
    // 0.5 * 255 ≈ 127 or 128
    assert!((px[0] as i32 - 128).abs() <= 1);
    assert_eq!(px[0], px[1]); // R == G == B
    assert_eq!(px[0], px[2]);
    assert_eq!(px[3], 255);   // alpha = 255
}

#[test]
/// to_rgba8 should correctly convert 3-channel images.
fn test_to_rgba8_3ch() {
    let fi = FloatImage::from_pixel(1, 1, 3, &[1.0, 0.0, 0.5]);
    let rgba = fi.to_rgba8();
    let px = rgba.get_pixel(0, 0).0;
    assert_eq!(px[0], 255);
    assert_eq!(px[1], 0);
    assert!((px[2] as i32 - 128).abs() <= 1);
    assert_eq!(px[3], 255); // alpha always 255 for 3ch
}

#[test]
/// Bilinear sampling at exact pixel centers should return exact pixel values.
fn test_bilinear_sample_exact() {
    let mut img = FloatImage::new(3, 3, 1);
    img.put_pixel(1, 1, &[0.8]);
    let mut out = [0.0f32];
    img.bilinear_sample(1.0, 1.0, &mut out);
    assert!((out[0] - 0.8).abs() < 1e-5);
}

#[test]
/// Bilinear sampling between pixels should interpolate correctly.
fn test_bilinear_sample_interpolation() {
    let mut img = FloatImage::new(2, 1, 1);
    img.put_pixel(0, 0, &[0.0]);
    img.put_pixel(1, 0, &[1.0]);
    let mut out = [0.0f32];
    img.bilinear_sample(0.5, 0.0, &mut out);
    assert!((out[0] - 0.5).abs() < 1e-5);
}

#[test]
/// Bilinear sampling should clamp out-of-bounds coordinates to edge pixels.
fn test_bilinear_sample_clamp() {
    let img = FloatImage::from_pixel(2, 2, 1, &[0.5]);
    let mut out = [0.0f32];
    img.bilinear_sample(-5.0, -5.0, &mut out);
    assert!((out[0] - 0.5).abs() < 1e-5);
    img.bilinear_sample(100.0, 100.0, &mut out);
    assert!((out[0] - 0.5).abs() < 1e-5);
}

#[test]
/// Bilinear sampling with multiple channels should interpolate all channels.
fn test_bilinear_sample_multichannel() {
    let mut img = FloatImage::new(2, 1, 3);
    img.put_pixel(0, 0, &[0.0, 0.0, 0.0]);
    img.put_pixel(1, 0, &[1.0, 0.5, 0.25]);
    let mut out = [0.0f32; 3];
    img.bilinear_sample(0.5, 0.0, &mut out);
    assert!((out[0] - 0.5).abs() < 1e-5);
    assert!((out[1] - 0.25).abs() < 1e-5);
    assert!((out[2] - 0.125).abs() < 1e-5);
}

#[test]
/// Resize should produce the correct dimensions and preserve channel count.
fn test_resize_dimensions() {
    let img = FloatImage::from_pixel(4, 4, 2, &[0.5, 1.0]);
    let resized = img.resize(2, 2);
    assert_eq!(resized.width(), 2);
    assert_eq!(resized.height(), 2);
    assert_eq!(resized.channels(), 2);
}

#[test]
/// Resize of a uniform image should produce a uniform result.
fn test_resize_uniform() {
    let img = FloatImage::from_pixel(4, 4, 1, &[0.7]);
    let resized = img.resize(8, 8);
    for px in resized.pixels() {
        assert!((px[0] - 0.7).abs() < 1e-4);
    }
}

#[test]
/// from_raw should return None for mismatched data length.
fn test_from_raw_invalid() {
    assert!(FloatImage::from_raw(2, 2, 3, vec![0.0; 11]).is_none());
    assert!(FloatImage::from_raw(2, 2, 3, vec![0.0; 12]).is_some());
}

#[test]
#[should_panic(expected = "channels must be 1–4")]
/// new should panic with channels outside 1-4 range.
fn test_invalid_channels() {
    FloatImage::new(1, 1, 5);
}

#[test]
/// pixels_mut should allow modifying pixel data in place.
fn test_pixels_mut() {
    let mut img = FloatImage::new(2, 2, 1);
    for px in img.pixels_mut() {
        px[0] = 0.42;
    }
    for px in img.pixels() {
        assert!((px[0] - 0.42).abs() < 1e-6);
    }
}
