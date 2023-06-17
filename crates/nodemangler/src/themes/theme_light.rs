use epaint::{Color32, Hsva};
use super::{theme::{ThemeValues, desaturate}, theme_light_blue::theme_light_blue};

pub fn theme_light() -> ThemeValues 
{
    let other = theme_light_blue();

    ThemeValues {
        panel_fill: desaturate(other.panel_fill),
        extreme_bg_color: desaturate(other.extreme_bg_color),
        override_text_color: desaturate(other.override_text_color),
        menu_bar: Hsva::new(0.565, 0.0, 0.8, 1.0).into(),
        menu_bar_button: desaturate(other.menu_bar_button),
        menu_bar_button_selected: Hsva::new(0.555, 0.79, 0.97, 1.0).into(),

        grid_bg: Hsva::new(0.565, 0.0, 0.88, 1.0).into(),
        grid_lines: Hsva::new(0.565, 0.0, 0.8, 1.0).into(),
        grid_connection_line: Hsva::new(0.555, 0.0, 0.55, 1.0).into(),
        grid_connection_line_width: 1.0,
        grid_connection_dot: Hsva::new(0.555, 0.0, 0.55, 1.0).into(),
        grid_connection_dot_hover: Hsva::new(0.555, 0.3, 0.8, 1.0).into(),
        grid_connection_dot_disabled: desaturate(other.grid_connection_dot_disabled),
        node_header_bg: Hsva::new(0.565, 0.0, 0.65, 1.0).into(),
        node_header_selected_border: Hsva::new(0.555, 0.79, 0.97, 1.0).into(),
        
        text_faint: desaturate(other.text_faint),

        window_rounding: other.window_rounding,
        window_shadow:other.window_shadow,
        window_fill: Hsva::new(0.565, 0.0, 0.75, 1.0).into(),
        window_stroke: other.window_stroke,
        
        widgets_noninteractive_bg_fill: Hsva::new(0.565, 0.0, 0.8, 1.0).into(),
        widgets_noninteractive_weak_bg_fill: Hsva::new(0.565, 0.0, 0.8, 1.0).into(),
        widgets_noninteractive_bg_stroke: (0.0, Color32::WHITE),
        widgets_noninteractive_rounding: 1.0,
        widgets_noninteractive_fg_stroke: (1.0, Color32::from_rgb(187, 210, 213)),
        widgets_noninteractive_expansion: 3.0,

        widgets_interactive_bg_fill: Hsva::new(0.565, 0.0, 0.8, 1.0).into(),
        widgets_interactive_weak_bg_fill: Hsva::new(0.565, 0.0, 0.8, 1.0).into(),
        widgets_interactive_bg_stroke: (0.0, Color32::WHITE),
        widgets_interactive_rounding: 1.0,
        widgets_interactive_fg_stroke: (1.0, Color32::from_rgb(187, 210, 213)),
        widgets_interactive_expansion: 3.0,

        widgets_hovered_bg_fill: Hsva::new(0.565, 0.0, 0.8, 1.0).into(),
        widgets_hovered_weak_bg_fill: desaturate(other.widgets_hovered_weak_bg_fill),
        widgets_hovered_bg_stroke: (0.0, Color32::WHITE),
        widgets_hovered_rounding: 1.0,
        widgets_hovered_fg_stroke: (1.0, Color32::from_rgb(187, 210, 213)),
        widgets_hovered_expansion: 3.0,

        widgets_active_bg_fill: Hsva::new(0.565, 0.0, 0.8, 1.0).into(),
        widgets_active_weak_bg_fill:Hsva::new(0.565, 0.0, 0.8, 1.0).into(),
        widgets_active_bg_stroke: (1.0, Color32::WHITE),
        widgets_active_rounding: 1.0,
        widgets_active_fg_stroke: (1.0, Color32::from_rgb(187, 210, 213)),
        widgets_active_expansion: 3.0,

        widgets_open_bg_fill: Hsva::new(0.565, 0.0, 0.8, 1.0).into(),
        widgets_open_weak_bg_fill:Hsva::new(0.565, 0.0, 0.8, 1.0).into(),
        widgets_open_bg_stroke: (0.0, Color32::WHITE),
        widgets_open_rounding: 1.0,
        widgets_open_fg_stroke: (1.0, Color32::from_rgb(187, 210, 213)),
        widgets_open_expansion: 3.0,

        dark_mode: false,
    }
}