use eframe::egui;
use eframe::egui::style::WidgetVisuals;
use eframe::egui::style::Widgets;
use epaint::Color32;
use epaint::HsvaGamma;
use epaint::Rounding;
use epaint::Stroke;

#[derive(Clone)]
pub struct Theme {
    pub panel_fill: HsvaGamma,  // bg of everything
    pub panel_border_lines: HsvaGamma,
    pub override_text_color: HsvaGamma,
    pub menu_bar: HsvaGamma,    // top bar with window controls
    pub menu_bar_button: HsvaGamma,
    pub menu_bar_button_selected: HsvaGamma,
    pub node_menu_bg: HsvaGamma,
    pub node_menu_bg_hover: HsvaGamma,
    pub grid_bg: HsvaGamma,
    pub grid_lines: HsvaGamma,
    pub grid_connection_line: HsvaGamma,
    pub grid_connection_line_width: f32,
    pub grid_connection_dot: HsvaGamma,
    pub grid_connection_dot_hover: HsvaGamma,
    pub grid_connection_dot_disabled: HsvaGamma,
    pub grid_connection_dot_viewing: HsvaGamma,
    pub node_header_bg: HsvaGamma,
    pub node_header_selected_border: HsvaGamma,
    pub text_faint: HsvaGamma,
    pub widgets_noninteractive_bg_fill: HsvaGamma,
    pub widgets_noninteractive_weak_bg_fill: HsvaGamma,
    pub widgets_noninteractive_bg_stroke: (f32, HsvaGamma), // width, color
    pub widgets_noninteractive_rounding: f32,
    pub widgets_noninteractive_fg_stroke: (f32, HsvaGamma), // width, color
    pub widgets_noninteractive_expansion: f32,
    pub widgets_interactive_bg_fill: HsvaGamma,
    pub widgets_interactive_weak_bg_fill: HsvaGamma,
    pub widgets_interactive_bg_stroke: (f32, HsvaGamma), // width, color
    pub widgets_interactive_rounding: f32,
    pub widgets_interactive_fg_stroke: (f32, HsvaGamma), // width, color
    pub widgets_interactive_expansion: f32,
    pub widgets_hovered_bg_fill: HsvaGamma,
    pub widgets_hovered_weak_bg_fill: HsvaGamma,
    pub widgets_hovered_bg_stroke: (f32, HsvaGamma), // width, color
    pub widgets_hovered_rounding: f32,
    pub widgets_hovered_fg_stroke: (f32, HsvaGamma), // width, color
    pub widgets_hovered_expansion: f32,
    pub widgets_active_bg_fill: HsvaGamma,
    pub widgets_active_weak_bg_fill: HsvaGamma,
    pub widgets_active_bg_stroke: (f32, HsvaGamma), // width, color
    pub widgets_active_rounding: f32,
    pub widgets_active_fg_stroke: (f32, HsvaGamma), // width, color
    pub widgets_active_expansion: f32,
    pub widgets_open_bg_fill: HsvaGamma,
    pub widgets_open_weak_bg_fill: HsvaGamma,
    pub widgets_open_bg_stroke: (f32, HsvaGamma), // width, color
    pub widgets_open_rounding: f32,
    pub widgets_open_fg_stroke: (f32, HsvaGamma), // width, color
    pub widgets_open_expansion: f32,
    pub dark_mode: bool,
}

