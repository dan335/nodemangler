use epaint::{Color32, Hsva};
use super::{theme::{ThemeValues, desaturate}, theme_light_blue::theme_light_blue};

pub fn theme_light() -> ThemeValues
{
    let other = theme_light_blue();

    // Reusable palette colors
    let menu: Color32 = Hsva::new(0.565, 0.0, 0.75, 1.0).into();
    let blue_accent: Color32 = Hsva::new(0.555, 0.79, 0.97, 1.0).into();
    let gray_connection: Color32 = Hsva::new(0.555, 0.0, 0.5, 1.0).into();
    let light_teal_text: Color32 = Hsva::new(0.5206857447508191, 0.25316729618415346, 0.665387298282272, 1.0).into();   // rgb(187, 210, 213)

    ThemeValues {
        panel_fill: desaturate(other.panel_fill),
        extreme_bg_color: desaturate(other.extreme_bg_color),
        override_text_color: desaturate(other.override_text_color),
        menu_bar: menu,
        menu_bar_button: desaturate(other.menu_bar_button),
        menu_bar_button_selected: blue_accent,

        grid_bg: Hsva::new(0.565, 0.0, 0.84, 1.0).into(),
        grid_lines: Hsva::new(0.565, 0.0, 0.73, 1.0).into(),
        grid_connection_line: gray_connection,
        grid_connection_line_width: 1.0,
        grid_connection_dot: gray_connection,
        grid_connection_dot_hover: Hsva::new(0.555, 0.0, 0.6, 1.0).into(),
        grid_connection_dot_disabled: Hsva::new(0.555, 0.0, 0.75, 1.0).into(),
        grid_connection_dot_error: blue_accent,

        node_header_dirty_bg: Hsva::new(0.08, 0.6, 0.85, 1.0).into(),
        node_header_bg: Hsva::new(0.565, 0.0, 0.65, 1.0).into(),
        node_header_selected_border: blue_accent,

        text_faint: desaturate(other.text_faint),

        // Histogram: neutral light grays, slightly muted RGB
        histogram_bg: Hsva::new(0.0, 0.0, 0.882, 1.0).into(),
        histogram_luminance: Hsva::new(0.0, 0.0, 0.667, 1.0).into(),
        histogram_red: Hsva::new(0.0, 0.744, 0.765, 0.55).into(),
        histogram_green: Hsva::new(0.347, 0.750, 0.627, 0.53).into(),
        histogram_blue: Hsva::new(0.667, 0.744, 0.765, 0.55).into(),

        window_corner_radius: other.window_corner_radius,
        window_shadow: other.window_shadow,
        window_fill: Hsva::new(0.565, 0.0, 0.7, 1.0).into(),
        window_stroke: other.window_stroke,

        widgets_noninteractive_bg_fill: menu,
        widgets_noninteractive_weak_bg_fill: menu,
        widgets_noninteractive_bg_stroke: (0.0, Color32::WHITE),
        widgets_noninteractive_rounding: 1.0,
        widgets_noninteractive_fg_stroke: (0.0, light_teal_text),
        widgets_noninteractive_expansion: 3.0,

        widgets_interactive_bg_fill: menu,
        widgets_interactive_weak_bg_fill: menu,
        widgets_interactive_bg_stroke: (0.0, Color32::WHITE),
        widgets_interactive_rounding: 1.0,
        widgets_interactive_fg_stroke: (0.0, light_teal_text),
        widgets_interactive_expansion: 3.0,

        widgets_hovered_bg_fill: menu,
        widgets_hovered_weak_bg_fill: desaturate(other.widgets_hovered_weak_bg_fill),
        widgets_hovered_bg_stroke: (0.0, Color32::WHITE),
        widgets_hovered_rounding: 1.0,
        widgets_hovered_fg_stroke: (0.0, light_teal_text),
        widgets_hovered_expansion: 3.0,

        widgets_active_bg_fill: menu,
        widgets_active_weak_bg_fill: menu,
        widgets_active_bg_stroke: (0.0, Color32::WHITE),
        widgets_active_rounding: 1.0,
        widgets_active_fg_stroke: (0.0, light_teal_text),
        widgets_active_expansion: 3.0,

        widgets_open_bg_fill: menu,
        widgets_open_weak_bg_fill: menu,
        widgets_open_bg_stroke: (0.0, Color32::WHITE),
        widgets_open_rounding: 1.0,
        widgets_open_fg_stroke: (0.0, light_teal_text),
        widgets_open_expansion: 3.0,

        dark_mode: false,
    }
}

#[cfg(test)]
mod tests {
    use epaint::{Color32, Hsva};

    /// Verify that the Hsva for (187,210,213) used in fg_stroke produces exact Color32 bytes.
    #[test]
    fn test_hsva_fg_stroke_exact() {
        let expected = Color32::from_rgb(187, 210, 213);
        let actual: Color32 = Hsva::new(0.5206857447508191, 0.25316729618415346, 0.665387298282272, 1.0).into();
        assert_eq!(expected, actual, "fg_stroke color mismatch: expected {:?}, got {:?}", expected, actual);
    }
}
