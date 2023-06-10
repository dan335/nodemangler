use eframe::egui;
use epaint::Color32;

pub struct Theme {
    pub panel_fill: Color32,
    pub dark_mode: bool,
}

pub const DARK: Theme = Theme {
    panel_fill: Color32::RED,
    dark_mode: true,
};

pub const LIGHT: Theme = Theme {
    panel_fill: Color32::GREEN,
    dark_mode: false,
};

pub fn set_theme(ctx: &egui::Context, theme: Theme) {
    let old = ctx.style().visuals.clone();

    ctx.set_visuals(egui::Visuals {
        panel_fill: theme.panel_fill,
        dark_mode: theme.dark_mode,
        ..old
    });
}