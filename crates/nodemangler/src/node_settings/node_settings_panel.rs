use eframe::{
    egui::{self, Label},
    epaint::Rounding,
};
use image::imageops::FilterType;
use mangler::{
    input::Input,
    value::{ImageFormat, Value},
};
use tokio::sync::mpsc::Sender;

use crate::{graph::graph_node::GraphNode, SetNodeInputMessage};

pub struct NodeSettingsPanel {}

impl NodeSettingsPanel {
    pub fn new() -> NodeSettingsPanel {
        NodeSettingsPanel {}
    }

    fn change_value(
        &self,
        tx_input: Sender<SetNodeInputMessage>,
        node_id: String,
        input_index: usize,
        input: &mut Input,
        value: Value,
    ) {
        let set_node_input_message = SetNodeInputMessage {
            node_id,
            input_index,
            value: value.clone(),
        };

        match tx_input.try_send(set_node_input_message) {
            Ok(_) => {}
            Err(err) => {
                println!("Error sending SetNodeInputMessage: {:?}", err);
            }
        }

        input.set_value(value);
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        node_option: Option<&mut GraphNode>,
        tx_input: Sender<SetNodeInputMessage>,
    ) {
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
                for (input_index, input) in node.inputs.iter_mut().enumerate() {
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
                                        let value = Value::Bool(x);
                                        self.change_value(
                                            tx_input.clone(),
                                            node.id.clone(),
                                            input_index,
                                            input,
                                            value.clone(),
                                        );
                                        input.set_value(value);
                                    }
                                }
                            }
                            Value::Integer(a) => {
                                if input.connection.is_some() {
                                    ui.label(a.to_string());
                                } else {
                                    let mut x = a;
                                    if ui.add(egui::DragValue::new(&mut x)).changed() {
                                        let value = Value::Integer(x);
                                        self.change_value(
                                            tx_input.clone(),
                                            node.id.clone(),
                                            input_index,
                                            input,
                                            value.clone(),
                                        );
                                        input.set_value(value);
                                    }
                                }
                            }
                            Value::Decimal(a) => {
                                if input.connection.is_some() {
                                    ui.label(a.to_string());
                                } else {
                                    let mut x = a;
                                    if ui.add(egui::DragValue::new(&mut x)).changed() {
                                        let value = Value::Decimal(x);
                                        self.change_value(
                                            tx_input.clone(),
                                            node.id.clone(),
                                            input_index,
                                            input,
                                            value.clone(),
                                        );
                                        input.set_value(value);
                                    }
                                }
                            }
                            Value::String(a) => {
                                if input.connection.is_some() {
                                    ui.label(a);
                                } else {
                                    let mut x = a;
                                    if ui.text_edit_singleline(&mut x).changed() {
                                        let value = Value::String(x);
                                        self.change_value(
                                            tx_input.clone(),
                                            node.id.clone(),
                                            input_index,
                                            input,
                                            value.clone(),
                                        );
                                        input.set_value(value);
                                    }
                                }
                            }
                            Value::Rgba32FImage(_) => {}
                            Value::Rgb32FImage(_) => {}
                            Value::Rgba16Image(_) => {}
                            Value::Rgb16Image(_) => {}
                            Value::GrayAlpha16Image(_) => {}
                            Value::Gray16Image(_) => {}
                            Value::RgbaImage(_) => {}
                            Value::RgbImage(_) => {}
                            Value::GrayAlphaImage(_) => {}
                            Value::GrayImage(_) => {}
                            Value::FilterType(a) => {
                                if input.connection.is_some() {
                                    ui.label(format!("{:?}", a));
                                } else {
                                    let mut x = a;
                                    egui::ComboBox::from_label("Filter Type")
                                        .selected_text(format!("{:?}", x))
                                        .show_ui(ui, |ui| {
                                            if ui
                                                .selectable_value(
                                                    &mut x,
                                                    FilterType::Nearest,
                                                    "Nearest Neighbor",
                                                )
                                                .changed()
                                            {
                                                let value = Value::FilterType(x);
                                                self.change_value(
                                                    tx_input.clone(),
                                                    node.id.clone(),
                                                    input_index,
                                                    input,
                                                    value.clone(),
                                                );
                                                input.set_value(value);
                                            }
                                            if ui
                                                .selectable_value(
                                                    &mut x,
                                                    FilterType::Triangle,
                                                    "Linear Filter (Triangle)",
                                                )
                                                .changed()
                                            {
                                                let value = Value::FilterType(x);
                                                self.change_value(
                                                    tx_input.clone(),
                                                    node.id.clone(),
                                                    input_index,
                                                    input,
                                                    value.clone(),
                                                );
                                                input.set_value(value);
                                            }
                                            if ui
                                                .selectable_value(
                                                    &mut x,
                                                    FilterType::CatmullRom,
                                                    "Cubic Filter ( CatmullRom)",
                                                )
                                                .changed()
                                            {
                                                let value = Value::FilterType(x);
                                                self.change_value(
                                                    tx_input.clone(),
                                                    node.id.clone(),
                                                    input_index,
                                                    input,
                                                    value.clone(),
                                                );
                                                input.set_value(value);
                                            }
                                            if ui
                                                .selectable_value(
                                                    &mut x,
                                                    FilterType::Gaussian,
                                                    "Gaussian Filter",
                                                )
                                                .changed()
                                            {
                                                let value = Value::FilterType(x);
                                                self.change_value(
                                                    tx_input.clone(),
                                                    node.id.clone(),
                                                    input_index,
                                                    input,
                                                    value.clone(),
                                                );
                                                input.set_value(value);
                                            }
                                            if ui
                                                .selectable_value(
                                                    &mut x,
                                                    FilterType::Lanczos3,
                                                    "Lanczos with window 3",
                                                )
                                                .changed()
                                            {
                                                let value = Value::FilterType(x);
                                                self.change_value(
                                                    tx_input.clone(),
                                                    node.id.clone(),
                                                    input_index,
                                                    input,
                                                    value.clone(),
                                                );
                                                input.set_value(value);
                                            }
                                        });
                                }
                            }
                            Value::ImageFormat(a) => {
                                if input.connection.is_some() {
                                    ui.label(format!("{:?}", a));
                                } else {
                                    let mut x = a;
                                    egui::ComboBox::from_label("Image Format")
                                        .selected_text(format!("{:?}", x))
                                        .show_ui(ui, |ui| {
                                            if ui
                                                .selectable_value(
                                                    &mut x,
                                                    ImageFormat::ImageGray16,
                                                    "Grayscale 16 bit",
                                                )
                                                .changed()
                                            {
                                                let value = Value::ImageFormat(x);
                                                self.change_value(
                                                    tx_input.clone(),
                                                    node.id.clone(),
                                                    input_index,
                                                    input,
                                                    value.clone(),
                                                );
                                                input.set_value(value);
                                            }
                                            if ui
                                                .selectable_value(
                                                    &mut x,
                                                    ImageFormat::ImageGray8,
                                                    "Grayscale 8 bit",
                                                )
                                                .changed()
                                            {
                                                let value = Value::ImageFormat(x);
                                                self.change_value(
                                                    tx_input.clone(),
                                                    node.id.clone(),
                                                    input_index,
                                                    input,
                                                    value.clone(),
                                                );
                                                input.set_value(value);
                                            }
                                            if ui
                                                .selectable_value(
                                                    &mut x,
                                                    ImageFormat::ImageGrayA16,
                                                    "Grayscale with alpha 16 bit",
                                                )
                                                .changed()
                                            {
                                                let value = Value::ImageFormat(x);
                                                self.change_value(
                                                    tx_input.clone(),
                                                    node.id.clone(),
                                                    input_index,
                                                    input,
                                                    value.clone(),
                                                );
                                                input.set_value(value);
                                            }
                                            if ui
                                                .selectable_value(
                                                    &mut x,
                                                    ImageFormat::ImageGrayA8,
                                                    "Grayscale with alpha 8 bit",
                                                )
                                                .changed()
                                            {
                                                let value = Value::ImageFormat(x);
                                                self.change_value(
                                                    tx_input.clone(),
                                                    node.id.clone(),
                                                    input_index,
                                                    input,
                                                    value.clone(),
                                                );
                                                input.set_value(value);
                                            }
                                            if ui
                                                .selectable_value(
                                                    &mut x,
                                                    ImageFormat::ImageRgb16,
                                                    "RGB 16 bit",
                                                )
                                                .changed()
                                            {
                                                let value = Value::ImageFormat(x);
                                                self.change_value(
                                                    tx_input.clone(),
                                                    node.id.clone(),
                                                    input_index,
                                                    input,
                                                    value.clone(),
                                                );
                                                input.set_value(value);
                                            }
                                            if ui
                                                .selectable_value(
                                                    &mut x,
                                                    ImageFormat::ImageRgb32F,
                                                    "RGB 32 bit float",
                                                )
                                                .changed()
                                            {
                                                let value = Value::ImageFormat(x);
                                                self.change_value(
                                                    tx_input.clone(),
                                                    node.id.clone(),
                                                    input_index,
                                                    input,
                                                    value.clone(),
                                                );
                                                input.set_value(value);
                                            }
                                            if ui
                                                .selectable_value(
                                                    &mut x,
                                                    ImageFormat::ImageRgb8,
                                                    "RGB 8 bit",
                                                )
                                                .changed()
                                            {
                                                let value = Value::ImageFormat(x);
                                                self.change_value(
                                                    tx_input.clone(),
                                                    node.id.clone(),
                                                    input_index,
                                                    input,
                                                    value.clone(),
                                                );
                                                input.set_value(value);
                                            }
                                            if ui
                                                .selectable_value(
                                                    &mut x,
                                                    ImageFormat::ImageRgba16,
                                                    "RGBA 16 bit",
                                                )
                                                .changed()
                                            {
                                                let value = Value::ImageFormat(x);
                                                self.change_value(
                                                    tx_input.clone(),
                                                    node.id.clone(),
                                                    input_index,
                                                    input,
                                                    value.clone(),
                                                );
                                                input.set_value(value);
                                            }
                                            if ui
                                                .selectable_value(
                                                    &mut x,
                                                    ImageFormat::ImageRgba32F,
                                                    "RGBA 32 bit float",
                                                )
                                                .changed()
                                            {
                                                let value = Value::ImageFormat(x);
                                                self.change_value(
                                                    tx_input.clone(),
                                                    node.id.clone(),
                                                    input_index,
                                                    input,
                                                    value.clone(),
                                                );
                                                input.set_value(value);
                                            }
                                            if ui
                                                .selectable_value(
                                                    &mut x,
                                                    ImageFormat::ImageRgba8,
                                                    "RGBA 8 bit",
                                                )
                                                .changed()
                                            {
                                                let value = Value::ImageFormat(x);
                                                self.change_value(
                                                    tx_input.clone(),
                                                    node.id.clone(),
                                                    input_index,
                                                    input,
                                                    value.clone(),
                                                );
                                                input.set_value(value);
                                            }
                                        });
                                }
                            }
                            Value::UiButton(a) => {
                                if input.connection.is_some() {
                                    ui.label(format!("{:?}", a));
                                } else {
                                    if ui.add(egui::Button::new(input.name.clone())).clicked() {
                                        self.change_value(
                                            tx_input.clone(),
                                            node.id.clone(),
                                            input_index,
                                            input,
                                            Value::UiButton(true),
                                        );
                                    }
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
    }
}
