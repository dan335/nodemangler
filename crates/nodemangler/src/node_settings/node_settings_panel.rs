use eframe::{egui, epaint::Rounding};
use mangler::{input::Input, nodes::{operation::UiType, node_settings::NodeSettings}};

pub struct NodeSettingsPanel {
  
}

impl NodeSettingsPanel {
    pub fn new() -> NodeSettingsPanel {
        NodeSettingsPanel {

        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, node_settings: Option<&mut NodeSettings>, node_inputs: Option<&mut Vec<Input>>) {
        let mut name: String = "Right Panel".to_string();

        if let Some(settings) = node_settings {
            name = settings.name.clone();
        }

        ui.painter().add(egui::Shape::rect_filled(
            ui.max_rect(),
            Rounding::none(),
            egui::Color32::from_gray(40),
        ));
        ui.vertical_centered(|ui| {
            ui.heading(name);
        });

        if let Some(inputs) = node_inputs {
            for input in inputs.iter_mut() {
                if let Some(ui_type) = &input.ui_type {
                    match ui_type {
                        UiType::DragValue => {
                            //ui.add(egui::DragValue::new(&mut input.value));
                        },
                        UiType::Checkbox => todo!(),
                        UiType::Slider => todo!(),
                        UiType::TextEdit => todo!(),
                    }
                }
            }
        }
    }
}