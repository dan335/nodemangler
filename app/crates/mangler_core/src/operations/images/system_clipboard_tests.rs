//! Tests for the serialized clipboard access point.
//!
//! What matters here is the *serialization*, not the clipboard's contents —
//! whether this machine has a usable pasteboard, and what is on it, is not
//! something a test can control.

use super::*;
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::sync::Arc;

#[test]
fn concurrent_access_never_overlaps() {
    // The bug this module exists for: two threads inside the platform
    // clipboard at once. If the lock is dropped or scoped too narrowly, the
    // overlap counter goes above one here — and on macOS the real
    // `NSPasteboard` traps rather than merely racing.
    let inside = Arc::new(AtomicUsize::new(0));
    let overlapped = Arc::new(AtomicBool::new(false));

    let threads: Vec<_> = (0..8)
        .map(|_| {
            let (inside, overlapped) = (Arc::clone(&inside), Arc::clone(&overlapped));
            std::thread::spawn(move || {
                for _ in 0..25 {
                    with_clipboard(|_| {
                        if inside.fetch_add(1, Ordering::SeqCst) != 0 {
                            overlapped.store(true, Ordering::SeqCst);
                        }
                        std::thread::yield_now();
                        inside.fetch_sub(1, Ordering::SeqCst);
                    });
                }
            })
        })
        .collect();
    for t in threads {
        t.join().expect("a clipboard worker panicked");
    }

    assert!(!overlapped.load(Ordering::SeqCst), "two threads were in the clipboard at once");
}

#[test]
fn a_panicking_caller_does_not_wedge_the_lock() {
    // A panic inside the closure poisons the mutex. The OS clipboard is fine,
    // so later callers must still get in rather than inheriting the poison.
    let _ = std::panic::catch_unwind(|| {
        with_clipboard(|_| panic!("boom"));
    });

    let reached = with_clipboard(|_| ()).is_some();
    // `None` only means this machine has no usable clipboard (headless CI);
    // either way the call must return instead of panicking on the poison.
    let _ = reached;
}

#[test]
fn the_handle_is_usable_inside_the_closure() {
    // Exercises the real open-then-use path. A machine without a pasteboard
    // yields `None`, which is the documented "nothing there" answer.
    if let Some(result) = with_clipboard(|clipboard| clipboard.get_text()) {
        // Whatever the outcome, reading must not have panicked.
        let _ = result.is_ok();
    }
}
