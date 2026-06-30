//! Tests for the video-from-url loader.

use super::*;

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpVideoFromUrl::settings().name, "video from url");
    assert_eq!(OpVideoFromUrl::create_inputs().len(), 1);
    // Same eight outputs as `video from file`.
    assert_eq!(OpVideoFromUrl::create_outputs().len(), 8);
}

#[cfg(feature = "video")]
#[test]
fn cached_path_is_stable_and_url_specific() {
    let a = cached_video_path("https://example.com/clip.mp4");
    let b = cached_video_path("https://example.com/clip.mp4");
    let c = cached_video_path("https://example.com/other.mp4");
    assert_eq!(a, b, "same URL should map to the same cache path");
    assert_ne!(a, c, "different URLs should map to different cache paths");
}

#[cfg(feature = "video")]
#[test]
fn cached_path_preserves_video_extension() {
    // Extension is taken from the URL, with query/fragment stripped.
    let p = cached_video_path("https://example.com/dir/clip.webm?token=123");
    assert_eq!(p.extension().and_then(|e| e.to_str()), Some("webm"));
}

#[cfg(feature = "video")]
#[test]
fn cached_path_defaults_extension_when_missing() {
    let p = cached_video_path("https://example.com/streamendpoint");
    assert_eq!(p.extension().and_then(|e| e.to_str()), Some("mp4"));
}

#[cfg(not(feature = "video"))]
#[tokio::test]
async fn run_without_video_feature_errors() {
    use crate::input::Input;
    use crate::value::Value;
    let mut inputs = vec![Input::new("url".to_string(), Value::Text("http://example.com/x.mp4".to_string()), None, None)];
    let err = OpVideoFromUrl::run(&mut inputs).await.unwrap_err();
    assert!(err.node_error.unwrap().contains("Video support is not enabled"));
}
