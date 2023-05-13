use eframe::{egui, epaint::Rounding};

pub struct ViewPanel {
  
}

impl ViewPanel {
    pub fn new() -> ViewPanel {
        ViewPanel {

        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.painter().add(egui::Shape::rect_filled(
            ui.max_rect(),
            Rounding::none(),
            egui::Color32::from_gray(30),
        ));
        ui.vertical_centered(|ui| {
            ui.heading("Top Panel");
        });
    }
}