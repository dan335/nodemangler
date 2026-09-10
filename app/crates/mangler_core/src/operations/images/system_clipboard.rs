//! Serialized access to the OS clipboard.
//!
//! Both clipboard nodes reach the platform clipboard through `arboard`, and
//! ops run on the multi-threaded tokio runtime — so two clipboard nodes in one
//! graph can hit it from two threads at once. The platform APIs are not
//! prepared for that: on macOS `NSPasteboard` mutates an internal type cache
//! while enumerating it and the process traps (`EXC_BREAKPOINT` inside
//! `-[NSPasteboard _updateTypeCacheIfNeeded]`), which is also what made the
//! test suite crash intermittently when the two clipboard tests happened to
//! land on different threads.
//!
//! Every `Clipboard` handle is therefore opened and used inside [`with_clipboard`],
//! which holds a process-wide lock for the whole borrow. The clipboard is a
//! single shared OS resource, so serializing it costs nothing worth having.

use arboard::Clipboard;
use std::sync::{Mutex, MutexGuard, OnceLock};

fn lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    // A poisoned lock only means some other clipboard op panicked; the OS
    // clipboard itself is not left in a broken state, so carry on with it.
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|e| e.into_inner())
}

/// Runs `f` with an open clipboard handle, exclusively for its duration.
///
/// Returns `None` when the platform clipboard cannot be opened at all (no
/// display server, no pasteboard), which callers already treat as "nothing
/// there" / "could not copy".
pub fn with_clipboard<T>(f: impl FnOnce(&mut Clipboard) -> T) -> Option<T> {
    let _guard = lock();
    let mut clipboard = Clipboard::new().ok()?;
    Some(f(&mut clipboard))
}

#[cfg(test)]
#[path = "system_clipboard_tests.rs"]
mod tests;
