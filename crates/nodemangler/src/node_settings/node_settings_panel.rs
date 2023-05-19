use eframe::{
    egui::{self, Label},
    epaint::Rounding,
};
use image::imageops::FilterType;
use mangler::{
    nodes::{node::Node},
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
        node_option: Option<&mut Node>,
        // node_settings: Option<&mut NodeSettings>,
        // node_inputs: Option<&mut Vec<Input>>,
        // node_outputs: Option<&Vec<Output>>,
    ) -> NodeSettingsPanelResponse {
        let mut response = NodeSettingsPanelResponse::default();

        // background
        ui.painter().add(egui::Shape::rect_filled(
            ui.max_rect(),
            Rounding::none(),
            egui::Color32::from_gray(40),
        ));

        let left_top = ui.max_rect().left_top();
        let right_bottom = ui.max_rect().right_bottom();
        let padding = 10.0;

        // create rect for content
        let ui_rect = egui::Rect::from_two_pos(
            egui::Pos2::new(left_top.x + padding, left_top.y + padding),
            egui::Pos2::new(right_bottom.x - padding, right_bottom.y - padding),
        );

        ui.allocate_ui_at_rect(ui_rect, |ui| {

            if let Some(node) = node_option {
                let name = node.settings.name.clone();
                ui.vertical_centered(|ui| {
                    ui.heading(name);
                });

                ui.heading("Inputs");

                // show properties
                for (_, input) in node.inputs.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(input.name.clone());
                        // todo: redo this
                        // each value type should only have one option
                        match input.get_value().clone() {
                            Value::Bool(a) => {
                                if input.connection.is_some() {
                                    ui.label(a.to_string());
                                } else {
                                    let mut x = a;
                                    if ui.add(egui::Checkbox::new(&mut x, "")).changed() {
                                        input.set_value(Value::Bool(x));
                                        response.has_node_changed = true;
                                    }
                                }
                            },
                            Value::Integer(a) => {
                                if input.connection.is_some() {
                                    ui.label(a.to_string());
                                } else {
                                    let mut x = a;
                                    if ui.add(egui::DragValue::new(&mut x)).changed() {
                                        input.set_value(Value::Integer(x));
                                        response.has_node_changed = true;
                                    }
                                }
                            },
                            Value::Decimal(a) => {
                                if input.connection.is_some() {
                                    ui.label(a.to_string());
                                } else {
                                    let mut x = a;
                                    if ui.add(egui::DragValue::new(&mut x)).changed() {
                                        input.set_value(Value::Decimal(x));
                                        response.has_node_changed = true;
                                    }
                                }
                            },
                            Value::String(a) => {
                                if input.connection.is_some() {
                                    ui.label(a);
                                } else {
                                    let mut x = a;
                                    if ui.text_edit_singleline(&mut x).changed() {
                                        input.set_value(Value::String(x));
                                        response.has_node_changed = true;
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
                                            input.set_value(Value::FilterType(x));
                                            response.has_node_changed = true;
                                        }
                                        if ui.selectable_value(&mut x, FilterType::Triangle, "Linear Filter (Triangle)").changed() {
                                            input.set_value(Value::FilterType(x));
                                            response.has_node_changed = true;
                                        }
                                        if ui.selectable_value(&mut x, FilterType::CatmullRom, "Cubic Filter ( CatmullRom)").changed() {
                                            input.set_value(Value::FilterType(x));
                                            response.has_node_changed = true;
                                        }
                                        if ui.selectable_value(&mut x, FilterType::Gaussian, "Gaussian Filter").changed() {
                                            input.set_value(Value::FilterType(x));
                                            response.has_node_changed = true;
                                        }
                                        if ui.selectable_value(&mut x, FilterType::Lanczos3, "Lanczos with window 3").changed() {
                                            input.set_value(Value::FilterType(x));
                                            response.has_node_changed = true;
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
                                            input.set_value(Value::ImageFormat(x));
                                            response.has_node_changed = true;
                                        }
                                        if ui.selectable_value(&mut x, ImageFormat::ImageGray8, "Grayscale 8 bit").changed() {
                                            input.set_value(Value::ImageFormat(x));
                                            response.has_node_changed = true;
                                        }
                                        if ui.selectable_value(&mut x, ImageFormat::ImageGrayA16, "Grayscale with alpha 16 bit").changed() {
                                            input.set_value(Value::ImageFormat(x));
                                            response.has_node_changed = true;
                                        }
                                        if ui.selectable_value(&mut x, ImageFormat::ImageGrayA8, "Grayscale with alpha 8 bit").changed() {
                                            input.set_value(Value::ImageFormat(x));
                                            response.has_node_changed = true;
                                        }
                                        if ui.selectable_value(&mut x, ImageFormat::ImageRgb16, "RGB 16 bit").changed() {
                                            input.set_value(Value::ImageFormat(x));
                                            response.has_node_changed = true;
                                        }
                                        if ui.selectable_value(&mut x, ImageFormat::ImageRgb32F, "RGB 32 bit float").changed() {
                                            input.set_value(Value::ImageFormat(x));
                                            response.has_node_changed = true;
                                        }
                                        if ui.selectable_value(&mut x, ImageFormat::ImageRgb8, "RGB 8 bit").changed() {
                                            input.set_value(Value::ImageFormat(x));
                                            response.has_node_changed = true;
                                        }
                                        if ui.selectable_value(&mut x, ImageFormat::ImageRgba16, "RGBA 16 bit").changed() {
                                            input.set_value(Value::ImageFormat(x));
                                            response.has_node_changed = true;
                                        }
                                        if ui.selectable_value(&mut x, ImageFormat::ImageRgba32F, "RGBA 32 bit float").changed() {
                                            input.set_value(Value::ImageFormat(x));
                                            response.has_node_changed = true;
                                        }
                                        if ui.selectable_value(&mut x, ImageFormat::ImageRgba8, "RGBA 8 bit").changed() {
                                            input.set_value(Value::ImageFormat(x));
                                            response.has_node_changed = true;
                                        }
                                    });
                                }
                            },
                        }
                    });
                }

                ui.add_space(20.0);
                ui.heading("Outputs");

                // outputs
                for output in node.outputs.iter() {
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
            } else {
                let name = "Graph Settings".to_string();
                ui.vertical_centered(|ui| {
                    ui.heading(name);
                });
            }
        });

        response
    }
}

pub struct NodeSettingsPanelResponse {
    pub has_node_changed: bool,
}

impl NodeSettingsPanelResponse {
    pub fn default() -> NodeSettingsPanelResponse {
        NodeSettingsPanelResponse {
            has_node_changed: false,
        }
    }
}
