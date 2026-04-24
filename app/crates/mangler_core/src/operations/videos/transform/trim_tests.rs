use super::*;
use crate::input::Input;
use crate::value::{Value, VideoCodec, VideoContainer, VideoMeta, VideoRef, VideoTransformOp};
use std::path::PathBuf;

fn source_video(fps: f32, total_frames: u32) -> VideoRef {
    let meta = VideoMeta {
        width: 100,
        height: 100,
        fps,
        duration_seconds: total_frames as f64 / fps as f64,
        total_frames,
        container: VideoContainer::Mp4,
        codec: VideoCodec::H264,
    };
    VideoRef {
        path: PathBuf::from("fake.mp4"),
        meta,
        source_meta: meta,
        transforms: Vec::new(),
    }
}

fn make_inputs(video: VideoRef, start: f32, end: f32) -> Vec<Input> {
    let mut inputs = OpVideoTrim::create_inputs();
    inputs[0].value = Value::Video(video);
    inputs[1].value = Value::Decimal(start);
    inputs[2].value = Value::Decimal(end);
    inputs
}

#[tokio::test]
async fn trim_shrinks_effective_duration_and_total_frames() {
    // 10s clip at 30fps → trim to [2s, 7s] → 5s / 150 effective frames.
    let video = source_video(30.0, 300);
    let mut inputs = make_inputs(video, 2.0, 7.0);
    let resp = OpVideoTrim::run(&mut inputs).await.expect("run ok");
    let Value::Video(out) = &resp.responses[0].value else {
        panic!("expected Video output")
    };
    assert!((out.meta.duration_seconds - 5.0).abs() < 1e-6);
    assert_eq!(out.meta.total_frames, 150);
    assert_eq!(out.source_meta.total_frames, 300, "source_meta is preserved");
    assert_eq!(out.transforms.len(), 1);
}

#[tokio::test]
async fn trim_maps_effective_time_to_source_time() {
    let video = source_video(30.0, 300);
    let mut inputs = make_inputs(video, 2.0, 7.0);
    let resp = OpVideoTrim::run(&mut inputs).await.expect("run ok");
    let Value::Video(out) = &resp.responses[0].value else { panic!() };

    // Effective second 0 → source second 2 → frame 60.
    assert_eq!(out.source_frame_for_effective_time(0.0), 60);
    // Effective second 3 → source second 5 → frame 150.
    assert_eq!(out.source_frame_for_effective_time(3.0), 150);
}

#[tokio::test]
async fn trim_chains_with_existing_transforms() {
    // Trim twice: first [2, 8] (6s), then [1, 4] of that (3s).
    // Second trim's 0s should map to source second 3.
    let video = source_video(30.0, 300);
    let mut inputs = make_inputs(video, 2.0, 8.0);
    let resp = OpVideoTrim::run(&mut inputs).await.expect("run 1");
    let Value::Video(once) = resp.responses[0].value.clone() else { panic!() };

    let mut inputs2 = OpVideoTrim::create_inputs();
    inputs2[0].value = Value::Video(once);
    inputs2[1].value = Value::Decimal(1.0);
    inputs2[2].value = Value::Decimal(4.0);
    let resp2 = OpVideoTrim::run(&mut inputs2).await.expect("run 2");
    let Value::Video(twice) = &resp2.responses[0].value else { panic!() };

    assert!((twice.meta.duration_seconds - 3.0).abs() < 1e-6);
    assert_eq!(twice.transforms.len(), 2);
    // Effective 0 → inner 1 → outer (1 + 2) = 3s → frame 90.
    assert_eq!(twice.source_frame_for_effective_time(0.0), 90);
    assert!(matches!(twice.transforms[0], VideoTransformOp::Trim { .. }));
    assert!(matches!(twice.transforms[1], VideoTransformOp::Trim { .. }));
}
