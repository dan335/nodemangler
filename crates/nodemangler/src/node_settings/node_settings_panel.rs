use std::{println, default, vec};

use eframe::{egui::{self, Label, style::Spacing, Margin}, epaint::Rounding};
use mangler::{input::Input, nodes::{operation::UiType, node_settings::NodeSettings}, value::Value, output::Output};

pub struct NodeSettingsPanel {
  
}

impl NodeSettingsPanel {
    pub fn new() -> NodeSettingsPanel {
        NodeSettingsPanel {

        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        node_settings: Option<&mut NodeSettings>,
        node_inputs: Option<&mut Vec<Input>>,
        node_outputs: Option<&Vec<Output>>
    ) -> NodeSettingsPanelResponse {
        let mut response = NodeSettingsPanelResponse::default();

        let mut name: String = "Right Panel".to_string();

        if let Some(settings) = node_settings {
            name = settings.name.clone();
        }

        // background color
        ui.painter().add(egui::Shape::rect_filled(
            ui.max_rect(),
            Rounding::none(),
            egui::Color32::from_gray(40),
        ));

        let left_top = ui.max_rect().left_top();
        let right_bottom = ui.max_rect().right_bottom();
        let padding = 10.0;

        let ui_rect = egui::Rect::from_two_pos(
            egui::Pos2::new(left_top.x + padding, left_top.y + padding),
            egui::Pos2::new(right_bottom.x - padding, right_bottom.y - padding),
        );
        
        ui.allocate_ui_at_rect(ui_rect, |ui| {

            // name of node
            ui.vertical_centered(|ui| {
                ui.heading(name);
            });

            ui.heading("Inputs");

            // show properties
            if let Some(inputs) = node_inputs {
                for (input_index, input) in inputs.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.add(Label::new(input.name.clone()));
                        if let Some(ui_type) = &input.ui_type {
                            match ui_type {
                                UiType::DragValue => {
                                    match input.value {
                                        Value::Integer(a) => {
                                            let mut x = a.clone();
                                            if ui.add(egui::DragValue::new(&mut x)).changed() {
                                                input.value = Value::Integer(x);
                                                response.input_indexes_that_changed.push(input_index);
                                            }
                                        },
                                        Value::Decimal(a) => {
                                            let mut x = a.clone();
                                            if ui.add(egui::DragValue::new(&mut x)).changed() {
                                                input.value = Value::Decimal(x);
                                                response.input_indexes_that_changed.push(input_index);
                                            }
                                        },
                                        Value::String(_) => todo!(),
                                    };
                                },
                                UiType::Checkbox => todo!(),
                                UiType::Slider => todo!(),
                                UiType::TextEdit => todo!(),
                            }
                        }
                    });
                    
                }
            }

            ui.add_space(20.0);
            ui.heading("Outputs");

            if let Some(outputs) = node_outputs {
                for output in outputs.iter() {
                    ui.horizontal(|ui| {
                        ui.add(Label::new(output.name.clone()));
                        match &output.value {
                            Value::Integer(v) => {
                                ui.add(Label::new(v.to_string()));
                            },
                            Value::Decimal(v) => {
                                ui.add(Label::new(v.to_string()));
                            },
                            Value::String(v) => {
                                ui.add(Label::new(v.to_string()));
                            },
                        }
                    });
                }
            }
        });

        response
    }
}


pub struct NodeSettingsPanelResponse {
    pub input_indexes_that_changed: Vec<usize>,
}

impl NodeSettingsPanelResponse {
    pub fn default() -> NodeSettingsPanelResponse {
        NodeSettingsPanelResponse {
            input_indexes_that_changed: vec![]
        }
    }
}