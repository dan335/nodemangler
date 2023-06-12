use eframe::egui;
use eframe::egui::style::WidgetVisuals;
use eframe::egui::style::Widgets;
use epaint::Color32;
use epaint::Rounding;
use epaint::Stroke;

#[derive(Clone)]
pub struct ThemeValues {
    pub panel_fill: Color32,  // bg of everything
    pub extreme_bg_color: Color32,
    //pub panel_border_lines: HsvaGamma,
    pub override_text_color: Color32,
    pub menu_bar: Color32,    // top bar with window controls
    pub menu_bar_button: Color32,
    pub menu_bar_button_selected: Color32,
    // pub node_menu_bg: Color32,
    // pub node_menu_bg_hover: Color32,
    pub grid_bg: Color32,
    pub grid_lines: Color32,
    pub grid_connection_line: Color32,
    pub grid_connection_line_width: f32,
    pub grid_connection_dot: Color32,
    pub grid_connection_dot_hover: Color32,
    pub grid_connection_dot_disabled: Color32,
    //pub grid_connection_dot_viewing: HsvaGamma,
    pub node_header_bg: Color32,
    pub node_header_selected_border: Color32,
    pub text_faint: Color32,
    pub widgets_noninteractive_bg_fill: Color32,
    pub widgets_noninteractive_weak_bg_fill: Color32,
    pub widgets_noninteractive_bg_stroke: (f32, Color32), // width, color
    pub widgets_noninteractive_rounding: f32,
    pub widgets_noninteractive_fg_stroke: (f32, Color32), // width, color
    pub widgets_noninteractive_expansion: f32,
    pub widgets_interactive_bg_fill: Color32,
    pub widgets_interactive_weak_bg_fill: Color32,
    pub widgets_interactive_bg_stroke: (f32, Color32), // width, color
    pub widgets_interactive_rounding: f32,
    pub widgets_interactive_fg_stroke: (f32, Color32), // width, color
    pub widgets_interactive_expansion: f32,
    pub widgets_hovered_bg_fill: Color32,
    pub widgets_hovered_weak_bg_fill: Color32,
    pub widgets_hovered_bg_stroke: (f32, Color32), // width, color
    pub widgets_hovered_rounding: f32,
    pub widgets_hovered_fg_stroke: (f32, Color32), // width, color
    pub widgets_hovered_expansion: f32,
    pub widgets_active_bg_fill: Color32,
    pub widgets_active_weak_bg_fill: Color32,
    pub widgets_active_bg_stroke: (f32, Color32), // width, color
    pub widgets_active_rounding: f32,
    pub widgets_active_fg_stroke: (f32, Color32), // width, color
    pub widgets_active_expansion: f32,
    pub widgets_open_bg_fill: Color32,
    pub widgets_open_weak_bg_fill: Color32,
    pub widgets_open_bg_stroke: (f32, Color32), // width, color
    pub widgets_open_rounding: f32,
    pub widgets_open_fg_stroke: (f32, Color32), // width, color
    pub widgets_open_expansion: f32,
    pub dark_mode: bool,
}

#[derive(Clone)]
pub enum Theme {
    Light,
    DarkGreen,
}

impl Theme {
    pub fn get(&self) -> ThemeValues {
        match self {
            Theme::Light => LIGHT,
            Theme::DarkGreen => DARK_GREEN,
        }
    }
}

// pub const DARK_OLD: Theme = Theme {
//     panel_fill: HsvaGamma { h: 0.353, s: 0.1, v: 0.16, a: 1.0 },
//     extreme_bg_color: HsvaGamma { h: 0.0, s: 0.0, v: 0.1, a: 1.0 },
//     //panel_border_lines: HsvaGamma { h: 0.353, s: 0.1, v: 0.02, a: 1.0 },
//     override_text_color: HsvaGamma { h: 0.353, s: 0.1, v: 0.85, a:1.0 },
//     menu_bar: HsvaGamma { h: 0.353, s: 0.1, v: 0.1, a: 1.0 },
//     menu_bar_button: HsvaGamma { h: 0.353, s: 0.1, v: 0.2, a: 1.0 },
//     menu_bar_button_selected: HsvaGamma { h: 0.353, s: 0.1, v: 0.3, a: 1.0 },

