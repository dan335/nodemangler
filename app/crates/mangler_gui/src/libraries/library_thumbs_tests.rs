//! Pure-policy tests for the library thumbnail cache queue / eviction rules.
//! No filesystem, no worker threads — same shape as `ingest_listing` tests.

use std::collections::VecDeque;
use std::path::PathBuf;

use eframe::egui;

use super::{
    enqueue_job, is_stale, next_job, prefetch_band, should_evict_lru, LIBRARY_THUMB_UI,
};

#[test]
fn next_job_is_fifo_from_front() {
    let mut jobs = VecDeque::new();
    enqueue_job(&mut jobs, PathBuf::from("a.jpg"), true);
    enqueue_job(&mut jobs, PathBuf::from("b.jpg"), true);
    // High-priority newest goes front → pops first.
    assert_eq!(next_job(&mut jobs), Some(PathBuf::from("b.jpg")));
    assert_eq!(next_job(&mut jobs), Some(PathBuf::from("a.jpg")));
    assert_eq!(next_job(&mut jobs), None);
}

#[test]
fn visible_jobs_beat_prefetch() {
    let mut jobs = VecDeque::new();
    // Prefetch a band of upcoming files first…
    enqueue_job(&mut jobs, PathBuf::from("below1.jpg"), false);
    enqueue_job(&mut jobs, PathBuf::from("below2.jpg"), false);
    // …then the user scrolls a cell into view (high priority).
    enqueue_job(&mut jobs, PathBuf::from("visible.jpg"), true);
    assert_eq!(next_job(&mut jobs), Some(PathBuf::from("visible.jpg")));
    // Prefetch continues in FIFO order after high-priority work.
    assert_eq!(next_job(&mut jobs), Some(PathBuf::from("below1.jpg")));
    assert_eq!(next_job(&mut jobs), Some(PathBuf::from("below2.jpg")));
}

#[test]
fn should_evict_lru_only_over_cap() {
    assert!(!should_evict_lru(0, 512));
    assert!(!should_evict_lru(512, 512));
    assert!(should_evict_lru(513, 512));
}

#[test]
fn is_stale_uses_saturating_age() {
    assert!(!is_stale(100, 100, 120));
    assert!(!is_stale(100, 220, 120)); // exactly age: not yet stale
    assert!(is_stale(100, 221, 120));
    // frame wrap / last_seen in the future shouldn't panic
    assert!(!is_stale(50, 10, 120));
}

#[test]
fn prefetch_band_expands_vertically() {
    let clip = egui::Rect::from_min_max(egui::pos2(0.0, 100.0), egui::pos2(200.0, 300.0));
    // height = 200; 1.5 viewports → 300, floor is LIBRARY_THUMB_UI*4 = 320 → 320 wins
    let expected_margin = (200.0_f32 * 1.5).max(LIBRARY_THUMB_UI * 4.0);
    let band = prefetch_band(clip, 1.5);
    assert_eq!(band.min.y, 100.0 - expected_margin);
    assert_eq!(band.max.y, 300.0 + expected_margin);
    assert_eq!(band.min.x, clip.min.x);
    assert_eq!(band.max.x, clip.max.x);
}

#[test]
fn prefetch_band_has_floor_margin() {
    // Tiny clip height would otherwise prefetch almost nothing.
    let clip = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 10.0));
    let band = prefetch_band(clip, 1.5);
    let min_margin = LIBRARY_THUMB_UI * 4.0;
    assert!((band.max.y - clip.max.y) >= min_margin - 0.1);
}
