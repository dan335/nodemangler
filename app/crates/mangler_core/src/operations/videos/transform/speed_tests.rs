use super::*;
use crate::input::Input;
use crate::value::{Value, VideoCodec, VideoContainer, VideoMeta, VideoRef};
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

fn make_inputs(video: VideoRef, factor: f32) -> Vec<Input> {
    let mut inputs = OpVideoSpeed::create_inputs();
    inputs[0].value = Value::Video(video);
    inputs[1].value = Value::Decimal(factor);
    inputs
}

#[tokio::test]
async fn speed_2x_halves_duration() {
    // 10s @ 30fps → 5s @ 30fps, same fps, 150 effective frames.
    let mut inputs = make_inputs(source_video(30.0, 300), 2.0);
    let resp = OpVideoSpeed::run(&mut inputs).await.expect("run");
    let Value::Video(out) = &resp.responses[0].value else { panic!() };
    assert!((out.meta.duration_seconds - 5.0).abs() < 1e-6);
    assert_eq!(out.meta.total_frames, 150);
    assert!((out.meta.fps - 30.0).abs() < 1e-6);
}

#[tokio::test]
async fn speed_2x_samples_source_at_2x_time() {
    // Effective 1s → source 2s → frame 60.
    let mut inputs = make_inputs(source_video(30.0, 300), 2.0);
    let resp = OpVideoSpeed::run(&mut inputs).await.expect("run");
    let Value::Video(out) = &resp.responses[0].value else { panic!() };
    assert_eq!(out.source_frame_for_effective_time(1.0), 60);
}

#[tokio::test]
async fn speed_half_doubles_duration() {
    let mut inputs = make_inputs(source_video(30.0, 300), 0.5);
    let resp = OpVideoSpeed::run(&mut inputs).await.expect("run");
    let Value::Video(out) = &resp.responses[0].value else { panic!() };
    assert!((out.meta.duration_seconds - 20.0).abs() < 1e-6);
    // Effective 4s → source 2s → frame 60.
    assert_eq!(out.source_frame_for_effective_time(4.0), 60);
}

#[tokio::test]
async fn speed_negative_is_clamped_not_inverted() {
    // Negative factor is clamped to a tiny positive; reverse is a separate op.
    let mut inputs = make_inputs(source_video(30.0, 300), -1.0);
    let resp = OpVideoSpeed::run(&mut inputs).await.expect("run");
    let Value::Video(out) = &resp.responses[0].value else { panic!() };
    assert!(out.meta.duration_seconds > 0.0);
}