//     node_menu_bg: HsvaGamma { h: 0.353, s: 0.1, v: 0.2, a: 1.0 },
//     node_menu_bg_hover: HsvaGamma { h: 0.353, s: 0.1, v: 0.4, a: 1.0 },

//     //grid_bg: Hsva { h: 0.353, s: 0.1, v: 0.13, a: 1.0 },
//     grid_bg: Color32::from_rgb(32, 41, 42),
//     grid_lines: HsvaGamma { h: 0.353, s: 0.1, v: 0.16, a: 1.0 },
//     grid_connection_line: HsvaGamma { h: 0.353, s: 0.1, v: 0.4, a:1.0 },
//     grid_connection_line_width: 1.0,
//     grid_connection_dot: HsvaGamma { h: 0.353, s: 0.1, v: 0.4, a:1.0 },
//     grid_connection_dot_hover: HsvaGamma { h: 0.353, s: 0.1, v: 0.7, a:1.0 },
//     grid_connection_dot_disabled: HsvaGamma { h: 0.353, s: 0.1, v: 0.25, a:1.0 },
//     //grid_connection_dot_viewing: HsvaGamma { h: 0.353, s: 0.1, v: 0.95, a: 1.0 },
//     node_header_bg: HsvaGamma { h: 0.353, s: 0.1, v: 0.22, a: 1.0 },
//     node_header_selected_border: HsvaGamma { h: 0.353, s: 0.1, v: 0.8, a: 1.0 },
    
//     text_faint: HsvaGamma { h: 0.353, s: 0.1, v: 0.4, a: 1.0 },
    
//     widgets_noninteractive_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.1, a: 1.0 },
//     widgets_noninteractive_weak_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.1, a: 1.0 },
//     widgets_noninteractive_bg_stroke: (0.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 0.0 }),
//     widgets_noninteractive_rounding: 1.0,
//     widgets_noninteractive_fg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.85, a: 1.0 }),
//     widgets_noninteractive_expansion: 3.0,

//     widgets_interactive_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.1, a: 1.0 },
//     widgets_interactive_weak_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.1, a: 1.0 },
//     widgets_interactive_bg_stroke: (0.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 0.0 }),
//     widgets_interactive_rounding: 1.0,
//     widgets_interactive_fg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.85, a: 1.0 }),
//     widgets_interactive_expansion: 3.0,

//     widgets_hovered_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.1, a: 1.0 },
//     widgets_hovered_weak_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.1, a: 1.0 },
//     widgets_hovered_bg_stroke: (0.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 0.0 }),
//     widgets_hovered_rounding: 1.0,
//     widgets_hovered_fg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.85, a: 1.0 }),
//     widgets_hovered_expansion: 3.0,

//     widgets_active_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.1, a: 1.0 },
//     widgets_active_weak_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.1, a: 1.0 },
//     widgets_active_bg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 0.0 }),
//     widgets_active_rounding: 1.0,
//     widgets_active_fg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.85, a: 1.0 }),
//     widgets_active_expansion: 3.0,

//     widgets_open_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.1, a: 1.0 },
//     widgets_open_weak_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.1, a: 1.0 },
//     widgets_open_bg_stroke: (0.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 0.0 }),
//     widgets_open_rounding: 1.0,
//     widgets_open_fg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.85, a: 1.0 }),
//     widgets_open_expansion: 3.0,

//     dark_mode: true,
// };