pub const DARK: Theme = Theme {
    panel_fill: HsvaGamma { h: 0.353, s: 0.1, v: 0.16, a: 1.0 },
    panel_border_lines: HsvaGamma { h: 0.353, s: 0.1, v: 0.02, a: 1.0 },
    override_text_color: HsvaGamma { h: 0.353, s: 0.1, v: 0.8, a:1.0 },
    menu_bar: HsvaGamma { h: 0.353, s: 0.1, v: 0.1, a: 1.0 },
    menu_bar_button: HsvaGamma { h: 0.353, s: 0.1, v: 0.2, a: 1.0 },
    menu_bar_button_selected: HsvaGamma { h: 0.353, s: 0.1, v: 0.3, a: 1.0 },
    node_menu_bg: HsvaGamma { h: 0.353, s: 0.1, v: 0.2, a: 1.0 },
    node_menu_bg_hover: HsvaGamma { h: 0.353, s: 0.1, v: 0.4, a: 1.0 },
    grid_bg: HsvaGamma { h: 0.353, s: 0.1, v: 0.13, a: 1.0 },
    grid_lines: HsvaGamma { h: 0.353, s: 0.1, v: 0.16, a: 1.0 },
    grid_connection_line: HsvaGamma { h: 0.353, s: 0.1, v: 0.5, a:1.0 },
    grid_connection_line_width: 1.5,
    grid_connection_dot: HsvaGamma { h: 0.353, s: 0.1, v: 0.5, a:1.0 },
    grid_connection_dot_hover: HsvaGamma { h: 0.353, s: 0.1, v: 0.7, a:1.0 },
    grid_connection_dot_disabled: HsvaGamma { h: 0.353, s: 0.1, v: 0.3, a:1.0 },
    grid_connection_dot_viewing: HsvaGamma { h: 0.353, s: 0.1, v: 0.95, a: 1.0 },
    node_header_bg: HsvaGamma { h: 0.353, s: 0.1, v: 0.3, a: 1.0 },
    node_header_selected_border: HsvaGamma { h: 0.353, s: 0.1, v: 0.8, a: 1.0 },
    
    text_faint: HsvaGamma { h: 0.353, s: 0.1, v: 0.5, a: 1.0 },
    
    widgets_noninteractive_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.2, a: 1.0 },
    widgets_noninteractive_weak_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.15, a: 1.0 },
    widgets_noninteractive_bg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 1.0 }),
    widgets_noninteractive_rounding: 0.0,
    widgets_noninteractive_fg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 1.0 }),
    widgets_noninteractive_expansion: 0.0,

    widgets_interactive_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.3, a: 1.0 },
    widgets_interactive_weak_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.25, a: 1.0 },
    widgets_interactive_bg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 1.0 }),
    widgets_interactive_rounding: 0.0,
    widgets_interactive_fg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 1.0 }),
    widgets_interactive_expansion: 0.0,

    widgets_hovered_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.4, a: 1.0 },
    widgets_hovered_weak_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.35, a: 1.0 },
    widgets_hovered_bg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 1.0 }),
    widgets_hovered_rounding: 0.0,
    widgets_hovered_fg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 1.0 }),
    widgets_hovered_expansion: 0.0,

    widgets_active_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.5, a: 1.0 },
    widgets_active_weak_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.45, a: 1.0 },
    widgets_active_bg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 1.0 }),
    widgets_active_rounding: 0.0,
    widgets_active_fg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 1.0 }),
    widgets_active_expansion: 0.0,

    widgets_open_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.5, a: 1.0 },
    widgets_open_weak_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.45, a: 1.0 },
    widgets_open_bg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 1.0 }),
    widgets_open_rounding: 0.0,
    widgets_open_fg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 1.0 }),
    widgets_open_expansion: 0.0,

    dark_mode: true,
    
};

pub const LIGHT: Theme = Theme {
    panel_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.9, a: 1.0 },
    panel_border_lines: HsvaGamma { h: 0.353, s: 0.1, v: 0.8, a: 1.0 },

    override_text_color: HsvaGamma { h: 0.353, s: 0.1, v: 0.1, a:1.0 },

    menu_bar: HsvaGamma { h: 0.353, s: 0.05, v: 0.7, a: 1.0 },
    menu_bar_button: HsvaGamma { h: 0.353, s: 0.05, v: 0.8, a: 1.0 },
    menu_bar_button_selected: HsvaGamma { h: 0.353, s: 0.05, v: 0.6, a: 1.0 },

    node_menu_bg: HsvaGamma { h: 0.353, s: 0.1, v: 0.8, a: 1.0 },
    node_menu_bg_hover: HsvaGamma { h: 0.353, s: 0.1, v: 0.6, a: 1.0 },

    grid_bg: HsvaGamma { h: 0.353, s: 0.025, v: 0.9, a: 1.0 },
    grid_lines: HsvaGamma { h: 0.353, s: 0.025, v: 0.84, a: 1.0, },
    grid_connection_line: HsvaGamma { h: 0.353, s: 0.1, v: 0.5, a:1.0 },
    grid_connection_line_width: 1.5,
    grid_connection_dot: HsvaGamma { h: 0.353, s: 0.1, v: 0.5, a:1.0 },
    grid_connection_dot_hover: HsvaGamma { h: 0.353, s: 0.1, v: 0.3, a:1.0 },
    grid_connection_dot_disabled: HsvaGamma { h: 0.353, s: 0.1, v: 0.7, a:1.0 },
    grid_connection_dot_viewing: HsvaGamma { h: 0.353, s: 0.1, v: 0.1, a: 1.0 },

    node_header_bg: HsvaGamma { h: 0.353, s: 0.1, v: 0.7, a: 1.0 },
    node_header_selected_border: HsvaGamma { h: 0.353, s: 0.1, v: 0.3, a: 1.0 },
    text_faint: HsvaGamma { h: 0.353, s: 0.1, v: 0.3, a: 1.0 },
    
    widgets_noninteractive_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.2, a: 1.0 },
    widgets_noninteractive_weak_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.15, a: 1.0 },
    widgets_noninteractive_bg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 1.0 }),
    widgets_noninteractive_rounding: 1.0,
    widgets_noninteractive_fg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 1.0 }),
    widgets_noninteractive_expansion: 3.0,

    widgets_interactive_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.2, a: 1.0 },
    widgets_interactive_weak_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.15, a: 1.0 },
    widgets_interactive_bg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 0.0 }),
    widgets_interactive_rounding: 1.0,
    widgets_interactive_fg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 1.0 }),
    widgets_interactive_expansion: 3.0,

    widgets_hovered_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.3, a: 1.0 },
    widgets_hovered_weak_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.35, a: 1.0 },
    widgets_hovered_bg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 1.0 }),
    widgets_hovered_rounding: 1.0,
    widgets_hovered_fg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 1.0 }),
    widgets_hovered_expansion: 3.0,

    widgets_active_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.5, a: 1.0 },
    widgets_active_weak_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.45, a: 1.0 },
    widgets_active_bg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 1.0 }),
    widgets_active_rounding: 1.0,
    widgets_active_fg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 1.0 }),
    widgets_active_expansion: 3.0,

    widgets_open_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.5, a: 1.0 },
    widgets_open_weak_bg_fill: HsvaGamma { h: 0.0, s: 0.0, v: 0.45, a: 1.0 },
    widgets_open_bg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 1.0 }),
    widgets_open_rounding: 1.0,
    widgets_open_fg_stroke: (1.0, HsvaGamma { h: 0.0, s: 0.0, v: 0.0, a: 1.0 }),
    widgets_open_expansion: 3.0,
    
    dark_mode: false,
};

