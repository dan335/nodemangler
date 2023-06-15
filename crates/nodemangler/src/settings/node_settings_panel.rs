use eframe::egui::{self, Label};
use epaint::vec2;
use image::imageops::FilterType;
use mangler::{
    input::{Input, InputSettings},
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

pub fn show(ui: &mut egui::Ui, node: &mut GraphNode, tx_change_node: Sender<ChangeNodeMessage>) {
    let name = node.settings.name.clone();
    ui.vertical_centered(|ui| {
        ui.heading(name);
    });

    ui.add_space(10.0);

    ui.heading("Inputs");
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
                            if ui.text_edit_multiline(&mut x).changed() {
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
                        egui::ComboBox::from_label("Image Format")
                            .selected_text(format!("{:?}", x))
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_value(
                                        &mut x,
                                        ColorFormat::ImageGray16,
                                        "Grayscale 16 bit",
                                    )
                                    .changed()
                                {
                                    let value = Value::ColorFormat(x);
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
                                        ColorFormat::ImageGray8,
                                        "Grayscale 8 bit",
                                    )
                                    .changed()
                                {
                                    let value = Value::ColorFormat(x);
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
                                        ColorFormat::ImageGrayA16,
                                        "Grayscale with alpha 16 bit",
                                    )
                                    .changed()
                                {
                                    let value = Value::ColorFormat(x);
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
                                        ColorFormat::ImageGrayA8,
                                        "Grayscale with alpha 8 bit",
                                    )
                                    .changed()
                                {
                                    let value = Value::ColorFormat(x);
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
                                    .selectable_value(&mut x, ColorFormat::ImageRgb16, "RGB 16 bit")
                                    .changed()
                                {
                                    let value = Value::ColorFormat(x);
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
                                        ColorFormat::ImageRgb32F,
                                        "RGB 32 bit float",
                                    )
                                    .changed()
                                {
                                    let value = Value::ColorFormat(x);
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
                                    .selectable_value(&mut x, ColorFormat::ImageRgb8, "RGB 8 bit")
                                    .changed()
                                {
                                    let value = Value::ColorFormat(x);
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
                                        ColorFormat::ImageRgba16,
                                        "RGBA 16 bit",
                                    )
                                    .changed()
                                {
                                    let value = Value::ColorFormat(x);
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
                                        ColorFormat::ImageRgba32F,
                                        "RGBA 32 bit float",
                                    )
                                    .changed()
                                {
                                    let value = Value::ColorFormat(x);
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
                                    .selectable_value(&mut x, ColorFormat::ImageRgba8, "RGBA 8 bit")
                                    .changed()
                                {
                                    let value = Value::ColorFormat(x);
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
                            println!("{:?}", input);
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
                Value::ImageFormat(image_format) => {

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
    ui.heading("Outputs");
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
}
