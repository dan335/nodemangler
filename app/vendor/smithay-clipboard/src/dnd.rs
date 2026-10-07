//! Receiving file drops over Wayland drag-and-drop (NodeMangler patch).
//!
//! winit 0.30 implements file drops on X11, Windows and macOS but not on
//! Wayland, so eframe apps never see `dropped_files` there. A client can't
//! simply bind a second `wl_data_device` to fill the gap: compositors are not
//! obliged to deliver drag events to every device a client owns, and Hyprland
//! sends them (and the clipboard selection) only to the client's *first*
//! device — which is this crate's, created by egui-winit before any app code
//! runs. So drops have to be accepted by the device that already exists.
//!
//! The handler receives the raw `text/uri-list` payload; turning it into
//! paths is the application's business. Until a handler is installed nothing
//! is accepted, so the crate behaves exactly like upstream.

use std::sync::Mutex;

type DropHandler = Box<dyn Fn(String) + Send + Sync>;

static DROP_HANDLER: Mutex<Option<DropHandler>> = Mutex::new(None);

/// The only drag payload we accept: what file managers offer for files.
pub(crate) const URI_LIST: &str = "text/uri-list";

/// Install the callback invoked (on the clipboard thread) with the
/// `text/uri-list` contents of every drop onto any of the client's surfaces.
///
/// Process-wide: every `Clipboard` shares it, which matches the compositor's
/// view — drops go to the client, not to a particular window's clipboard.
pub fn set_drop_handler(handler: impl Fn(String) + Send + Sync + 'static) {
    *DROP_HANDLER.lock().unwrap_or_else(|e| e.into_inner()) = Some(Box::new(handler));
}

pub(crate) fn has_handler() -> bool {
    DROP_HANDLER.lock().unwrap_or_else(|e| e.into_inner()).is_some()
}

pub(crate) fn deliver(uri_list: String) {
    if let Some(handler) = DROP_HANDLER.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        handler(uri_list);
    }
}
