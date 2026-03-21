//! Tests for FloatImage serialization and deserialization.

use super::*;
use crate::float_image::FloatImage;
use std::sync::Arc;

#[test]
/// Round-trip serialization should preserve all image data.
fn test_serde_round_trip() {
    let img = FloatImage::from_pixel(2, 3, 4, &[0.1, 0.2, 0.3, 0.4]);
    let arc = Arc::new(img);

    // Serialize to JSON
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Wrapper {
        #[serde(with = "super")]
        image: Arc<FloatImage>,
    }

    let wrapper = Wrapper { image: arc.clone() };
    let json = serde_json::to_string(&wrapper).expect("serialize failed");

    // Deserialize back
    let decoded: Wrapper = serde_json::from_str(&json).expect("deserialize failed");

    assert_eq!(decoded.image.width(), 2);
    assert_eq!(decoded.image.height(), 3);
    assert_eq!(decoded.image.channels(), 4);

    // Verify pixel data matches
    for (original, decoded) in arc.pixels().zip(decoded.image.pixels()) {
        for (a, b) in original.iter().zip(decoded.iter()) {
            assert!((a - b).abs() < 1e-6, "pixel mismatch: {} vs {}", a, b);
        }
    }
}

#[test]
/// Single-channel images should serialize and deserialize correctly.
fn test_serde_1ch() {
    let img = FloatImage::from_pixel(1, 1, 1, &[0.5]);
    let arc = Arc::new(img);

    #[derive(serde::Serialize, serde::Deserialize)]
    struct Wrapper {
        #[serde(with = "super")]
        image: Arc<FloatImage>,
    }

    let wrapper = Wrapper { image: arc };
    let json = serde_json::to_string(&wrapper).expect("serialize failed");
    let decoded: Wrapper = serde_json::from_str(&json).expect("deserialize failed");

    assert_eq!(decoded.image.channels(), 1);
    assert!((decoded.image.get_pixel(0, 0)[0] - 0.5).abs() < 1e-6);
}
