use super::*;
use crate::value::{VideoCodec, VideoContainer};
use std::path::Path;

// ── classify_container ───────────────────────────────────────────────────

#[test]
fn test_classify_container_single_token_mp4() {
    assert_eq!(
        classify_container("mp4", Path::new("clip.mp4")).unwrap(),
        VideoContainer::Mp4
    );
}

#[test]
fn test_classify_container_single_token_webm() {
    assert_eq!(
        classify_container("webm", Path::new("clip.webm")).unwrap(),
        VideoContainer::WebM
    );
}

#[test]
fn test_classify_container_single_token_avi() {
    assert_eq!(
        classify_container("avi", Path::new("clip.avi")).unwrap(),
        VideoContainer::Avi
    );
}

#[test]
fn test_classify_container_matroska_token_is_mkv() {
    // ffmpeg's matroska demuxer reports "matroska,webm" — with a .mkv path we
    // should land on Mkv (extension tiebreak), not WebM.
    assert_eq!(
        classify_container("matroska,webm", Path::new("clip.mkv")).unwrap(),
        VideoContainer::Mkv
    );
}

#[test]
fn test_classify_container_matroska_webm_with_webm_path_is_webm() {
    assert_eq!(
        classify_container("matroska,webm", Path::new("clip.webm")).unwrap(),
        VideoContainer::WebM
    );
}

#[test]
fn test_classify_container_mp4_family_with_mov_path_is_mov() {
    // The ISO BMFF / QuickTime family is lumped under a single demuxer name.
    // Extension picks MOV vs MP4 within that family.
    assert_eq!(
        classify_container("mov,mp4,m4a,3gp,3g2,mj2", Path::new("clip.mov")).unwrap(),
        VideoContainer::Mov
    );
}

#[test]
fn test_classify_container_mp4_family_with_mp4_path_is_mp4() {
    assert_eq!(
        classify_container("mov,mp4,m4a,3gp,3g2,mj2", Path::new("clip.mp4")).unwrap(),
        VideoContainer::Mp4
    );
}

#[test]
fn test_classify_container_mp4_family_with_unrelated_extension_falls_back_to_first_token() {
    // Extension doesn't appear in the family list → fall through to
    // first-recognised-token ordering (mov comes first in ffmpeg's string).
    assert_eq!(
        classify_container("mov,mp4,m4a,3gp,3g2,mj2", Path::new("clip.bin")).unwrap(),
        VideoContainer::Mov
    );
}

#[test]
fn test_classify_container_unknown_token_errors() {
    let err = classify_container("theora", Path::new("clip.ogv")).unwrap_err();
    assert!(err.0.contains("Unsupported container"));
}

#[test]
fn test_classify_container_case_insensitive() {
    assert_eq!(
        classify_container("MP4", Path::new("clip.MP4")).unwrap(),
        VideoContainer::Mp4
    );
}

// ── classify_codec ───────────────────────────────────────────────────────

#[test]
fn test_classify_codec_h264() {
    assert_eq!(
        classify_codec(ffmpeg_next::codec::Id::H264).unwrap(),
        VideoCodec::H264
    );
}

#[test]
fn test_classify_codec_hevc_is_h265() {
    assert_eq!(
        classify_codec(ffmpeg_next::codec::Id::HEVC).unwrap(),
        VideoCodec::H265
    );
}

#[test]
fn test_classify_codec_vp9() {
    assert_eq!(
        classify_codec(ffmpeg_next::codec::Id::VP9).unwrap(),
        VideoCodec::Vp9
    );
}

#[test]
fn test_classify_codec_av1() {
    assert_eq!(
        classify_codec(ffmpeg_next::codec::Id::AV1).unwrap(),
        VideoCodec::Av1
    );
}

#[test]
fn test_classify_codec_prores() {
    assert_eq!(
        classify_codec(ffmpeg_next::codec::Id::PRORES).unwrap(),
        VideoCodec::ProRes
    );
}

#[test]
fn test_classify_codec_unsupported_errors() {
    let err = classify_codec(ffmpeg_next::codec::Id::THEORA).unwrap_err();
    assert!(err.0.contains("Unsupported codec"));
}
