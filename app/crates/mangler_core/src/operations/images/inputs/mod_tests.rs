//! Tests for the graph-file image embedding format.
//!
//! These back `from clipboard`'s promise that a captured image survives a save.
//! The round-trip has to be *exact* for 8-bit content, because the node writes
//! the encoded string back into the graph on every capture: a codec that drifted
//! by a level per cycle would slowly degrade an image the user never re-captured.

use super::*;

/// An image whose every channel is a distinct 8-bit level, so a codec that
/// shifted or truncated a level shows up rather than averaging out.
fn stepped(w: u32, h: u32) -> FloatImage {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as f32;
            let level = |k: f32| ((i + k) % 256.0) / 255.0;
            img.put_pixel(x, y, &[level(0.0), level(64.0), level(128.0), level(192.0)]);
        }
    }
    img
}

#[test]
fn every_8_bit_level_round_trips_exactly() {
    // 256 pixels, one per level, on every channel. This is the property the
    // clipboard node depends on: its pixels arrived as RGBA8, so encoding them
    // back to PNG must be lossless or repeated captures would drift.
    let source = stepped(16, 16);
    let encoded = encode_png_base64(&source).unwrap();
    let decoded = decode_png_base64(&encoded).unwrap();

    assert_eq!(decoded.dimensions(), source.dimensions());
    assert_eq!(decoded.channels(), 4);
    for (i, (a, b)) in source.as_raw().iter().zip(decoded.as_raw()).enumerate() {
        assert_eq!(a, b, "channel {i} changed: {a} -> {b}");
    }
}

#[test]
fn a_second_round_trip_changes_nothing() {
    // The node re-encodes whatever it just decoded is *not* true today, but a
    // future refactor could make it so; pinning idempotence here means that
    // change cannot quietly start eroding the image.
    let source = stepped(8, 8);
    let once = decode_png_base64(&encode_png_base64(&source).unwrap()).unwrap();
    let twice = decode_png_base64(&encode_png_base64(&once).unwrap()).unwrap();
    assert_eq!(once.as_raw(), twice.as_raw());
}

#[test]
fn fewer_channels_are_promoted_to_rgba() {
    // `to_rgba8` broadcasts grey across the colour channels and fills alpha, so
    // the decoded image is always 4-channel. Worth pinning: a 1-channel source
    // coming back as 1-channel would be a silently different node output.
    let mut grey = FloatImage::new(4, 4, 1);
    for y in 0..4 {
        for x in 0..4 {
            grey.put_pixel(x, y, &[(x + y) as f32 / 6.0]);
        }
    }
    let decoded = decode_png_base64(&encode_png_base64(&grey).unwrap()).unwrap();
    assert_eq!(decoded.channels(), 4);
    let px = decoded.get_pixel(0, 0);
    assert_eq!(px[0], px[1]);
    assert_eq!(px[1], px[2]);
    assert_eq!(px[3], 1.0, "alpha should be opaque");
}

#[test]
fn the_encoding_is_plain_base64() {
    // It goes into a JSON string field, so it must contain nothing that needs
    // escaping — no quotes, backslashes, or newlines.
    let encoded = encode_png_base64(&stepped(4, 4)).unwrap();
    assert!(!encoded.is_empty());
    assert!(
        encoded.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'='),
        "unexpected characters in {}",
        &encoded[..encoded.len().min(80)]
    );
}

#[test]
fn png_is_dramatically_smaller_than_the_raw_floats() {
    // The reason the format is PNG and not the raw f32 buffer `float_image_serde`
    // uses: that buffer would be serialised as a JSON array of byte *numbers*,
    // several characters each. A flat image compresses to almost nothing here.
    let flat = FloatImage::from_pixel(256, 256, 4, &[0.25, 0.5, 0.75, 1.0]);
    let encoded = encode_png_base64(&flat).unwrap();
    let raw_floats = 256 * 256 * 4 * 4; // bytes, before any JSON expansion
    assert!(
        encoded.len() < raw_floats / 100,
        "encoded {} bytes vs {raw_floats} raw",
        encoded.len()
    );
}

#[test]
fn corrupt_input_errors_rather_than_returning_a_blank() {
    // A blank fallback would be worse than an error: the node would look like
    // it worked, and the next save would write the blank over the real data.
    assert!(decode_png_base64("not base64!!!").is_err());
    assert!(decode_png_base64("").is_err(), "empty is not a PNG");
    // Valid base64, but not a PNG.
    let not_png = crate::operations::text::encoding::base64_encode(b"hello world");
    assert!(decode_png_base64(&not_png).is_err());
}
