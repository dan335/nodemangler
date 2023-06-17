use epaint::Color32;
use super::theme::ThemeValues;

pub const DARK_GREEN: ThemeValues = ThemeValues {
    panel_fill: Color32::from_rgb(42, 54, 56),
    extreme_bg_color: Color32::from_rgb(29, 37, 38),
    override_text_color: Color32::from_rgb(187, 210, 213),
    menu_bar: Color32::from_rgb(29, 37, 38),
    menu_bar_button: Color32::from_rgb(29, 37, 38),
    menu_bar_button_selected: Color32::from_rgb(187, 52, 71),

    grid_bg: Color32::from_rgb(32, 41, 42),
    grid_lines: Color32::from_rgb(42, 54, 56),
    grid_connection_line: Color32::from_rgb(24, 140, 159),
    grid_connection_line_width: 1.0,
    grid_connection_dot: Color32::from_rgb(24, 140, 159),
    grid_connection_dot_hover: Color32::from_rgb(24, 140, 159),
    grid_connection_dot_disabled: Color32::from_rgb(24, 140, 159),
    node_header_bg: Color32::from_rgb(48, 62, 63),
    node_header_selected_border: Color32::from_rgb(187, 52, 71),
    
    text_faint: Color32::from_rgb(82, 128, 134),
    
    widgets_noninteractive_bg_fill: Color32::from_rgb(29, 37, 38),
    widgets_noninteractive_weak_bg_fill: Color32::from_rgb(29, 37, 38),
    widgets_noninteractive_bg_stroke: (0.0, Color32::WHITE),
    widgets_noninteractive_rounding: 1.0,
    widgets_noninteractive_fg_stroke: (1.0, Color32::from_rgb(187, 210, 213)),
    widgets_noninteractive_expansion: 3.0,

    widgets_interactive_bg_fill: Color32::from_rgb(29, 37, 38),
    widgets_interactive_weak_bg_fill: Color32::from_rgb(29, 37, 38),
    widgets_interactive_bg_stroke: (0.0, Color32::WHITE),
    widgets_interactive_rounding: 1.0,
    widgets_interactive_fg_stroke: (1.0, Color32::from_rgb(187, 210, 213)),
    widgets_interactive_expansion: 3.0,

    widgets_hovered_bg_fill: Color32::from_rgb(29, 37, 38),
    widgets_hovered_weak_bg_fill: Color32::from_rgb(187, 52, 71),
    widgets_hovered_bg_stroke: (0.0, Color32::WHITE),
    widgets_hovered_rounding: 1.0,
    widgets_hovered_fg_stroke: (1.0, Color32::from_rgb(187, 210, 213)),
    widgets_hovered_expansion: 3.0,

    widgets_active_bg_fill: Color32::from_rgb(29, 37, 38),
    widgets_active_weak_bg_fill: Color32::from_rgb(29, 37, 38),
    widgets_active_bg_stroke: (1.0, Color32::WHITE),
    widgets_active_rounding: 1.0,
    widgets_active_fg_stroke: (1.0, Color32::from_rgb(187, 210, 213)),
    widgets_active_expansion: 3.0,

    widgets_open_bg_fill: Color32::from_rgb(29, 37, 38),
    widgets_open_weak_bg_fill: Color32::from_rgb(29, 37, 38),
    widgets_open_bg_stroke: (0.0, Color32::WHITE),
    widgets_open_rounding: 1.0,
    widgets_open_fg_stroke: (1.0, Color32::from_rgb(187, 210, 213)),
    widgets_open_expansion: 3.0,

    dark_mode: true,
};