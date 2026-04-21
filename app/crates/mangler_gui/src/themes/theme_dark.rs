use epaint::{Color32, Hsva};
use super::{theme::{ThemeValues, desaturate}, theme_dark_green::theme_dark_green};

pub fn theme_dark() -> ThemeValues
{
    let dark_green = theme_dark_green();

    // Reusable palette colors
    let light_teal_text: Color32 = Hsva::new(0.5206857447508191, 0.25316729618415346, 0.665387298282272, 1.0).into();   // rgb(187, 210, 213)

    ThemeValues {
        panel_fill: desaturate(dark_green.panel_fill),
        extreme_bg_color: desaturate(dark_green.extreme_bg_color),
        override_text_color: desaturate(dark_green.override_text_color),
        menu_bar: desaturate(dark_green.menu_bar),
        menu_bar_button: desaturate(dark_green.menu_bar_button),
        menu_bar_button_selected: dark_green.menu_bar_button_selected,

        grid_bg: desaturate(dark_green.grid_bg),
        grid_lines: desaturate(dark_green.grid_lines),
        grid_connection_line: desaturate(dark_green.grid_connection_line),
        grid_connection_line_width: 1.0,
        grid_connection_dot: desaturate(dark_green.grid_connection_dot),
        grid_connection_dot_hover: desaturate(dark_green.grid_connection_dot_hover),
        grid_connection_dot_disabled: Hsva::new(0.522, 0.0, 0.075, 1.0).into(),
        grid_connection_dot_error: dark_green.menu_bar_button_selected,

        node_header_dirty_bg: desaturate(dark_green.node_header_dirty_bg),
        node_header_bg: desaturate(dark_green.node_header_bg),
        node_header_selected_border: dark_green.node_header_selected_border,

        text_faint: desaturate(dark_green.text_faint),

        // Histogram: neutral grays for bg/luminance, slightly muted RGB
        histogram_bg: Hsva::new(0.0, 0.0, 0.086, 1.0).into(),
        histogram_luminance: Hsva::new(0.0, 0.0, 0.243, 1.0).into(),
        histogram_red: Hsva::new(0.0, 0.725, 0.784, 0.53).into(),
        histogram_green: Hsva::new(0.333, 0.725, 0.784, 0.49).into(),
        histogram_blue: Hsva::new(0.667, 0.725, 0.784, 0.49).into(),

        window_corner_radius: dark_green.window_corner_radius,
        window_shadow: dark_green.window_shadow,
        window_fill: desaturate(dark_green.window_fill),
        window_stroke: dark_green.window_stroke,

        widgets_noninteractive_bg_fill: desaturate(dark_green.widgets_noninteractive_bg_fill),
        widgets_noninteractive_weak_bg_fill: desaturate(dark_green.widgets_noninteractive_weak_bg_fill),
        widgets_noninteractive_bg_stroke: (0.0, Color32::WHITE),
        widgets_noninteractive_rounding: 1.0,
        widgets_noninteractive_fg_stroke: (1.0, light_teal_text),
        widgets_noninteractive_expansion: 0.0,

        widgets_interactive_bg_fill: desaturate(dark_green.widgets_interactive_bg_fill),
        widgets_interactive_weak_bg_fill: desaturate(dark_green.widgets_interactive_weak_bg_fill),
        widgets_interactive_bg_stroke: (0.0, Color32::WHITE),
        widgets_interactive_rounding: 1.0,
        widgets_interactive_fg_stroke: (1.0, light_teal_text),
        widgets_interactive_expansion: 0.0,

        widgets_hovered_bg_fill: desaturate(dark_green.widgets_hovered_bg_fill),
        widgets_hovered_weak_bg_fill: dark_green.widgets_hovered_weak_bg_fill,
        widgets_hovered_bg_stroke: (0.0, Color32::WHITE),
        widgets_hovered_rounding: 1.0,
        widgets_hovered_fg_stroke: (1.0, light_teal_text),
        widgets_hovered_expansion: 0.0,

        widgets_active_bg_fill: desaturate(dark_green.widgets_active_bg_fill),
        widgets_active_weak_bg_fill: desaturate(dark_green.widgets_active_weak_bg_fill),
        widgets_active_bg_stroke: (0.0, Color32::WHITE),
        widgets_active_rounding: 1.0,
        widgets_active_fg_stroke: (1.0, light_teal_text),
        widgets_active_expansion: 0.0,

        widgets_open_bg_fill: desaturate(dark_green.widgets_open_bg_fill),
        widgets_open_weak_bg_fill: desaturate(dark_green.widgets_open_weak_bg_fill),
        widgets_open_bg_stroke: (0.0, Color32::WHITE),
        widgets_open_rounding: 1.0,
        widgets_open_fg_stroke: (1.0, light_teal_text),
        widgets_open_expansion: 0.0,

        dark_mode: true,
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
