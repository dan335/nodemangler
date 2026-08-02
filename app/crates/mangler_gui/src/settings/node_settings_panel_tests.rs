use super::{
    batch_button_enabled, watch_button_enabled, watch_last_line, watch_status_line, WatchStatusView,
};

fn status(captured: usize, pending: usize, skipped: usize) -> WatchStatusView {
    WatchStatusView {
        captured,
        pending,
        skipped,
        ..Default::default()
    }
}

// === watch_status_line ===

#[test]
fn status_line_omits_skipped_when_zero() {
    assert_eq!(watch_status_line(&status(12, 2, 0)), "12 captured · 2 pending");
}

#[test]
fn status_line_appends_skipped_when_non_zero() {
    assert_eq!(
        watch_status_line(&status(12, 2, 3)),
        "12 captured · 2 pending · 3 skipped"
    );
}

#[test]
fn status_line_at_session_start() {
    // The snapshot sent on start, before any photo has landed.
    assert_eq!(watch_status_line(&status(0, 0, 0)), "0 captured · 0 pending");
}

// === watch_last_line ===

#[test]
fn last_line_shows_the_stem() {
    assert_eq!(watch_last_line(Some("IMG_0042")), "last: IMG_0042");
}

#[test]
fn last_line_falls_back_to_waiting() {
    assert_eq!(watch_last_line(None), "waiting for the first photo…");
}

// === mutual exclusion predicates ===

#[test]
fn batch_is_startable_only_when_nothing_watches() {
    assert!(batch_button_enabled(false));
    assert!(!batch_button_enabled(true));
}

#[test]
fn watch_is_startable_only_when_no_batch_runs() {
    assert!(watch_button_enabled(false));
    assert!(!watch_button_enabled(true));
}
