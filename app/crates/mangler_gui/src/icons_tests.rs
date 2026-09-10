use super::*;
use ab_glyph::{Font, FontRef};

/// Every constant in this module must map to a real glyph in the vendored
/// font. A codepoint that isn't in the font renders as tofu (or nothing) —
/// there is no error, no warning, and no test elsewhere would notice, so a
/// typo'd `\u{...}` on a newly added icon would ship silently.
#[test]
fn every_icon_constant_resolves_to_a_glyph() {
    let font = FontRef::try_from_slice(include_bytes!("../res/Phosphor.ttf"))
        .expect("the vendored Phosphor.ttf should parse");

    // Kept in step with the constants below by hand; the list is short and
    // adding an icon without adding it here defeats the point of the test.
    let icons: &[(&str, &str)] = &[
        ("BOOKS", BOOKS),
        ("CARET_DOWN", CARET_DOWN),
        ("CARET_RIGHT", CARET_RIGHT),
        ("CUBE", CUBE),
        ("DOTS_SIX_VERTICAL", DOTS_SIX_VERTICAL),
        ("FILE", FILE),
        ("FOLDER", FOLDER),
        ("GRAPH", GRAPH),
        ("IMAGE", IMAGE),
        ("LIST", LIST),
        ("PLUS", PLUS),
        ("SLIDERS", SLIDERS),
        ("SQUARES_FOUR", SQUARES_FOUR),
        ("WARNING", WARNING),
        ("X", X),
    ];

    for (name, glyph) in icons {
        let mut chars = glyph.chars();
        let c = chars.next().unwrap_or_else(|| panic!("{name} is empty"));
        assert!(
            chars.next().is_none(),
            "{name} should be a single codepoint, not {glyph:?}"
        );
        // `glyph_id` returns 0 (the .notdef / tofu glyph) for anything the
        // font's cmap doesn't cover.
        assert_ne!(
            font.glyph_id(c).0,
            0,
            "{name} (U+{:04X}) is not in res/Phosphor.ttf — it would render as tofu",
            c as u32
        );
    }
}
