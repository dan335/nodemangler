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

fn make_inputs(video: VideoRef) -> Vec<Input> {
    let mut inputs = OpVideoReverse::create_inputs();
    inputs[0].value = Value::Video(video);
    inputs
}

#[tokio::test]
async fn reverse_preserves_duration_and_fps() {
    let mut inputs = make_inputs(source_video(30.0, 300));
    let resp = OpVideoReverse::run(&mut inputs).await.expect("run");
    let Value::Video(out) = &resp.responses[0].value else { panic!() };
    assert!((out.meta.duration_seconds - 10.0).abs() < 1e-6);
    assert_eq!(out.meta.total_frames, 300);
    assert!((out.meta.fps - 30.0).abs() < 1e-6);
}

#[tokio::test]
async fn reverse_flips_time_to_source() {
    // 10s clip. Effective 0s should map to source 10s → frame 299 (clamped
    // from 300 to total_frames - 1).
    let mut inputs = make_inputs(source_video(30.0, 300));
    let resp = OpVideoReverse::run(&mut inputs).await.expect("run");
    let Value::Video(out) = &resp.responses[0].value else { panic!() };

    assert_eq!(out.source_frame_for_effective_time(0.0), 299);
    // Effective 10s should map to source 0s → frame 0.
    assert_eq!(out.source_frame_for_effective_time(10.0), 0);
    // Effective 4s → source 6s → frame 180.
    assert_eq!(out.source_frame_for_effective_time(4.0), 180);
}