const DARK_GREEN: ThemeValues = ThemeValues {
    panel_fill: Color32::from_rgb(42, 54, 56),
    extreme_bg_color: Color32::from_rgb(29, 37, 38),
    //panel_border_lines: HsvaGamma { h: 0.353, s: 0.1, v: 0.02, a: 1.0 },
    override_text_color: Color32::from_rgb(187, 210, 213),
    menu_bar: Color32::from_rgb(29, 37, 38),
    menu_bar_button: Color32::from_rgb(29, 37, 38),
    menu_bar_button_selected: Color32::from_rgb(187, 52, 71),

    // node_menu_bg: HsvaGamma { h: 0.353, s: 0.1, v: 0.2, a: 1.0 },
    // node_menu_bg_hover: HsvaGamma { h: 0.353, s: 0.1, v: 0.4, a: 1.0 },

    //grid_bg: HsvaGamma { h: 0.27058, s: 0.14, v: 0.15, a: 1.0 },
    grid_bg: Color32::from_rgb(32, 41, 42),
    grid_lines: Color32::from_rgb(42, 54, 56),
    grid_connection_line: Color32::from_rgb(24, 140, 159),
    grid_connection_line_width: 1.0,
    grid_connection_dot: Color32::from_rgb(24, 140, 159),
    grid_connection_dot_hover: Color32::from_rgb(24, 140, 159),
    grid_connection_dot_disabled: Color32::from_rgb(24, 140, 159),
    //grid_connection_dot_viewing: HsvaGamma { h: 0.353, s: 0.1, v: 0.95, a: 1.0 },
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
    widgets_hovered_weak_bg_fill: Color32::from_rgb(29, 37, 38),
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

const LIGHT: ThemeValues = ThemeValues {
    panel_fill: Color32::from_rgb(42, 54, 56),
    extreme_bg_color: Color32::from_rgb(29, 37, 38),
    //panel_border_lines: HsvaGamma { h: 0.353, s: 0.1, v: 0.8, a: 1.0 },

    override_text_color: Color32::from_rgb(187, 210, 213),

    menu_bar: Color32::from_rgb(29, 37, 38),
    menu_bar_button: Color32::from_rgb(29, 37, 38),
    menu_bar_button_selected: Color32::from_rgb(29, 37, 38),

    // node_menu_bg: HsvaGamma { h: 0.353, s: 0.0, v: 0.8, a: 1.0 },
    // node_menu_bg_hover: HsvaGamma { h: 0.353, s: 0.0, v: 0.6, a: 1.0 },

    //grid_bg: HsvaGamma { h: 0.353, s: 0.0, v: 0.86, a: 1.0 },
    grid_bg: Color32::from_rgb(32, 41, 42),
    grid_lines: Color32::from_rgb(42, 54, 56),
    grid_connection_line: Color32::from_rgb(24, 140, 159),
    grid_connection_line_width: 1.0,
    grid_connection_dot: Color32::from_rgb(24, 140, 159),
    grid_connection_dot_hover: Color32::from_rgb(24, 140, 159),
    grid_connection_dot_disabled: Color32::from_rgb(24, 140, 159),
    //grid_connection_dot_viewing: HsvaGamma { h: 0.353, s: 0.1, v: 0.1, a: 1.0 },

    node_header_bg: Color32::from_rgb(48, 62, 63),
    node_header_selected_border: Color32::from_rgb(48, 62, 63),
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
    widgets_hovered_weak_bg_fill: Color32::from_rgb(29, 37, 38),
    widgets_hovered_bg_stroke: (0.0, Color32::WHITE),
    widgets_hovered_rounding: 1.0,
    widgets_hovered_fg_stroke: (1.0, Color32::from_rgb(187, 210, 213)),
    widgets_hovered_expansion: 3.0,

    widgets_active_bg_fill: Color32::from_rgb(29, 37, 38),
    widgets_active_weak_bg_fill: Color32::from_rgb(29, 37, 38),
    widgets_active_bg_stroke: (0.0, Color32::WHITE),
    widgets_active_rounding: 1.0,
    widgets_active_fg_stroke: (1.0, Color32::from_rgb(187, 210, 213)),
    widgets_active_expansion: 3.0,

    widgets_open_bg_fill: Color32::from_rgb(29, 37, 38),
    widgets_open_weak_bg_fill: Color32::from_rgb(29, 37, 38),
    widgets_open_bg_stroke: (0.0, Color32::WHITE),
    widgets_open_rounding: 1.0,
    widgets_open_fg_stroke: (1.0, Color32::from_rgb(187, 210, 213)),
    widgets_open_expansion: 3.0,
    
    dark_mode: false,
};

pub fn set_theme(ctx: &egui::Context, theme: Theme) {
    let old = ctx.style().visuals.clone();
    let theme_values = theme.get();

    ctx.set_visuals(egui::Visuals {
        panel_fill: Color32::from(theme_values.panel_fill),
        extreme_bg_color: Color32::from(theme_values.extreme_bg_color),
        override_text_color: Some(Color32::from(theme_values.override_text_color)),
        dark_mode: theme_values.dark_mode,
        widgets: Widgets {
            noninteractive: WidgetVisuals { bg_fill: Color32::from(theme_values.widgets_noninteractive_bg_fill), weak_bg_fill: Color32::from(theme_values.widgets_noninteractive_weak_bg_fill), bg_stroke: Stroke::new(theme_values.widgets_noninteractive_bg_stroke.0, theme_values.widgets_noninteractive_bg_stroke.1), rounding: Rounding::same(theme_values.widgets_noninteractive_rounding), fg_stroke: Stroke::new(theme_values.widgets_noninteractive_fg_stroke.0, theme_values.widgets_noninteractive_fg_stroke.1), expansion: theme_values.widgets_noninteractive_expansion },
            inactive: WidgetVisuals { bg_fill: Color32::from(theme_values.widgets_interactive_bg_fill), weak_bg_fill: Color32::from(theme_values.widgets_interactive_weak_bg_fill), bg_stroke: Stroke::new(theme_values.widgets_interactive_bg_stroke.0, theme_values.widgets_interactive_bg_stroke.1), rounding: Rounding::same(theme_values.widgets_interactive_rounding), fg_stroke: Stroke::new(theme_values.widgets_interactive_fg_stroke.0, theme_values.widgets_interactive_fg_stroke.1), expansion: theme_values.widgets_interactive_expansion },
            hovered: WidgetVisuals { bg_fill: Color32::from(theme_values.widgets_hovered_bg_fill), weak_bg_fill: Color32::from(theme_values.widgets_hovered_weak_bg_fill), bg_stroke: Stroke::new(theme_values.widgets_hovered_bg_stroke.0, theme_values.widgets_hovered_bg_stroke.1), rounding: Rounding::same(theme_values.widgets_hovered_rounding), fg_stroke: Stroke::new(theme_values.widgets_hovered_fg_stroke.0, theme_values.widgets_hovered_fg_stroke.1), expansion: theme_values.widgets_hovered_expansion },
            active: WidgetVisuals { bg_fill: Color32::from(theme_values.widgets_active_bg_fill), weak_bg_fill: Color32::from(theme_values.widgets_active_weak_bg_fill), bg_stroke: Stroke::new(theme_values.widgets_active_bg_stroke.0, theme_values.widgets_active_bg_stroke.1), rounding: Rounding::same(theme_values.widgets_active_rounding), fg_stroke: Stroke::new(theme_values.widgets_active_fg_stroke.0, theme_values.widgets_active_fg_stroke.1), expansion: theme_values.widgets_active_expansion },
            open: WidgetVisuals { bg_fill: Color32::from(theme_values.widgets_open_bg_fill), weak_bg_fill: Color32::from(theme_values.widgets_open_weak_bg_fill), bg_stroke: Stroke::new(theme_values.widgets_open_bg_stroke.0, theme_values.widgets_open_bg_stroke.1), rounding: Rounding::same(theme_values.widgets_open_rounding), fg_stroke: Stroke::new(theme_values.widgets_open_fg_stroke.0, theme_values.widgets_open_fg_stroke.1), expansion: theme_values.widgets_open_expansion },
        },
        ..old
    });
}
