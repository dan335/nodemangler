use super::*;
use crate::input::Input;
use crate::value::{Value, VideoCodec, VideoContainer, VideoMeta, VideoRef};
use std::path::PathBuf;

fn video_ref_with(fps: f32, total_frames: u32) -> VideoRef {
    let meta = VideoMeta {
        width: 100,
        height: 100,
        fps,
        duration_seconds: if fps > 0.0 {
            total_frames as f64 / fps as f64
        } else {
            0.0
        },
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
    let mut inputs = OpExtractFrameByIndex::create_inputs();
    inputs[0].value = Value::Video(video);
    inputs
}

#[test]
fn test_apply_render_time_computes_frame_from_fps() {
    let video = video_ref_with(30.0, 300);
    let mut inputs = make_inputs(video);

    // t = 2.0 seconds at 30fps → frame 60.
    OpExtractFrameByIndex::apply_render_time(&mut inputs, 2.0);
    match &inputs[1].value {
        Value::Integer(n) => assert_eq!(*n, 60),
        other => panic!("expected Integer, got {:?}", other),
    }
}

#[test]
fn test_apply_render_time_clamps_to_total_frames() {
    let video = video_ref_with(30.0, 100);
    let mut inputs = make_inputs(video);

    // t = 10s at 30fps would give frame 300; clamp to 99 (total_frames - 1).
    OpExtractFrameByIndex::apply_render_time(&mut inputs, 10.0);
    match &inputs[1].value {
        Value::Integer(n) => assert_eq!(*n, 99),
        other => panic!("expected Integer, got {:?}", other),
    }
}

#[test]
fn test_apply_render_time_skips_when_path_empty() {
    // Default VideoRef has an empty path; the hook must not compute or write.
    let mut inputs = make_inputs(VideoRef::default());
    inputs[1].value = Value::Integer(42);

    OpExtractFrameByIndex::apply_render_time(&mut inputs, 5.0);
    match &inputs[1].value {
        // Untouched from what we set above.
        Value::Integer(42) => {}
        other => panic!("expected Integer(42), got {:?}", other),
    }
}

#[test]
fn test_apply_render_time_skips_when_fps_zero() {
    // Clips with unknown fps should be skipped rather than dividing-by-zero
    // or writing a nonsense index.
    let video = video_ref_with(0.0, 100);
    let mut inputs = make_inputs(video);
    inputs[1].value = Value::Integer(7);

    OpExtractFrameByIndex::apply_render_time(&mut inputs, 1.0);
    match &inputs[1].value {
        Value::Integer(7) => {}
        other => panic!("expected Integer(7), got {:?}", other),
    }
}
