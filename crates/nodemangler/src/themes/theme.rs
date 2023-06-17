use eframe::egui;
use eframe::egui::style::WidgetVisuals;
use eframe::egui::style::Widgets;
use epaint::Color32;
use epaint::Rounding;
use epaint::Shadow;
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

#[derive(Clone, PartialEq, Debug)]
pub enum Theme {
    Light,
    DarkGreen,
}

impl Theme {
    pub fn list() -> Vec<Theme> {
        vec![
            Theme::Light,
            Theme::DarkGreen,
        ]
    }

    pub fn name(&self) -> String {
        match self {
            Theme::Light => "Light".to_string(),
            Theme::DarkGreen => "Dark Green".to_string(),
        }
    }

    pub fn get(&self) -> ThemeValues {
        match self {
            Theme::Light => super::theme_light::LIGHT,
            Theme::DarkGreen => super::theme_dark_green::DARK_GREEN,
        }
    }
}


pub fn set_theme(ctx: &egui::Context, theme: Theme) {
    let old = ctx.style().visuals.clone();
    let theme_values = theme.get();

    ctx.set_visuals(egui::Visuals {
        panel_fill: Color32::from(theme_values.panel_fill),
        extreme_bg_color: Color32::from(theme_values.extreme_bg_color),
        override_text_color: Some(Color32::from(theme_values.override_text_color)),
        dark_mode: theme_values.dark_mode,
        //window_shadow: Shadow::NONE,
        popup_shadow: Shadow::NONE,
        
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
