use eframe::egui::{self, Label, Layout};
use epaint::vec2;
use image::imageops::FilterType;
use mangler::{
    input::{Input, InputSettings, TextInputType},
    value::{ColorFormat, Value},
    ChangeNodeMessage,
};
use tokio::sync::mpsc::Sender;

use crate::graph::graph_node::GraphNode;

fn change_value(
    tx_change_node: Sender<ChangeNodeMessage>,
    node_id: String,
    input_index: usize,
    input: &mut Input,
    value: Value,
) {
    let message = ChangeNodeMessage::SetInput {
        node_id,
        input_index,
        value: value.clone(),
    };

    match tx_change_node.try_send(message) {
        Ok(_) => {}
        Err(err) => {
            println!("Error sending SetNodeInputMessage: {:?}", err);
        }
    }

    input.value = value;
}

pub fn show(ui: &mut egui::Ui, node: &mut GraphNode, tx_change_node: Sender<ChangeNodeMessage>) -> NodeSettingsResponse {
    let mut node_settings_response = NodeSettingsResponse::new();

    let name = node.settings.name.clone();

    ui.horizontal(|ui| {
        ui.heading(format!("{} settings", name));
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("X").clicked() {
                node_settings_response.deselect_node = true;
            }
        });
    });
    

    ui.add_space(20.0);

    ui.heading("inputs");
    ui.add_space(8.0);

    // todo: try using ui.columns

    // show properties
    for (input_index, input) in node.inputs.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(format!("{}      ", input.name.clone()));
            // todo: redo this
            // each value type should only have one option
            match input.value.clone() {
                Value::Bool(a) => {
                    if input.connection.is_some() {
                        ui.label(a.to_string());
                    } else {
                        let mut x = a;
                        if ui.add(egui::Checkbox::new(&mut x, "")).changed() {
                            let value = Value::Bool(x);
                            change_value(
                                tx_change_node.clone(),
                                node.id.clone(),
                                input_index,
                                input,
                                value.clone(),
                            );
                            input.value = value;
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
                            change_value(
                                tx_change_node.clone(),
                                node.id.clone(),
                                input_index,
                                input,
                                value.clone(),
                            );
                            input.value = value;
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
                            change_value(
                                tx_change_node.clone(),
                                node.id.clone(),
                                input_index,
                                input,
                                value.clone(),
                            );
                            input.value = value;
                        }
                    }
                }
                Value::String(a) => {
                    if input.connection.is_some() {
                        ui.label(a);
                    } else {
                        let mut x = a;
                        ui.allocate_ui(egui::Vec2::new(ui.available_width() - 70.0, 16.0), |ui| {
                            let settings = input.settings.clone();
                            if let InputSettings::String(text_input_type) = settings {
                                let text_edit = match text_input_type {
                                    TextInputType::SingleLine => {
                                        ui.text_edit_singleline(&mut x)
                                    },
                                    TextInputType::MultiLine => {
                                        ui.text_edit_multiline(&mut x)
                                    },
                                };

                                if text_edit.changed() {
                                    let value = Value::String(x);
                                    change_value(
                                        tx_change_node.clone(),
                                        node.id.clone(),
                                        input_index,
                                        input,
                                        value.clone(),
                                    );
                                    input.value = value;
                                }
                            }
                        });
                        
                    }
                }
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
                                    change_value(
                                        tx_change_node.clone(),
                                        node.id.clone(),
                                        input_index,
                                        input,
                                        value.clone(),
                                    );
                                    input.value = value;
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
                                    change_value(
                                        tx_change_node.clone(),
                                        node.id.clone(),
                                        input_index,
                                        input,
                                        value.clone(),
                                    );
                                    input.value = value;
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
                                    change_value(
                                        tx_change_node.clone(),
                                        node.id.clone(),
                                        input_index,
                                        input,
                                        value.clone(),
                                    );
                                    input.value = value;
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
                                    change_value(
                                        tx_change_node.clone(),
                                        node.id.clone(),
                                        input_index,
                                        input,
                                        value.clone(),
                                    );
                                    input.value = value;
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
                                    change_value(
                                        tx_change_node.clone(),
                                        node.id.clone(),
                                        input_index,
                                        input,
                                        value.clone(),
                                    );
                                    input.value = value;
                                }
                            });
                    }
                }
                Value::ColorFormat(a) => {
                    if input.connection.is_some() {
                        ui.label(format!("{:?}", a));
                    } else {
                        let mut x = a;
                        egui::ComboBox::from_label("Color Format")
                            .selected_text(format!("{:?}", x))
                            .show_ui(ui, |ui| {
                                for color_format in ColorFormat::types().iter() {
                                    if ui.selectable_value(&mut x, color_format.clone(), format!("{:?}", color_format)).changed() {
                                        let value = Value::ColorFormat(color_format.clone());
                                        change_value(tx_change_node.clone(), node.id.clone(), input_index, input, value.clone());
                                        input.value = value;
                                    }
                                }
                            });
                    }
                }
                Value::Trigger => {
                    if input.connection.is_some() {
                        ui.label(format!("trigger"));
                    } else if ui.add(egui::Button::new(input.name.clone())).clicked() {
                        change_value(
                            tx_change_node.clone(),
                            node.id.clone(),
                            input_index,
                            input,
                            Value::Trigger,
                        );
                    }
                }
                Value::DynamicImage{data:_, change_id:_} => {}
                Value::Path(path) => {
                    if input.connection.is_some() {
                        ui.label(path.into_os_string().into_string().unwrap());
                    } else {
                        ui.allocate_ui(
                            vec2(ui.available_width() - 20.0, ui.available_height()),
                            |ui| {
                                ui.add_enabled_ui(false, |ui| {
                                    ui.text_edit_singleline(
                                        &mut path.into_os_string().into_string().unwrap(),
                                    )
                                });
                            },
                        );

                        if ui.button("🗀").clicked() {
                            if let InputSettings::Path {
                                extension_filter,
                                set_directory,
                                set_file_name,
                                set_title,
                                file_dialog_type
                            } = input.settings.clone() {

                                let mut extensions: Vec<&str> = Vec::new();
                                for s in &extension_filter {
                                    extensions.push(s.as_str());
                                }

                                let title = set_title.unwrap_or("file".to_string());

                                let file_dialog = rfd::FileDialog::new().add_filter(&title, &extensions);

                                if let Some(save_path) = match file_dialog_type {
                                    mangler::input::FileDialogType::PickFile => file_dialog.pick_file(),
                                    mangler::input::FileDialogType::PickFolder => file_dialog.pick_folder(),
                                    mangler::input::FileDialogType::SaveFile => file_dialog.save_file(),
                                } {
                                    let value = Value::Path(save_path);
                                    change_value(
                                        tx_change_node.clone(),
                                        node.id.clone(),
                                        input_index,
                                        input,
                                        value.clone(),
                                    );
                                    input.value = value;
                                }
                            }
                        }
                    }
                }
                Value::ImageType(a) => {
                    if input.connection.is_some() {
                        ui.label(format!("{:?}", a));
                    } else {
                        let mut x = a;
                        egui::ComboBox::from_label("Image Format")
                            .selected_text(format!("{:?}", x))
                            .show_ui(ui, |ui| {
                                for image_type in mangler::value::ImageType::types().iter() {
                                    if ui.selectable_value(&mut x, image_type.format(), image_type.format().extensions_str()[0].to_string()).changed() {
                                        let value = Value::ImageType(image_type.format());
                                        change_value(tx_change_node.clone(), node.id.clone(), input_index, input, value.clone());
                                        input.value = value;
                                    }
                                }
                            });
                    }
                },
            }

            // exposed checkbox
            ui.add_space(12.0);
            let mut is_exposed = input.is_exposed;
            if ui
                .add(egui::Checkbox::new(&mut is_exposed, "   expose"))
                .changed()
            {
                let message = ChangeNodeMessage::SetExposeInput {
                    node_id: node.id.clone(),
                    input_index,
                    set_to: is_exposed,
                };

                match tx_change_node.try_send(message) {
                    Ok(_) => {
                        input.is_exposed = is_exposed;
                    }
                    Err(err) => {
                        println!("Error sending SetNodeInputMessage: {:?}", err);
                    }
                }
            }
        });
        ui.add_space(6.0);
    }

    ui.add_space(36.0);
    ui.heading("outputs");
    ui.add_space(8.0);

    // outputs
    for (output_index, output) in node.outputs.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(format!("{}      ", output.name.clone()));
            //ui.add(Label::new(output.name.clone()));
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
                _ => {}
            }

            // exposed checkbox
            ui.add_space(12.0);
            let mut is_exposed = output.is_exposed;
            if ui
                .add(egui::Checkbox::new(&mut is_exposed, "   expose"))
                .changed()
            {
                let message = ChangeNodeMessage::SetExposeOutput {
                    node_id: node.id.clone(),
                    output_index,
                    set_to: is_exposed,
                };

                match tx_change_node.try_send(message) {
                    Ok(_) => {
                        output.is_exposed = is_exposed;
                    }
                    Err(err) => {
                        println!("Error sending SetNodeInputMessage: {:?}", err);
                    }
                }
            }
        });
    }

    node_settings_response
}


pub struct NodeSettingsResponse {
    pub deselect_node: bool,
}

impl NodeSettingsResponse {
    pub fn new() -> Self {
        Self {
            deselect_node: false,
        }
    }
}