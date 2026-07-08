use super::theme::ThemeValues;
use epaint::{Color32, CornerRadius, Hsva, Shadow, Stroke};

pub fn theme_dark_green() -> ThemeValues {
    // Reusable palette colors — originally rgb(R,G,B), converted to linear-space Hsva.
    let dark_teal: Color32 = Hsva::new(
        0.5270115912356882,
        0.4145241382374324,
        0.03954623527673284,
        1.0,
    )
    .into(); // rgb(42, 54, 56)
    let darkest_teal: Color32 = Hsva::new(
        0.5207195759723184,
        0.36609949715546325,
        0.019382360956935723,
        1.0,
    )
    .into(); // rgb(29, 37, 38)
    let light_teal_text: Color32 = Hsva::new(
        0.5206857447508191,
        0.25316729618415346,
        0.665387298282272,
        1.0,
    )
    .into(); // rgb(187, 210, 213)
    let rose_accent: Color32 = Hsva::new(
        0.9896704737652213,
        0.9308965048608298,
        0.4969329950608704,
        1.0,
    )
    .into(); // rgb(187, 52, 71)
    let dark_teal_bg: Color32 = Hsva::new(
        0.5187434955149994,
        0.3761665804884992,
        0.02315336617811041,
        1.0,
    )
    .into(); // rgb(32, 41, 42)
    let teal_cyan: Color32 = Hsva::new(
        0.5416967342144122,
        0.9736545952238085,
        0.3467040563550296,
        1.0,
    )
    .into(); // rgb(24, 140, 159)
    let teal_interactive: Color32 = Hsva::new(0.522, 0.85, 0.32, 1.0).into();
    // Resting fill for "strong" controls (checkbox boxes, slider troughs). Kept
    // noticeably lighter than both `window_fill` (the menu-popup background) and
    // `panel_fill`, so these controls stay legible against either — previously
    // they used `darkest_teal`, which matched the menu background and made the
    // checkbox outline / slider bar disappear. Buttons use the *weak* fill and
    // are deliberately left on `darkest_teal`.
    let control_surface: Color32 = Hsva::new(0.522, 0.30, 0.075, 1.0).into();

    ThemeValues {
        panel_fill: dark_teal,
        extreme_bg_color: darkest_teal,
        override_text_color: light_teal_text,
        menu_bar: darkest_teal,
        menu_bar_button: darkest_teal,
        menu_bar_button_selected: rose_accent,

        grid_bg: dark_teal_bg,
        grid_lines: dark_teal,
        grid_connection_line: teal_cyan,
        grid_connection_line_width: 1.0,
        grid_connection_dot: teal_cyan,
        grid_connection_dot_hover: Hsva::new(0.522, 0.85, 0.62, 1.0).into(),
        grid_connection_dot_disabled: Hsva::new(0.522, 0.1, 0.2, 1.0).into(),
        grid_connection_dot_error: rose_accent,

        node_header_dirty_bg: Hsva::new(0.08, 0.7, 0.55, 1.0).into(),
        node_header_bg: Hsva::new(
            0.5126944764641147,
            0.40537363922409836,
            0.04970656598412723,
            1.0,
        )
        .into(), // rgb(48, 62, 63)
        node_header_selected_border: rose_accent,

        text_faint: Hsva::new(
            0.5243873894790635,
            0.6460693362148398,
            0.238397573812271,
            1.0,
        )
        .into(), // rgb(82, 128, 134)

        // Settings-panel section hairline. Darker than `darkest_teal` (the
        // extreme_bg_color, rgb(29,37,38)) so sections read as divided by a
        // quiet rule rather than a change of surface. Written directly with
        // `Color32::from_rgb` rather than a computed Hsva local: it's a
        // one-off value with no other reuse, so the exact-roundtrip Hsva
        // dance the rest of this file does (see the test below) isn't worth
        // the indirection here.
        settings_section_rule: Color32::from_rgb(20, 26, 27),
        // Reuse the existing teal-cyan accent (also `grid_connection_line`)
        // for inline links, so "help" toggles read as clickable without
        // introducing a color that collides with `rose_accent`, which is
        // reserved for selection/error states.
        text_link: teal_cyan,

        histogram_bg: Hsva::new(
            0.5187434955149994,
            0.3761665804884992,
            0.02315336617811041,
            1.0,
        )
        .into(),
        histogram_luminance: Hsva::new(0.52, 0.15, 0.28, 1.0).into(),
        histogram_red: Hsva::new(0.99, 0.82, 0.50, 0.55).into(),
        histogram_green: Hsva::new(0.43, 0.87, 0.50, 0.51).into(),
        histogram_blue: Hsva::new(0.60, 0.88, 0.54, 0.51).into(),

        window_corner_radius: CornerRadius::same(1),
        window_shadow: Shadow::NONE,
        window_fill: Hsva::new(0.525, 0.25, 0.018, 1.0).into(),
        window_stroke: Stroke::NONE,

        widgets_noninteractive_bg_fill: darkest_teal,
        widgets_noninteractive_weak_bg_fill: darkest_teal,
        widgets_noninteractive_bg_stroke: (0.0, Color32::WHITE),
        widgets_noninteractive_rounding: 1.0,
        widgets_noninteractive_fg_stroke: (0.0, light_teal_text),
        widgets_noninteractive_expansion: 0.0,

        widgets_interactive_bg_fill: control_surface,
        widgets_interactive_weak_bg_fill: darkest_teal,
        widgets_interactive_bg_stroke: (0.0, Color32::WHITE),
        widgets_interactive_rounding: 1.0,
        widgets_interactive_fg_stroke: (1.0, teal_interactive),
        widgets_interactive_expansion: 0.0,

        // Hover fill must stay darker than the fg-stroke checkmark
        // (`teal_interactive`) so the mark stays visible when hovering a
        // checkbox — it used to be the same `teal_interactive` value as the
        // checkmark stroke, so the mark effectively disappeared on hover.
        widgets_hovered_bg_fill: Hsva::new(0.522, 0.50, 0.11, 1.0).into(),
        // Was `dark_teal_bg` rgb(32,41,42), nearly identical to the button
        // resting fill `darkest_teal` rgb(29,37,38) — that made button hover
        // hard to notice. Reuse `control_surface`, which is clearly lighter
        // than both.
        widgets_hovered_weak_bg_fill: control_surface,
        widgets_hovered_bg_stroke: (0.0, Color32::WHITE),
        widgets_hovered_rounding: 1.0,
        widgets_hovered_fg_stroke: (1.0, teal_interactive),
        widgets_hovered_expansion: 0.0,

        // Active (pressed) fill: a touch brighter than the hover fill above,
        // both teal. Used to be `rose_accent`, which produced a jarring red
        // flash on every checkbox/slider click since this is the fill used
        // while a widget is being pressed.
        widgets_active_bg_fill: Hsva::new(0.522, 0.55, 0.16, 1.0).into(),
        widgets_active_weak_bg_fill: darkest_teal,
        widgets_active_bg_stroke: (0.0, Color32::WHITE),
        widgets_active_rounding: 1.0,
        // Checkmark/slider-handle stroke while pressed: brighter teal instead
        // of `rose_accent`, which made the checkmark flash rose while held
        // down (the other half of the "weird red flash on click" report).
        widgets_active_fg_stroke: (1.0, Hsva::new(0.522, 0.85, 0.45, 1.0).into()),
        widgets_active_expansion: 0.0,

        widgets_open_bg_fill: darkest_teal,
        widgets_open_weak_bg_fill: darkest_teal,
        widgets_open_bg_stroke: (0.0, Color32::WHITE),
        widgets_open_rounding: 1.0,
        widgets_open_fg_stroke: (0.0, light_teal_text),
        widgets_open_expansion: 0.0,

        dark_mode: true,
    }
}

