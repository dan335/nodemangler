use epaint::{Color32, Hsva};
use super::{theme::{ThemeValues, desaturate}, theme_light_blue::theme_light_blue};

pub fn theme_light() -> ThemeValues 
{
    let other = theme_light_blue();
    let menu: Color32 = Hsva::new(0.565, 0.0, 0.75, 1.0).into();

    ThemeValues {
        panel_fill: desaturate(other.panel_fill),
        extreme_bg_color: desaturate(other.extreme_bg_color),
        override_text_color: desaturate(other.override_text_color),
        menu_bar: menu,
        menu_bar_button: desaturate(other.menu_bar_button),
        menu_bar_button_selected: Hsva::new(0.555, 0.79, 0.97, 1.0).into(),

        grid_bg: Hsva::new(0.565, 0.0, 0.84, 1.0).into(),
        grid_lines: Hsva::new(0.565, 0.0, 0.73, 1.0).into(),
        grid_connection_line: Hsva::new(0.555, 0.0, 0.5, 1.0).into(),
        grid_connection_line_width: 1.0,
        grid_connection_dot: Hsva::new(0.555, 0.0, 0.5, 1.0).into(),
        grid_connection_dot_hover: Hsva::new(0.555, 0.0, 0.6, 1.0).into(),
        grid_connection_dot_disabled: Hsva::new(0.555, 0.0, 0.75, 1.0).into(),
        grid_connection_dot_error: Hsva::new(0.555, 0.79, 0.97, 1.0).into(),

        node_header_bg: Hsva::new(0.565, 0.0, 0.65, 1.0).into(),
        node_header_selected_border: Hsva::new(0.555, 0.79, 0.97, 1.0).into(),
        
        text_faint: desaturate(other.text_faint),

        window_corner_radius: other.window_corner_radius,
        window_shadow:other.window_shadow,
        window_fill: Hsva::new(0.565, 0.0, 0.7, 1.0).into(),
        window_stroke: other.window_stroke,
        
        widgets_noninteractive_bg_fill: menu,
        widgets_noninteractive_weak_bg_fill: menu,
        widgets_noninteractive_bg_stroke: (0.0, Color32::WHITE),
        widgets_noninteractive_rounding: 1.0,
        widgets_noninteractive_fg_stroke: (0.0, Color32::from_rgb(187, 210, 213)),
        widgets_noninteractive_expansion: 3.0,

        widgets_interactive_bg_fill: menu,
        widgets_interactive_weak_bg_fill: menu,
        widgets_interactive_bg_stroke: (0.0, Color32::WHITE),
        widgets_interactive_rounding: 1.0,
        widgets_interactive_fg_stroke: (0.0, Color32::from_rgb(187, 210, 213)),
        widgets_interactive_expansion: 3.0,

        widgets_hovered_bg_fill: menu,
        widgets_hovered_weak_bg_fill: desaturate(other.widgets_hovered_weak_bg_fill),
        widgets_hovered_bg_stroke: (0.0, Color32::WHITE),
        widgets_hovered_rounding: 1.0,
        widgets_hovered_fg_stroke: (0.0, Color32::from_rgb(187, 210, 213)),
        widgets_hovered_expansion: 3.0,

        widgets_active_bg_fill: menu,
        widgets_active_weak_bg_fill: menu,
        widgets_active_bg_stroke: (0.0, Color32::WHITE),
        widgets_active_rounding: 1.0,
        widgets_active_fg_stroke: (0.0, Color32::from_rgb(187, 210, 213)),
        widgets_active_expansion: 3.0,

        widgets_open_bg_fill: menu,
        widgets_open_weak_bg_fill: menu,
        widgets_open_bg_stroke: (0.0, Color32::WHITE),
        widgets_open_rounding: 1.0,
        widgets_open_fg_stroke: (0.0, Color32::from_rgb(187, 210, 213)),
        widgets_open_expansion: 3.0,

        dark_mode: false,
    }
}