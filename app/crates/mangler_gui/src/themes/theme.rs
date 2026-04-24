use eframe::egui;
use eframe::egui::style::WidgetVisuals;
use eframe::egui::style::Widgets;
use epaint::Color32;
use epaint::CornerRadius;
use epaint::Hsva;
use epaint::Shadow;
use epaint::Stroke;

#[derive(Clone)]
pub struct ThemeValues {
    pub panel_fill: Color32, // bg of everything
    pub extreme_bg_color: Color32,
    //pub panel_border_lines: HsvaGamma,
    pub override_text_color: Color32,
    pub menu_bar: Color32, // top bar with window controls
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
    pub grid_connection_dot_error: Color32,
    //pub grid_connection_dot_viewing: HsvaGamma,
    pub node_header_bg: Color32,
    /// Header background for manual-run nodes with pending input changes.
    pub node_header_dirty_bg: Color32,
    pub node_header_selected_border: Color32,
    pub text_faint: Color32,

    // Histogram visualization colors
    /// Background color for the histogram chart area.
    pub histogram_bg: Color32,
    /// Luminance histogram bar color (also used as grayscale fallback).
    pub histogram_luminance: Color32,
    /// Red channel histogram bar color (should include alpha for overlay blending).
    pub histogram_red: Color32,
    /// Green channel histogram bar color (should include alpha for overlay blending).
    pub histogram_green: Color32,
    /// Blue channel histogram bar color (should include alpha for overlay blending).
    pub histogram_blue: Color32,

    pub window_corner_radius: CornerRadius,
    pub window_shadow: Shadow,
    pub window_fill: Color32,
    pub window_stroke: Stroke,

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
    Dark,
    DarkGreen,
    Light,
    LightBlue,
}

impl Theme {
    pub fn list() -> Vec<Theme> {
        vec![
            Theme::Dark,
            Theme::DarkGreen,
            Theme::Light,
            Theme::LightBlue,
        ]
    }

    pub fn name(&self) -> String {
        match self {
            Theme::Light => "light".to_string(),
            Theme::DarkGreen => "dark green".to_string(),
            Theme::Dark => "dark".to_string(),
            Theme::LightBlue => "light blue".to_string(),
        }
    }

    /// Config-safe name for serialization (no spaces).
    pub fn config_name(&self) -> &str {
        match self {
            Theme::Light => "light",
            Theme::DarkGreen => "dark_green",
            Theme::Dark => "dark",
            Theme::LightBlue => "light_blue",
        }
    }

    /// Resolve a theme from its config name. Returns None if unrecognized.
    pub fn from_name(name: &str) -> Option<Theme> {
        match name {
            "light" => Some(Theme::Light),
            "dark_green" | "dark green" => Some(Theme::DarkGreen),
            "dark" => Some(Theme::Dark),
            "light_blue" | "light blue" => Some(Theme::LightBlue),
            _ => None,
        }
    }

    pub fn get(&self) -> ThemeValues {
        match self {
            Theme::Light => super::theme_light::theme_light(),
            Theme::DarkGreen => super::theme_dark_green::theme_dark_green(),
            Theme::Dark => super::theme_dark::theme_dark(),
            Theme::LightBlue => super::theme_light_blue::theme_light_blue(),
        }
    }
}