#[cfg(test)]
mod tests {
    use epaint::{Color32, Hsva};

    /// Verify that our linear-space Hsva values produce the exact same Color32 bytes
    /// as the original Color32::from_rgb values. egui's Hsva uses linear light for V,
    /// so the HSV values were computed by converting sRGB→linear before the HSV transform.
    #[test]
    fn test_hsva_roundtrip_exact() {
        let cases: Vec<((u8, u8, u8), Hsva)> = vec![
            (
                (42, 54, 56),
                Hsva::new(
                    0.5270115912356882,
                    0.4145241382374324,
                    0.03954623527673284,
                    1.0,
                ),
            ),
            (
                (29, 37, 38),
                Hsva::new(
                    0.5207195759723184,
                    0.36609949715546325,
                    0.019382360956935723,
                    1.0,
                ),
            ),
            (
                (187, 210, 213),
                Hsva::new(
                    0.5206857447508191,
                    0.25316729618415346,
                    0.665387298282272,
                    1.0,
                ),
            ),
            (
                (187, 52, 71),
                Hsva::new(
                    0.9896704737652213,
                    0.9308965048608298,
                    0.4969329950608704,
                    1.0,
                ),
            ),
            (
                (32, 41, 42),
                Hsva::new(
                    0.5187434955149994,
                    0.3761665804884992,
                    0.02315336617811041,
                    1.0,
                ),
            ),
            (
                (24, 140, 159),
                Hsva::new(
                    0.5416967342144122,
                    0.9736545952238085,
                    0.3467040563550296,
                    1.0,
                ),
            ),
            (
                (48, 62, 63),
                Hsva::new(
                    0.5126944764641147,
                    0.40537363922409836,
                    0.04970656598412723,
                    1.0,
                ),
            ),
            (
                (82, 128, 134),
                Hsva::new(
                    0.5243873894790635,
                    0.6460693362148398,
                    0.238397573812271,
                    1.0,
                ),
            ),
        ];

        for ((r, g, b), hsva) in cases {
            let expected = Color32::from_rgb(r, g, b);
            let actual: Color32 = hsva.into();
            assert_eq!(
                expected, actual,
                "Color mismatch for ({},{},{}): expected {:?}, got {:?}",
                r, g, b, expected, actual
            );
        }
    }
}
