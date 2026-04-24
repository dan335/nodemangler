use super::*;
use crate::float_image::FloatImage;
use crate::value::{VideoCodec, VideoContainer};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

/// Returns a unique temp path per test invocation so parallel tests never
/// collide on the output file.
fn unique_tmp_path(prefix: &str, ext: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!(
        "mangler_encoder_{}_{}_{}.{}",
        prefix, pid, n, ext
    ))
}

// The matrix-rejection check is upfront of any FFmpeg call, so we can exercise
// it with a nonsense path — it fails before touching disk.

#[test]
fn test_encoder_rejects_invalid_combo_h264_in_webm() {
    let tmp = PathBuf::from("./__encoder_test_invalid.webm");
    let err = VideoEncoder::open(
        &tmp, 64, 64, 30.0,
        VideoContainer::WebM, VideoCodec::H264,
    )
    .err().expect("H.264 in WebM should be rejected");
    assert!(err.0.contains("not valid in"), "unexpected error: {}", err.0);
}

#[test]
fn test_encoder_rejects_invalid_combo_vp9_in_mp4() {
    let tmp = PathBuf::from("./__encoder_test_invalid.mp4");
    let err = VideoEncoder::open(
        &tmp, 64, 64, 30.0,
        VideoContainer::Mp4, VideoCodec::Vp9,
    )
    .err().expect("VP9 in MP4 should be rejected");
    assert!(err.0.contains("not valid in"), "unexpected error: {}", err.0);
}

// ── Real FFmpeg encoder smoke tests ────────────────────────────────────
//
// These tests actually open an FFmpeg encoder and push real frames through.
// They catch regressions that the earlier "combo rejection" tests miss —
// `VideoEncoder::open` can succeed at the matrix check and still fail
// downstream when FFmpeg rejects the settings for a specific shape, pixel
// format, or codec preset.
//
// They require an ffmpeg build with the `libx264` encoder (i.e. built with
// GPL licence). vcpkg's default `ffmpeg:x64-windows` port omits it; see
// `docs/video-setup.md`. If your build doesn't have a usable H.264 encoder
// these will fail with "Invalid argument" — that's the test doing its job.

/// The common-case path: open H.264 in MP4 at a small even size, push a few
/// frames, finalize. If this test fails, real renders are broken.
#[tokio::test]
async fn test_encode_h264_mp4_tiny_happy_path() {
    let path = unique_tmp_path("h264_mp4_tiny", "mp4");
    let w: u32 = 64;
    let h: u32 = 48;
    let fps: f32 = 30.0;

    let mut encoder = VideoEncoder::open(
        &path, w, h, fps,
        VideoContainer::Mp4, VideoCodec::H264,
    )
    .unwrap_or_else(|e| panic!("encoder open failed: {}", e.0));

    // Push a handful of grey frames.
    let grey_pixel = [0.5f32, 0.5, 0.5, 1.0];
    let frame = std::sync::Arc::new(FloatImage::from_pixel(w, h, 4, &grey_pixel));
    for _ in 0..4 {
        encoder
            .push_frame(&frame)
            .await
            .unwrap_or_else(|e| panic!("push_frame failed: {}", e.0));
    }
    encoder
        .finalize()
        .await
        .unwrap_or_else(|e| panic!("finalize failed: {}", e.0));

    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    assert!(size > 0, "encoded file is empty at {}", path.display());
    let _ = std::fs::remove_file(&path);
}

/// H.264 with YUV420P requires even dimensions. `VideoEncoder::open` is
/// supposed to succeed at even sizes across a wide range — verify the common
/// source shapes (SD, HD, FHD, 4K-ish) all open cleanly.
#[tokio::test]
async fn test_encode_h264_mp4_common_shapes_open_cleanly() {
    let shapes: &[(u32, u32)] = &[
        (64, 48),
        (320, 240),
        (640, 480),
        (1280, 720),
        (1920, 1080),
        (3840, 2160),
    ];

    for &(w, h) in shapes {
        let path = unique_tmp_path(&format!("h264_{}x{}", w, h), "mp4");
        let result = VideoEncoder::open(
            &path, w, h, 30.0,
            VideoContainer::Mp4, VideoCodec::H264,
        );
        match result {
            Ok(_enc) => {
                // Drop the encoder without pushing any frames — we're only
                // testing open. Remove the empty file.
                drop(_enc);
                let _ = std::fs::remove_file(&path);
            }
            Err(e) => panic!(
                "opening h264/mp4 encoder at {}x{} failed: {}",
                w, h, e.0
            ),
        }
    }
}

/// Odd dimensions are an x264 YUV420P hard-fail. Callers are expected to
/// round-down to even; the `& !1` dance in render.rs does this. To make this
/// test *specifically* assert odd-dim rejection (rather than pass trivially
/// when the whole encoder is broken), we first confirm even dims succeed on
/// this build — skipping with a warning if they don't.
#[tokio::test]
async fn test_encode_h264_mp4_rejects_odd_dimensions() {
    // Baseline: if even dims already fail, this machine has no H.264 encoder
    // at all (see the module-level comment). Skip rather than pass for the
    // wrong reason.
    let baseline_path = unique_tmp_path("h264_odd_baseline", "mp4");
    let baseline = VideoEncoder::open(
        &baseline_path, 320, 240, 30.0,
        VideoContainer::Mp4, VideoCodec::H264,
    );
    let _ = std::fs::remove_file(&baseline_path);
    if baseline.is_err() {
        eprintln!(
            "skipping test_encode_h264_mp4_rejects_odd_dimensions: \
             even dimensions also fail to open, so this build has no H.264 encoder"
        );
        return;
    }

    let path = unique_tmp_path("h264_odd", "mp4");
    let result = VideoEncoder::open(
        &path, 321, 241, 30.0,
        VideoContainer::Mp4, VideoCodec::H264,
    );
    let _ = std::fs::remove_file(&path);
    assert!(
        result.is_err(),
        "h264/yuv420p should reject odd dimensions — got Ok at 321x241"
    );
}

#[test]
fn test_encoder_rejects_not_yet_implemented_combo() {
    // H.265 in MP4 is legal per the compatibility matrix but we don't wire up
    // an encoder preset for it today — should surface a distinct error.
    let tmp = PathBuf::from("./__encoder_test_nyi.mp4");
    let err = VideoEncoder::open(
        &tmp, 64, 64, 30.0,
        VideoContainer::Mp4, VideoCodec::H265,
    )
    .err().expect("encoder preset not yet wired for H.265 in MP4");
    assert!(
        err.0.contains("not yet implemented"),
        "unexpected error: {}",
        err.0,
    );
}