pub fn set_theme(ctx: &egui::Context, theme: Theme) {
    ctx.global_style_mut(|style| {
        style.spacing.item_spacing.y = 5.0;
        style.spacing.button_padding = egui::vec2(13.0, 4.0);
        style.spacing.interact_size.y = 20.0;
        style.spacing.menu_margin = egui::Margin::same(16);
        style.spacing.menu_spacing = 8.0;
    });

    let old = ctx.global_style().visuals.clone();
    let theme_values = theme.get();

    ctx.set_visuals(egui::Visuals {
        panel_fill: Color32::from(theme_values.panel_fill),
        extreme_bg_color: Color32::from(theme_values.extreme_bg_color),
        override_text_color: Some(Color32::from(theme_values.override_text_color)),
        dark_mode: theme_values.dark_mode,

        window_corner_radius: theme_values.window_corner_radius,
        window_shadow: theme_values.window_shadow,
        window_fill: Color32::from(theme_values.window_fill),
        window_stroke: theme_values.window_stroke,

        menu_corner_radius: CornerRadius::same(1),
        popup_shadow: Shadow::NONE,

        widgets: Widgets {
            noninteractive: WidgetVisuals {
                bg_fill: Color32::from(theme_values.widgets_noninteractive_bg_fill),
                weak_bg_fill: Color32::from(theme_values.widgets_noninteractive_weak_bg_fill),
                bg_stroke: Stroke::new(
                    theme_values.widgets_noninteractive_bg_stroke.0,
                    theme_values.widgets_noninteractive_bg_stroke.1,
                ),
                corner_radius: CornerRadius::same(
                    theme_values.widgets_noninteractive_rounding as u8,
                ),
                fg_stroke: Stroke::new(
                    theme_values.widgets_noninteractive_fg_stroke.0,
                    theme_values.widgets_noninteractive_fg_stroke.1,
                ),
                expansion: theme_values.widgets_noninteractive_expansion,
            },
            inactive: WidgetVisuals {
                bg_fill: Color32::from(theme_values.widgets_interactive_bg_fill),
                weak_bg_fill: Color32::from(theme_values.widgets_interactive_weak_bg_fill),
                bg_stroke: Stroke::new(
                    theme_values.widgets_interactive_bg_stroke.0,
                    theme_values.widgets_interactive_bg_stroke.1,
                ),
                corner_radius: CornerRadius::same(theme_values.widgets_interactive_rounding as u8),
                fg_stroke: Stroke::new(
                    theme_values.widgets_interactive_fg_stroke.0,
                    theme_values.widgets_interactive_fg_stroke.1,
                ),
                expansion: theme_values.widgets_interactive_expansion,
            },
            hovered: WidgetVisuals {
                bg_fill: Color32::from(theme_values.widgets_hovered_bg_fill),
                weak_bg_fill: Color32::from(theme_values.widgets_hovered_weak_bg_fill),
                bg_stroke: Stroke::new(
                    theme_values.widgets_hovered_bg_stroke.0,
                    theme_values.widgets_hovered_bg_stroke.1,
                ),
                corner_radius: CornerRadius::same(theme_values.widgets_hovered_rounding as u8),
                fg_stroke: Stroke::new(
                    theme_values.widgets_hovered_fg_stroke.0,
                    theme_values.widgets_hovered_fg_stroke.1,
                ),
                expansion: theme_values.widgets_hovered_expansion,
            },
            active: WidgetVisuals {
                bg_fill: Color32::from(theme_values.widgets_active_bg_fill),
                weak_bg_fill: Color32::from(theme_values.widgets_active_weak_bg_fill),
                bg_stroke: Stroke::new(
                    theme_values.widgets_active_bg_stroke.0,
                    theme_values.widgets_active_bg_stroke.1,
                ),
                corner_radius: CornerRadius::same(theme_values.widgets_active_rounding as u8),
                fg_stroke: Stroke::new(
                    theme_values.widgets_active_fg_stroke.0,
                    theme_values.widgets_active_fg_stroke.1,
                ),
                expansion: theme_values.widgets_active_expansion,
            },
            open: WidgetVisuals {
                bg_fill: Color32::from(theme_values.widgets_open_bg_fill),
                weak_bg_fill: Color32::from(theme_values.widgets_open_weak_bg_fill),
                bg_stroke: Stroke::new(
                    theme_values.widgets_open_bg_stroke.0,
                    theme_values.widgets_open_bg_stroke.1,
                ),
                corner_radius: CornerRadius::same(theme_values.widgets_open_rounding as u8),
                fg_stroke: Stroke::new(
                    theme_values.widgets_open_fg_stroke.0,
                    theme_values.widgets_open_fg_stroke.1,
                ),
                expansion: theme_values.widgets_open_expansion,
            },
        },
        ..old
    });
}

pub fn desaturate(color: Color32) -> Color32 {
    let mut hsva: Hsva = color.into();
    hsva.s = 0.0;
    let color: Color32 = hsva.into();
    color
}
