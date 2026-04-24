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

fn make_inputs(video: VideoRef, count: i32) -> Vec<Input> {
    let mut inputs = OpVideoLoop::create_inputs();
    inputs[0].value = Value::Video(video);
    inputs[1].value = Value::Integer(count);
    inputs
}

#[tokio::test]
async fn loop_multiplies_duration_and_total_frames() {
    let mut inputs = make_inputs(source_video(30.0, 300), 3);
    let resp = OpVideoLoop::run(&mut inputs).await.expect("run");
    let Value::Video(out) = &resp.responses[0].value else { panic!() };
    assert!((out.meta.duration_seconds - 30.0).abs() < 1e-6);
    assert_eq!(out.meta.total_frames, 900);
}

#[tokio::test]
async fn loop_wraps_effective_time_modulo_source() {
    // 10s clip looped 3x. Effective 15s → source 5s → frame 150.
    let mut inputs = make_inputs(source_video(30.0, 300), 3);
    let resp = OpVideoLoop::run(&mut inputs).await.expect("run");
    let Value::Video(out) = &resp.responses[0].value else { panic!() };
    assert_eq!(out.source_frame_for_effective_time(15.0), 150);
    assert_eq!(out.source_frame_for_effective_time(25.0), 150);
    assert_eq!(out.source_frame_for_effective_time(5.0), 150);
}

#[tokio::test]
async fn loop_count_below_one_is_treated_as_identity() {
    let mut inputs = make_inputs(source_video(30.0, 300), 0);
    let resp = OpVideoLoop::run(&mut inputs).await.expect("run");
    let Value::Video(out) = &resp.responses[0].value else { panic!() };
    // count=0 clamps to 1 → unchanged.
    assert!((out.meta.duration_seconds - 10.0).abs() < 1e-6);
}
