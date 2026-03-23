use epaint::{Color32, CornerRadius, Hsva, Shadow, Stroke};
use super::theme::ThemeValues;

pub fn theme_light_blue() -> ThemeValues
{
    // Reusable palette colors
    let menu_bar: Color32 = Hsva::new(0.565, 0.4, 0.92, 1.0).into();
    let pink_accent: Color32 = Hsva::new(0.95, 0.69, 0.97, 1.0).into();
    let blue_bright: Color32 = Hsva::new(0.565, 0.81, 0.97, 1.0).into();
    let blue_light: Color32 = Hsva::new(0.565, 0.15, 0.9, 1.0).into();

    ThemeValues {
        panel_fill: Hsva::new(0.0, 0.0, 0.95, 1.0).into(),
        extreme_bg_color: Hsva::new(0.565, 0.15, 0.8, 1.0).into(),

        menu_bar: menu_bar,
        menu_bar_button: blue_light,
        menu_bar_button_selected: pink_accent,

        grid_bg: Hsva::new(0.565, 0.1, 0.95, 1.0).into(),
        grid_lines: Hsva::new(0.565, 0.1, 0.85, 1.0).into(),
        grid_connection_line: blue_bright,
        grid_connection_line_width: 1.0,
        grid_connection_dot: blue_bright,
        grid_connection_dot_hover: blue_bright,
        grid_connection_dot_disabled: Hsva::new(0.565, 0.1, 0.62, 1.0).into(),
        grid_connection_dot_error: pink_accent,

        node_header_bg: Hsva::new(0.565, 0.25, 0.88, 1.0).into(),
        node_header_selected_border: pink_accent,

        override_text_color: Hsva::new(0.565, 0.05, 0.1, 1.0).into(),
        text_faint: Hsva::new(0.565, 0.05, 0.3, 1.0).into(),

        histogram_bg: Hsva::new(0.565, 0.08, 0.9, 1.0).into(),
        histogram_luminance: Hsva::new(0.565, 0.08, 0.7, 1.0).into(),
        histogram_red: Hsva::new(0.989, 0.763, 0.745, 0.57).into(),
        histogram_green: Hsva::new(0.407, 0.806, 0.608, 0.55).into(),
        histogram_blue: Hsva::new(0.630, 0.800, 0.784, 0.57).into(),

        window_corner_radius: CornerRadius::same(1),
        window_shadow: Shadow::NONE,
        window_fill: Hsva::new(0.565, 0.4, 0.82, 1.0).into(),
        window_stroke: Stroke::NONE,

        widgets_noninteractive_bg_fill: menu_bar,
        widgets_noninteractive_weak_bg_fill: menu_bar,
        widgets_noninteractive_bg_stroke: (0.0, Color32::WHITE),
        widgets_noninteractive_rounding: 1.0,
        widgets_noninteractive_fg_stroke: (0.0, blue_light),
        widgets_noninteractive_expansion: 3.0,

        widgets_interactive_bg_fill: menu_bar,
        widgets_interactive_weak_bg_fill: menu_bar,
        widgets_interactive_bg_stroke: (0.0, Color32::WHITE),
        widgets_interactive_rounding: 1.0,
        widgets_interactive_fg_stroke: (0.0, blue_light),
        widgets_interactive_expansion: 3.0,

        widgets_hovered_bg_fill: menu_bar,
        widgets_hovered_weak_bg_fill: pink_accent,
        widgets_hovered_bg_stroke: (0.0, Color32::WHITE),
        widgets_hovered_rounding: 1.0,
        widgets_hovered_fg_stroke: (0.0, blue_light),
        widgets_hovered_expansion: 3.0,

        widgets_active_bg_fill: menu_bar,
        widgets_active_weak_bg_fill: menu_bar,
        widgets_active_bg_stroke: (0.0, Color32::WHITE),
        widgets_active_rounding: 1.0,
        widgets_active_fg_stroke: (0.0, blue_light),
        widgets_active_expansion: 3.0,

        widgets_open_bg_fill: menu_bar,
        widgets_open_weak_bg_fill: menu_bar,
        widgets_open_bg_stroke: (0.0, Color32::WHITE),
        widgets_open_rounding: 1.0,
        widgets_open_fg_stroke: (0.0, blue_light),
        widgets_open_expansion: 3.0,

        dark_mode: false,
    }
}
