//! The handful of Phosphor icon glyphs the UI uses, vendored.
//!
//! This replaces the `egui-phosphor` crate. That crate is pinned to a single
//! egui minor version (0.13 → egui 0.35) and had no egui 0.36 release, which
//! made it the only thing holding the whole app back a version. Depending on it
//! anyway would put *two* egui versions in the lockfile.
//!
//! Vendoring costs almost nothing: the crate's only egui touchpoint is the
//! eight-line `add_to_fonts` reproduced below, and the icons themselves are
//! plain `&str` codepoints. The font file it embeds (`res/Phosphor.ttf`) was
//! already compiled into the binary, so this is not new weight — see
//! `res/LICENSE-Phosphor.txt` for attribution.
//!
//! To add an icon, look its codepoint up in the Phosphor web font
//! (<https://phosphoricons.com>) and add a `const` here.

use eframe::egui;

/// Installs the Phosphor glyphs as a fallback family behind the proportional
/// font, so an icon `&str` renders inline in ordinary label text.
///
/// Inserted at index 1 — *behind* the UI font but ahead of egui's own
/// fallbacks — so real text keeps using our font and only the icon codepoints
/// (which no text font defines) fall through to Phosphor.
pub fn add_to_fonts(fonts: &mut egui::FontDefinitions) {
    fonts.font_data.insert(
        "phosphor".into(),
        egui::FontData::from_static(include_bytes!("../res/Phosphor.ttf")).into(),
    );

    if let Some(font_keys) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        font_keys.insert(1, "phosphor".into());
    }
}

pub const BOOKS: &str = "\u{E758}";
pub const CARET_DOWN: &str = "\u{E136}";
pub const CARET_RIGHT: &str = "\u{E13A}";
pub const CUBE: &str = "\u{E1DA}";
pub const DOTS_SIX_VERTICAL: &str = "\u{EAE2}";
pub const FILE: &str = "\u{E230}";
pub const FOLDER: &str = "\u{E24A}";
pub const GRAPH: &str = "\u{EB58}";
pub const IMAGE: &str = "\u{E2CA}";
pub const LIST: &str = "\u{E2F0}";
pub const PLUS: &str = "\u{E3D4}";
pub const SLIDERS: &str = "\u{E432}";
pub const SQUARES_FOUR: &str = "\u{E464}";
pub const WARNING: &str = "\u{E4E0}";
pub const X: &str = "\u{E4F6}";

// Used to dress the file dialog, which ships emoji defaults that render as
// mismatched glyphs against the rest of the UI.
pub const ARROW_LEFT: &str = "\u{E058}";
pub const ARROW_RIGHT: &str = "\u{E06C}";
pub const CARET_UP: &str = "\u{E13C}";
pub const DOTS_THREE_VERTICAL: &str = "\u{E208}";
pub const FILE_TEXT: &str = "\u{E23A}";
pub const FLOPPY_DISK: &str = "\u{E248}";
pub const FOLDER_PLUS: &str = "\u{E258}";
pub const HARD_DRIVE: &str = "\u{E29E}";
pub const MAGNIFYING_GLASS: &str = "\u{E30C}";
pub const PENCIL_SIMPLE: &str = "\u{E3B4}";
pub const PUSH_PIN: &str = "\u{E3E2}";
pub const HOUSE: &str = "\u{E2C2}";
pub const DESKTOP: &str = "\u{E560}";
pub const DOWNLOAD_SIMPLE: &str = "\u{E20C}";
pub const MUSIC_NOTE: &str = "\u{E33C}";
pub const FILM_STRIP: &str = "\u{E792}";

#[cfg(test)]
#[path = "icons_tests.rs"]
mod tests;
