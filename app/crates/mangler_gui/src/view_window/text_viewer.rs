use eframe::egui::{self, RichText};

pub struct TextViewer {

}

impl TextViewer {
    pub fn show(ui: &mut egui::Ui, value: String) {
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(RichText::new(format!("{}", value)).font(egui::FontId::proportional(20.0)));
        });
        
    }
}