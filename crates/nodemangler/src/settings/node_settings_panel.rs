use eframe::egui::{self, Label};
use epaint::vec2;
use image::imageops::FilterType;
use mangler::{
    input::Input,
    value::{ImageFormat, Value},
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
                                        ImageFormat::ImageGray8,
                                        "Grayscale 8 bit",
                                    )
                                    .changed()
                                {
                                    let value = Value::ImageFormat(x);
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
                                        ImageFormat::ImageGrayA16,
                                        "Grayscale with alpha 16 bit",
                                    )
                                    .changed()
                                {
                                    let value = Value::ImageFormat(x);
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
                                        ImageFormat::ImageGrayA8,
                                        "Grayscale with alpha 8 bit",
                                    )
                                    .changed()
                                {
                                    let value = Value::ImageFormat(x);
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
                                    .selectable_value(&mut x, ImageFormat::ImageRgb16, "RGB 16 bit")
                                    .changed()
                                {
                                    let value = Value::ImageFormat(x);
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
                                        ImageFormat::ImageRgb32F,
                                        "RGB 32 bit float",
                                    )
                                    .changed()
                                {
                                    let value = Value::ImageFormat(x);
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
                                    .selectable_value(&mut x, ImageFormat::ImageRgb8, "RGB 8 bit")
                                    .changed()
                                {
                                    let value = Value::ImageFormat(x);
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
                                        ImageFormat::ImageRgba16,
                                        "RGBA 16 bit",
                                    )
                                    .changed()
                                {
                                    let value = Value::ImageFormat(x);
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
                                        ImageFormat::ImageRgba32F,
                                        "RGBA 32 bit float",
                                    )
                                    .changed()
                                {
                                    let value = Value::ImageFormat(x);
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
                                    .selectable_value(&mut x, ImageFormat::ImageRgba8, "RGBA 8 bit")
                                    .changed()
                                {
                                    let value = Value::ImageFormat(x);
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
                Value::UiButton(a) => {
                    if input.connection.is_some() {
                        ui.label(format!("{:?}", a));
                    } else if ui.add(egui::Button::new(input.name.clone())).clicked() {
                        change_value(
                            tx_change_node.clone(),
                            node.id.clone(),
                            input_index,
                            input,
                            Value::UiButton(true),
                        );
                    }
                }
                Value::DynamicImage{data:_, change_id:_} => {}
                Value::Path(a) => {
                    if input.connection.is_some() {
                        ui.label(a.into_os_string().into_string().unwrap());
                    } else {
                        // let mut x = a.into_os_string().into_string().unwrap();
                        // if ui.text_edit_singleline(&mut x).changed() {
                        //     let value = Value::Path(PathBuf::from(x));
                        //     change_value(
                        //         tx_change_node.clone(),
                        //         node.id.clone(),
                        //         input_index,
                        //         input,
                        //         value.clone(),
                        //     );
                        //     input.value = value;
                        // }

                        ui.allocate_ui(
                            vec2(ui.available_width() - 20.0, ui.available_height()),
                            |ui| {
                                ui.add_enabled_ui(false, |ui| {
                                    ui.text_edit_singleline(
                                        &mut a.into_os_string().into_string().unwrap(),
                                    )
                                });
                            },
                        );

                        if ui.button("🗀").clicked() {
                            if let Some(save_path) = rfd::FileDialog::new()
                                .add_filter("mangler", &["mangle"])
                                .pick_file()
                            {
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
