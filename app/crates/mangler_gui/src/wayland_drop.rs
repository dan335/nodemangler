//! File drops from the desktop on Wayland.
//!
//! winit 0.30 only reports `DroppedFile` on X11, Windows and macOS, so under a
//! Wayland compositor (Hyprland/Omarchy, GNOME, KDE…) dragging an image in
//! from a file manager did nothing. The drop is received by our patched
//! smithay-clipboard (`app/vendor/smithay-clipboard`, which explains why it has
//! to live there) and handed over here as raw `text/uri-list` text, which
//! [`WaylandDrops::inject`] turns into ordinary `RawInput::dropped_files` — so
//! everything downstream (`Program::update`'s drop handling) stays
//! platform-neutral. On X11 the handler is installed but never fires.

use eframe::egui;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

pub struct WaylandDrops {
    rx: Receiver<String>,
}

impl WaylandDrops {
    /// Start accepting drops. The handler runs on the clipboard thread, so it
    /// only forwards the payload and wakes the UI.
    pub fn install(ctx: &egui::Context) -> Self {
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        smithay_clipboard::set_drop_handler(move |uri_list| {
            if tx.send(uri_list).is_ok() {
                ctx.request_repaint();
            }
        });
        Self { rx }
    }

    /// Append every drop received since the last frame to `raw.dropped_files`.
    pub fn inject(&self, raw: &mut egui::RawInput) {
        for uri_list in self.rx.try_iter() {
            raw.dropped_files.extend(
                parse_uri_list(&uri_list)
                    .into_iter()
                    .map(|path| Arc::new(DroppedPath(path)) as egui::DroppedFileHandle),
            );
        }
    }
}

/// egui 0.36's `DroppedFile` is a trait, and egui-winit's implementation of
/// it (`NativeFile`) is private; this is the same thing.
#[derive(Debug)]
struct DroppedPath(PathBuf);

impl egui::DroppedFile for DroppedPath {
    fn path(&self) -> &Path {
        &self.0
    }

    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|err| err.to_string())
    }
}

/// The local paths in a `text/uri-list` payload (RFC 2483): one URI per line,
/// CRLF-separated, `#` lines are comments. Non-`file:` URIs (a link dragged
/// from a browser) and remote `file://host/…` URIs are skipped; paths are
/// percent-decoded to raw bytes, since a Linux filename need not be UTF-8.
pub fn parse_uri_list(text: &str) -> Vec<PathBuf> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(file_uri_to_path)
        .collect()
}

fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file:")?;
    // `file:///p` (empty authority), `file://localhost/p`, or the bare
    // `file:/p` some tools emit.
    let path = match rest.strip_prefix("//") {
        Some(authority_and_path) => {
            let slash = authority_and_path.find('/')?;
            let host = &authority_and_path[..slash];
            if !host.is_empty() && !host.eq_ignore_ascii_case("localhost") {
                return None;
            }
            &authority_and_path[slash..]
        }
        None => rest,
    };
    if !path.starts_with('/') {
        return None;
    }
    Some(PathBuf::from(OsString::from_vec(percent_decode(path))))
}

/// `%XX` → byte; a malformed escape is kept literally rather than rejected.
fn percent_decode(s: &str) -> Vec<u8> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3).and_then(|h| std::str::from_utf8(h).ok());
            if let Some(byte) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

#[cfg(test)]
#[path = "wayland_drop_tests.rs"]
mod tests;
