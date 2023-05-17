use core::panic;

use eframe::{
    egui::{self, Label},
    epaint::Rounding,
};
use image::imageops::FilterType;
use mangler::{
    input::Input,
    nodes::{node_settings::NodeSettings, operation::UiType},
    output::Output,
    value::{Value, ImageFormat},
};

pub struct NodeSettingsPanel {}

impl NodeSettingsPanel {
    pub fn new() -> NodeSettingsPanel {
        NodeSettingsPanel {}
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        node_settings: Option<&mut NodeSettings>,
        node_inputs: Option<&mut Vec<Input>>,
        node_outputs: Option<&Vec<Output>>,
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
                        ui.label(input.name.clone());
                        // todo: redo this
                        // each value type should only have one option
                        match input.value.clone() {
                            Value::Bool(a) => {
                                if input.connection.is_some() {
                                    ui.label(a.to_string());
                                } else {
                                    let mut x = a;
                                    if ui.add(egui::Checkbox::new(&mut x, "")).changed() {
                                        input.value = Value::Bool(x);
                                        response
                                            .input_indexes_that_changed
                                            .push(input_index);
                                    }
                                }
                            },
                            Value::Integer(a) => {
                                if input.connection.is_some() {
                                    ui.label(a.to_string());
                                } else {
                                    let mut x = a;
                                    if ui.add(egui::DragValue::new(&mut x)).changed() {
                                        input.value = Value::Integer(x);
                                        response
                                            .input_indexes_that_changed
                                            .push(input_index);
                                    }
                                }
                            },
                            Value::Decimal(a) => {
                                if input.connection.is_some() {
                                    ui.label(a.to_string());
                                } else {
                                    let mut x = a;
                                    if ui.add(egui::DragValue::new(&mut x)).changed() {
                                        input.value = Value::Decimal(x);
                                        response
                                            .input_indexes_that_changed
                                            .push(input_index);
                                    }
                                }
                            },
                            Value::String(a) => {
                                if input.connection.is_some() {
                                    ui.label(a);
                                } else {
                                    let mut x = a;
                                    if ui.text_edit_singleline(&mut x).changed() {
                                        input.value = Value::String(x);
                                        response.input_indexes_that_changed.push(input_index);
                                    }
                                }
                            },
                            Value::ImageRgba32F(_) => {},
                            Value::ImageRgb32F(_) => {},
                            Value::ImageRgba16(_) => {},
                            Value::ImageRgb16(_) => {},
                            Value::ImageGrayA16(_) => {},
                            Value::ImageGray16(_) => {},
                            Value::ImageRgba8(_) => {},
                            Value::ImageRgb8(_) => {},
                            Value::ImageGrayA8(_) => {},
                            Value::ImageGray8(_) => {},
                            Value::FilterType(a) => {
                                if input.connection.is_some() {
                                    ui.label(format!("{:?}", a));
                                } else {
                                    let mut x = a;
                                    egui::ComboBox::from_label("Filter Type").selected_text(format!("{:?}", x)).show_ui(ui, |ui| {
                                        if ui.selectable_value(&mut x, FilterType::Nearest, "Nearest Neighbor").changed() {
                                            input.value = Value::FilterType(x);
                                            response.input_indexes_that_changed.push(input_index);
                                        }
                                        if ui.selectable_value(&mut x, FilterType::Triangle, "Linear Filter (Triangle)").changed() {
                                            input.value = Value::FilterType(x);
                                            response.input_indexes_that_changed.push(input_index);
                                        }
                                        if ui.selectable_value(&mut x, FilterType::CatmullRom, "Cubic Filter ( CatmullRom)").changed() {
                                            input.value = Value::FilterType(x);
                                            response.input_indexes_that_changed.push(input_index);
                                        }
                                        if ui.selectable_value(&mut x, FilterType::Gaussian, "Gaussian Filter").changed() {
                                            input.value = Value::FilterType(x);
                                            response.input_indexes_that_changed.push(input_index);
                                        }
                                        if ui.selectable_value(&mut x, FilterType::Lanczos3, "Lanczos with window 3").changed() {
                                            input.value = Value::FilterType(x);
                                            response.input_indexes_that_changed.push(input_index);
                                        }
                                    });
                                }
                            },
                            Value::ImageFormat(a) => {
                                if input.connection.is_some() {
                                    ui.label(format!("{:?}", a));
                                } else {
                                    let mut x = a;
                                    egui::ComboBox::from_label("Image Format").selected_text(format!("{:?}", x)).show_ui(ui, |ui| {
                                        if ui.selectable_value(&mut x, ImageFormat::ImageGray16, "Grayscale 16 bit").changed() {
                                            input.value = Value::ImageFormat(x);
                                            response.input_indexes_that_changed.push(input_index);
                                        }
                                        if ui.selectable_value(&mut x, ImageFormat::ImageGray8, "Grayscale 8 bit").changed() {
                                            input.value = Value::ImageFormat(x);
                                            response.input_indexes_that_changed.push(input_index);
                                        }
                                        if ui.selectable_value(&mut x, ImageFormat::ImageGrayA16, "Grayscale with alpha 16 bit").changed() {
                                            input.value = Value::ImageFormat(x);
                                            response.input_indexes_that_changed.push(input_index);
                                        }
                                        if ui.selectable_value(&mut x, ImageFormat::ImageGrayA8, "Grayscale with alpha 8 bit").changed() {
                                            input.value = Value::ImageFormat(x);
                                            response.input_indexes_that_changed.push(input_index);
                                        }
                                        if ui.selectable_value(&mut x, ImageFormat::ImageRgb16, "RGB 16 bit").changed() {
                                            input.value = Value::ImageFormat(x);
                                            response.input_indexes_that_changed.push(input_index);
                                        }
                                        if ui.selectable_value(&mut x, ImageFormat::ImageRgb32F, "RGB 32 bit float").changed() {
                                            input.value = Value::ImageFormat(x);
                                            response.input_indexes_that_changed.push(input_index);
                                        }
                                        if ui.selectable_value(&mut x, ImageFormat::ImageRgb8, "RGB 8 bit").changed() {
                                            input.value = Value::ImageFormat(x);
                                            response.input_indexes_that_changed.push(input_index);
                                        }
                                        if ui.selectable_value(&mut x, ImageFormat::ImageRgba16, "RGBA 16 bit").changed() {
                                            input.value = Value::ImageFormat(x);
                                            response.input_indexes_that_changed.push(input_index);
                                        }
                                        if ui.selectable_value(&mut x, ImageFormat::ImageRgba32F, "RGBA 32 bit float").changed() {
                                            input.value = Value::ImageFormat(x);
                                            response.input_indexes_that_changed.push(input_index);
                                        }
                                        if ui.selectable_value(&mut x, ImageFormat::ImageRgba8, "RGBA 8 bit").changed() {
                                            input.value = Value::ImageFormat(x);
                                            response.input_indexes_that_changed.push(input_index);
                                        }
                                    });
                                }
                            },
                        }
                    });
                }
            }

            ui.add_space(20.0);
            ui.heading("Outputs");

            // outputs
            if let Some(outputs) = node_outputs {
                for output in outputs.iter() {
                    ui.horizontal(|ui| {
                        ui.add(Label::new(output.name.clone()));
                        match &output.value {
                            Value::Integer(v) => {
                                ui.add(Label::new(v.to_string()));
                            }
                            Value::Decimal(v) => {
                                ui.add(Label::new(v.to_string()));
                            }
                            Value::String(v) => {
                                ui.add(Label::new(v.to_string()));
                            }
                            // Value::ImageRgba32F(_) => todo!(),
                            // Value::ImageRgba8(_) => todo!(),
                            // Value::ImageGray8(_) => todo!(),
                            _ => {}
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
            input_indexes_that_changed: vec![],
        }
    }
}
