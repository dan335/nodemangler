use eframe::{egui::{self, RichText}, egui_glow::painter};
use epaint::Color32;
use mangler::color::Color;

pub struct ColorViewer {

}

impl ColorViewer {
    pub fn show(ui: &mut egui::Ui, value: Color) {
        let rgba = value.to_srgba_u8();
        let color: Color32 = Color32::from_rgba_unmultiplied(rgba.0, rgba.1, rgba.2, rgba.3);
        //ui.painter().rect_filled(ui.max_rect(), 0.0, color);

        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(RichText::new("         ").font(egui::FontId::proportional(20.0)).background_color(color));
        });
        
    }
}