pub fn set_theme(ctx: &egui::Context, theme: Theme) {
    let old = ctx.style().visuals.clone();

    ctx.set_visuals(egui::Visuals {
        panel_fill: Color32::from(theme.panel_fill),
        override_text_color: Some(Color32::from(theme.override_text_color)),
        dark_mode: theme.dark_mode,
        widgets: Widgets {
            noninteractive: WidgetVisuals { bg_fill: Color32::from(theme.widgets_noninteractive_bg_fill), weak_bg_fill: Color32::from(theme.widgets_noninteractive_weak_bg_fill), bg_stroke: Stroke::new(theme.widgets_noninteractive_bg_stroke.0, theme.widgets_noninteractive_bg_stroke.1), rounding: Rounding::same(theme.widgets_noninteractive_rounding), fg_stroke: Stroke::new(theme.widgets_noninteractive_fg_stroke.0, theme.widgets_noninteractive_fg_stroke.1), expansion: theme.widgets_noninteractive_expansion },
            inactive: WidgetVisuals { bg_fill: Color32::from(theme.widgets_interactive_bg_fill), weak_bg_fill: Color32::from(theme.widgets_interactive_weak_bg_fill), bg_stroke: Stroke::new(theme.widgets_interactive_bg_stroke.0, theme.widgets_interactive_bg_stroke.1), rounding: Rounding::same(theme.widgets_interactive_rounding), fg_stroke: Stroke::new(theme.widgets_interactive_fg_stroke.0, theme.widgets_interactive_fg_stroke.1), expansion: theme.widgets_interactive_expansion },
            hovered: WidgetVisuals { bg_fill: Color32::from(theme.widgets_hovered_bg_fill), weak_bg_fill: Color32::from(theme.widgets_hovered_weak_bg_fill), bg_stroke: Stroke::new(theme.widgets_hovered_bg_stroke.0, theme.widgets_hovered_bg_stroke.1), rounding: Rounding::same(theme.widgets_hovered_rounding), fg_stroke: Stroke::new(theme.widgets_hovered_fg_stroke.0, theme.widgets_hovered_fg_stroke.1), expansion: theme.widgets_hovered_expansion },
            active: WidgetVisuals { bg_fill: Color32::from(theme.widgets_active_bg_fill), weak_bg_fill: Color32::from(theme.widgets_active_weak_bg_fill), bg_stroke: Stroke::new(theme.widgets_active_bg_stroke.0, theme.widgets_active_bg_stroke.1), rounding: Rounding::same(theme.widgets_active_rounding), fg_stroke: Stroke::new(theme.widgets_active_fg_stroke.0, theme.widgets_active_fg_stroke.1), expansion: theme.widgets_active_expansion },
            open: WidgetVisuals { bg_fill: Color32::from(theme.widgets_open_bg_fill), weak_bg_fill: Color32::from(theme.widgets_open_weak_bg_fill), bg_stroke: Stroke::new(theme.widgets_open_bg_stroke.0, theme.widgets_open_bg_stroke.1), rounding: Rounding::same(theme.widgets_open_rounding), fg_stroke: Stroke::new(theme.widgets_open_fg_stroke.0, theme.widgets_open_fg_stroke.1), expansion: theme.widgets_open_expansion },
        },
        ..old
    });
}
