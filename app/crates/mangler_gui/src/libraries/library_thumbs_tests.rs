//! Pure-policy tests for the library thumbnail cache queue / eviction rules.
//! No filesystem, no worker threads — same shape as `ingest_listing` tests.

use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;

use eframe::egui;

use super::{
    enqueue_job, evict_lru, is_stale, next_job, prefetch_band, should_evict_lru,
    LIBRARY_THUMB_UI,
};

fn paths(names: &[&str]) -> VecDeque<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

fn set(names: &[&str]) -> HashSet<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

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

#[test]
fn evict_lru_drops_the_least_recently_touched_first() {
    let mut lru = paths(&["a", "b", "c", "d"]);
    let evicted = evict_lru(&mut lru, &set(&[]), 4, 2);

    assert_eq!(evicted, vec![PathBuf::from("a"), PathBuf::from("b")]);
    assert_eq!(lru, paths(&["c", "d"]));
}

/// The regression this helper exists for: an in-flight path must survive the
/// cap. Evicting it would strand its queued job, the result would be discarded
/// as no-longer-Loading, and the next frame would re-enqueue the same decode —
/// a loop for as long as the cache stays over cap.
#[test]
fn evict_lru_never_drops_an_in_flight_decode() {
    let mut lru = paths(&["a", "b", "c", "d"]);
    let evicted = evict_lru(&mut lru, &set(&["a"]), 4, 2);

    assert_eq!(
        evicted,
        vec![PathBuf::from("b"), PathBuf::from("c")],
        "the in-flight 'a' must be skipped and the next-oldest taken instead"
    );
    assert!(lru.contains(&PathBuf::from("a")), "'a' must stay tracked");
    // Skipped entries go back at the front: still the oldest, so still the
    // first candidates once their decodes land.
    assert_eq!(lru, paths(&["a", "d"]));
}

/// With every entry in flight there is nothing legal to evict, so the pass must
/// terminate over cap rather than spin or drop live work.
#[test]
fn evict_lru_terminates_when_everything_is_in_flight() {
    let mut lru = paths(&["a", "b", "c"]);
    let evicted = evict_lru(&mut lru, &set(&["a", "b", "c"]), 3, 1);

    assert!(evicted.is_empty());
    assert_eq!(lru, paths(&["a", "b", "c"]), "order is preserved");
}

#[test]
fn evict_lru_is_a_no_op_at_or_under_cap() {
    let mut lru = paths(&["a", "b"]);
    assert!(evict_lru(&mut lru, &set(&[]), 2, 2).is_empty());
    assert_eq!(lru, paths(&["a", "b"]));
}
