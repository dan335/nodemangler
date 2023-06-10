use eframe::egui;
use epaint::Color32;
use epaint::HsvaGamma;

#[derive(Clone)]
pub struct Theme {
    pub panel_fill: HsvaGamma,
    pub grid_bg: HsvaGamma,
    pub dark_mode: bool,
}

pub const DARK: Theme = Theme {
    panel_fill: HsvaGamma {
        h: 0.0,
        s: 0.0,
        v: 0.1,
        a: 1.0,
    },
    grid_bg: HsvaGamma {
        h: 0.0,
        s: 0.0,
        v: 0.15,
        a: 1.0,
    },
    dark_mode: true,
};

pub const LIGHT: Theme = Theme {
    panel_fill: HsvaGamma {
        h: 0.0,
        s: 0.0,
        v: 0.95,
        a: 1.0,
    },
    grid_bg: HsvaGamma {
        h: 0.0,
        s: 0.0,
        v: 0.9,
        a: 1.0,
    },
    dark_mode: false,
};

pub fn set_theme(ctx: &egui::Context, theme: Theme) {
    let old = ctx.style().visuals.clone();

    ctx.set_visuals(egui::Visuals {
        panel_fill: Color32::from(theme.panel_fill),
        dark_mode: theme.dark_mode,
        ..old
    });
}
