use eframe::egui::{self, Label, Layout, RichText};
use epaint::{vec2, Color32};
use image::imageops::FilterType;
use mangler_core::{
    input::{Input, InputSettings},
    value::{ColorFormat, Value, TextHAlign, TextVAlign},
    ChangeNodeMessage, operations::images::noise::worley_distance::NoiseWorleyDistanceFunction, color::{color_spaces::ColorSpace, blend::BlendMode},
};
use egui_extras::{TableBuilder, Column};
use tokio::sync::mpsc::Sender;

use crate::{graph::graph_node::GraphNode, themes::theme::Theme};

fn change_value(
    tx_change_node: &Sender<ChangeNodeMessage>,
    node_id: &str,
    input_index: usize,
    input: &mut Input,
    value: Value,
) {
    let message = ChangeNodeMessage::SetInput {
        node_id: node_id.to_owned(),
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

pub fn show(ui: &mut egui::Ui, node: &mut GraphNode, tx_change_node: &Sender<ChangeNodeMessage>, theme: &Theme) -> NodeSettingsResponse {
    let mut node_settings_response = NodeSettingsResponse::new();

    ui.horizontal(|ui| {
        ui.heading(format!("{} settings", &node.settings.name));
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("X").clicked() {
                node_settings_response.deselect_node = true;
            }
        });
    });
    ui.label(egui::RichText::new(format!("{}", node.settings.description)).color(theme.get().text_faint));

    ui.add_space(12.0);

    // Enabled checkbox
    {
        let mut is_enabled = node.is_enabled;
        if ui.add(egui::Checkbox::new(&mut is_enabled, "Enabled")).changed() {
            let message = ChangeNodeMessage::SetEnabled {
                node_id: node.id.clone(),
                set_to: is_enabled,
            };
            match tx_change_node.try_send(message) {
                Ok(_) => {
                    node.is_enabled = is_enabled;
                }
                Err(err) => {
                    println!("Error sending SetEnabled: {:?}", err);
                }
            }
        }
    }

    ui.add_space(40.0);

    ui.heading("inputs");
    ui.add_space(12.0);

    // todo: try using ui.columns

    // Extract sibling image format before the mutable input loop so the
    // ColorFormat dropdown can grey out incompatible formats.
    let sibling_image_format = node.inputs.iter().find_map(|i| {
        if let Value::ImageType(fmt) = &i.value {
            Some(fmt.clone())
        } else {
            None
        }
    });

    // Auto-correct: if the current color format is incompatible with the
    // selected image format, switch to a sensible default.
    if let Some(ref img_fmt) = sibling_image_format {
        if let Some((cf_idx, _)) = node.inputs.iter().enumerate().find(|(_, i)| {
            if let Value::ColorFormat(cf) = &i.value {
                !cf.is_compatible_with_image_format(img_fmt)
            } else {
                false
            }
        }) {
            let new_cf = ColorFormat::default_for_image_format(img_fmt);
            let value = Value::ColorFormat(new_cf);
            change_value(tx_change_node, &node.id, cf_idx, &mut node.inputs[cf_idx], value);
        }
    }

    ui.push_id("inputs", |ui| {
        TableBuilder::new(ui).striped(true)
        .column(Column::auto().at_least(50.0).at_most(130.0).resizable(false))
        .column(Column::remainder().resizable(false))
        //.column(Column::exact(26.0).resizable(false))
        .header(30.0, |mut header| {
            header.col(|ui| {
                ui.label(RichText::new("name").color(theme.get().text_faint));
            });
            header.col(|ui| {
                ui.label(RichText::new("value").color(theme.get().text_faint));
            });
            // header.col(|ui| {
            //     ui.label("");
            // });
        })
        .body(|mut body| {
            for (input_index, input) in node.inputs.iter_mut().enumerate() {

                body.row(30.0, |mut row| {
                    row.col(|ui| {
                        ui.horizontal_centered(|ui| {
                            ui.label(&input.name);
                        });
                    });

                    row.col(|ui| {
                        ui.horizontal_centered(|ui| {
                            input_value(ui, input.value.clone(), input, input_index, &node.id, &tx_change_node, sibling_image_format);
                        });
                    });                        

                    // row.col(|ui| {
                    //     ui.horizontal_centered(|ui| {
                    //         let mut is_exposed = input.is_exposed;
                    //         if ui
                    //             .add(egui::Checkbox::new(&mut is_exposed, ""))
                    //             .changed()
                    //         {
                    //             let message = ChangeNodeMessage::SetExposeInput {
                    //                 node_id: node.id.clone(),
                    //                 input_index,
                    //                 set_to: is_exposed,
                    //             };
                
                    //             match tx_change_node.try_send(message) {
                    //                 Ok(_) => {
                    //                     input.is_exposed = is_exposed;
                    //                 }
                    //                 Err(err) => {
                    //                     println!("Error sending SetNodeInputMessage: {:?}", err);
                    //                 }
                    //             }
                    //         }
                    //     });
                    // });
                });
            }
        });
    });
    

    ui.add_space(40.0);
    ui.heading("outputs");
    ui.add_space(12.0);

    ui.push_id("outputs", |ui| {
        TableBuilder::new(ui).striped(true)
            .column(Column::auto().at_least(50.0).at_most(130.0).resizable(false))
            .column(Column::remainder().resizable(false))
            //.column(Column::exact(26.0).resizable(false))
            .header(30.0, |mut header| {
                header.col(|ui| {
                    ui.label(RichText::new("name").color(theme.get().text_faint));
                });
                header.col(|ui| {
                    ui.label(RichText::new("value").color(theme.get().text_faint));
                });
                // header.col(|ui| {
                //     ui.label("");
                // });
            })
            .body(|mut body| {
                for (_output_index, output) in node.outputs.iter_mut().enumerate() {
                    body.row(30.0, |mut row| {
                        row.col(|ui| {
                            ui.horizontal_centered(|ui| {
                                ui.label(&output.name);
                            });
                        });

                        row.col(|ui| {
                            ui.horizontal_centered(|ui| {
                                output_value(ui, &output.value);
                            });
                        });

                        // subgraph
                        // row.col(|ui| {
                        //     ui.horizontal_centered(|ui| {
                        //         let mut is_exposed = output.is_exposed;
                        //         if ui
                        //             .add(egui::Checkbox::new(&mut is_exposed, ""))
                        //             .changed()
                        //         {
                        //             let message = ChangeNodeMessage::SetExposeOutput {
                        //                 node_id: node.id.clone(),
                        //                 output_index,
                        //                 set_to: is_exposed,
                        //             };

                        //             match tx_change_node.try_send(message) {
                        //                 Ok(_) => {
                        //                     output.is_exposed = is_exposed;
                        //                 }
                        //                 Err(err) => {
                        //                     println!("Error sending SetNodeInputMessage: {:?}", err);
                        //                 }
                        //             }
                        //         }
                        //     });
                        // });
                    });
                }
            });
    });

    node_settings_response
}


fn output_value(ui: &mut egui::Ui,  value: &Value) {
    match &value {
        Value::Integer(v) => {
            ui.add(Label::new(v.to_string()));
        }
        Value::Decimal(v) => {
            ui.add(Label::new(format!("{:?}", v)));
        }
        Value::Text(v) => {
            ui.add(Label::new(v.to_string()));
        }
        Value::Color(v) => {
            let rgba = v.to_srgb_u8();
            let color = Color32::from_rgba_unmultiplied(rgba.0, rgba.1, rgba.2, rgba.3);
            ui.label(RichText::new("                        ").background_color(color));
        }
        _ => {}
    }
}


fn input_value(ui: &mut egui::Ui, value: Value, input: &mut Input, input_index: usize, node_id: &str, tx_change_node: &Sender<ChangeNodeMessage>, sibling_image_format: Option<image::ImageFormat>) {
    match value {
        Value::Bool(a) => {
            if input.connection.is_some() {
                ui.label(a.to_string());
            } else {
                let mut x = a;
                if ui.add(egui::Checkbox::new(&mut x, "")).changed() {
                    let value = Value::Bool(x);
                    change_value(
                        tx_change_node,
                        node_id,
                        input_index,
                        input,
                        value,
                    );
                }
            }
        }
        Value::Integer(a) => {
            if input.connection.is_some() {
                ui.label(a.to_string());
            } else {
                let mut x = a;

                if let Some(input_type) = &input.settings {
                    match input_type {
                        InputSettings::DragValue { clamp, speed: _ } => {
                            let mut drag = egui::DragValue::new(&mut x);

                            drag = if let Some(clamp) = clamp {
                                drag.range(clamp.0..=clamp.1)
                            } else {
                                drag
                            };

                            if ui.add(drag).changed() {
                                let value = Value::Integer(x);
                                change_value(
                                    tx_change_node,
                                    node_id,
                                    input_index,
                                    input,
                                    value,
                                );
                            }
                        },
                        InputSettings::Slider { range, step_by: _, clamp_to_range } => {
                            if ui.add(egui::Slider::new(&mut x, range.0 as i32..=range.1 as i32).clamping(if *clamp_to_range { egui::SliderClamping::Always } else { egui::SliderClamping::Never })).changed() {
                                let value = Value::Integer(x);
                                change_value(
                                    tx_change_node,
                                    node_id,
                                    input_index,
                                    input,
                                    value,
                                );
                            }
                        },
                        _ => {}
                    }
                }
            }
        }
        Value::Decimal(a) => {
            if input.connection.is_some() {
                ui.label(format!("{:?}", a));
            } else {
                let mut x: f32 = a;

                if let Some(input_type) = &input.settings {
                    match input_type {
                        InputSettings::DragValue { speed, clamp } => {
                            let mut drag = egui::DragValue::new(&mut x);

                            drag = if let Some(speed) = *speed {
                                drag.speed(speed)
                            } else {
                                drag
                            };

                            drag = if let Some(clamp) = clamp {
                                drag.range(clamp.0..=clamp.1)
                            } else {
                                drag
                            };

                            if ui.add(drag).changed() {
                                let value = Value::Decimal(x);
                                change_value(
                                    tx_change_node,
                                    node_id,
                                    input_index,
                                    input,
                                    value,
                                );
                            }
                        },
                        InputSettings::Slider { range, step_by: _, clamp_to_range } => {
                            if ui.add(egui::Slider::new(&mut x, range.0..=range.1).clamping(if *clamp_to_range { egui::SliderClamping::Always } else { egui::SliderClamping::Never })).changed() {
                                let value = Value::Decimal(x);
                                change_value(
                                    tx_change_node,
                                    node_id,
                                    input_index,
                                    input,
                                    value,
                                );
                            }
                        },
                        _ => {}
                    }
                }

                
            }
        }
        Value::Text(a) => {
            if input.connection.is_some() {
                ui.label(a);
            } else {
                let mut x = a;
                ui.allocate_ui(egui::Vec2::new(ui.available_width() - 70.0, 16.0), |ui| {
                    let settings = input.settings.clone();
                    let widget = match settings {
                        Some(InputSettings::SingleLineText) => ui.text_edit_singleline(&mut x),
                        _ => ui.text_edit_multiline(&mut x),
                    };
                    if widget.changed() {
                        change_value(tx_change_node, node_id, input_index, input, Value::Text(x));
                    }
                });
            }
        }
        Value::Color(a) => {
            if input.connection.is_some() {
                let rgba = a.to_srgb_u8();
                let color = Color32::from_rgba_unmultiplied(rgba.0, rgba.1, rgba.2, rgba.3);
                ui.label(RichText::new("                        ").background_color(color));
            } else {
                let rgba = a.to_srgb_u8();
                let mut x = [rgba.0, rgba.1, rgba.2, rgba.3];
                if ui.color_edit_button_srgba_unmultiplied(&mut x).changed() {
                    let value = Value::Color(mangler_core::color::Color::from_srgb_u8(x[0], x[1], x[2], x[3]));
                    change_value(
                        tx_change_node,
                        node_id,
                        input_index,
                        input,
                        value,
                    );
                }
            }
        },
        Value::FilterType(a) => {
            if input.connection.is_some() {
                ui.label(format!("{:?}", a));
            } else {
                let mut x = a;
                egui::ComboBox::from_id_salt("Filter Type")
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
                                tx_change_node,
                                node_id,
                                input_index,
                                input,
                                value,
                            );
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
                                tx_change_node,
                                node_id,
                                input_index,
                                input,
                                value,
                            );
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
                                tx_change_node,
                                node_id,
                                input_index,
                                input,
                                value,
                            );
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
                                tx_change_node,
                                node_id,
                                input_index,
                                input,
                                value,
                            );
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
                                tx_change_node,
                                node_id,
                                input_index,
                                input,
                                value,
                            );
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
                            // Grey out color formats that are incompatible with the
                            // selected image format (if one exists on this node).
                            let compatible = sibling_image_format
                                .as_ref()
                                .map(|fmt| color_format.is_compatible_with_image_format(fmt))
                                .unwrap_or(true);
                            ui.add_enabled_ui(compatible, |ui| {
                                if ui.selectable_value(&mut x, color_format.clone(), format!("{:?}", color_format)).changed() {
                                    let value = Value::ColorFormat(color_format.clone());
                                    change_value(tx_change_node, node_id, input_index, input, value);
                                }
                            });
                        }
                    });
            }
        }
        Value::Trigger => {
            if input.connection.is_some() {
                ui.label(format!("trigger"));
            } else if ui.add(egui::Button::new(input.name.clone())).clicked() {
                change_value(
                    tx_change_node,
                    node_id,
                    input_index,
                    input,
                    Value::Trigger,
                );
            }
        }
        Value::DynamicImage{data:_, change_id:_} => {

        }
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
                    if let Some(InputSettings::Path {
                        extension_filter,
                        set_directory: _,
                        set_file_name: _,
                        set_title,
                        file_dialog_type
                    }) = input.settings.clone() {

                        let mut extensions: Vec<&str> = Vec::new();
                        for s in &extension_filter {
                            extensions.push(s.as_str());
                        }

                        let title = set_title.unwrap_or("file".to_string());

                        let file_dialog = rfd::FileDialog::new().add_filter(&title, &extensions);

                        if let Some(save_path) = match file_dialog_type {
                            mangler_core::input::FileDialogType::PickFile => file_dialog.pick_file(),
                            mangler_core::input::FileDialogType::PickFolder => file_dialog.pick_folder(),
                            mangler_core::input::FileDialogType::SaveFile => file_dialog.save_file(),
                        } {
                            let value = Value::Path(save_path);
                            change_value(
                                tx_change_node,
                                node_id,
                                input_index,
                                input,
                                value,
                            );
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
                egui::ComboBox::from_label("image format")
                    .selected_text(format!("{:?}", x))
                    .show_ui(ui, |ui| {
                        for image_type in mangler_core::value::ImageType::types().iter() {
                            if ui.selectable_value(&mut x, image_type.format(), image_type.format().extensions_str()[0].to_string()).changed() {
                                let value = Value::ImageType(image_type.format());
                                change_value(tx_change_node, node_id, input_index, input, value);
                            }
                        }
                    });
            }
        },
        Value::NoiseWorleyDistanceFunction(a) => {
            if input.connection.is_some() {
                ui.label(format!("{:?}", a));
            } else {
                let mut x = a;
                egui::ComboBox::from_label("distance function")
                    .selected_text(format!("{:?}", x))
                    .show_ui(ui, |ui| {
                        for distance_function in NoiseWorleyDistanceFunction::types().iter() {
                            if ui.selectable_value(&mut x, distance_function.clone(), format!("{:?}", distance_function)).changed() {
                                let value = Value::NoiseWorleyDistanceFunction(distance_function.clone());
                                change_value(tx_change_node, node_id, input_index, input, value);
                            }
                        }
                    });
            }
        },
        Value::ColorSpace(a) => if input.connection.is_some() {
            ui.label(format!("{:?}", a));
        } else {
            let mut x = a;
            egui::ComboBox::from_label("color space")
                .selected_text(format!("{:?}", x))
                .show_ui(ui, |ui| {
                    for color_space in ColorSpace::types().iter() {
                        if ui.selectable_value(&mut x, color_space.clone(), format!("{:?}", color_space)).changed() {
                            let value = Value::ColorSpace(color_space.clone());
                            change_value(tx_change_node, node_id, input_index, input, value);
                        }
                    }
                });
        }
        Value::BlendMode(a) => if input.connection.is_some() {
            ui.label(format!("{:?}", a));
        } else {
            let mut x = a;
            egui::ComboBox::from_label("blend mode")
                .selected_text(format!("{:?}", x))
                .show_ui(ui, |ui| {
                    for blend_mode in BlendMode::types().iter() {
                        if ui.selectable_value(&mut x, blend_mode.clone(), format!("{:?}", blend_mode)).changed() {
                            let value = Value::BlendMode(blend_mode.clone());
                            change_value(tx_change_node, node_id, input_index, input, value);
                        }
                    }
                });
        }
        Value::TextHAlign(a) => if input.connection.is_some() {
            ui.label(format!("{:?}", a));
        } else {
            let mut x = a;
            egui::ComboBox::from_label("h align")
                .selected_text(format!("{:?}", x))
                .show_ui(ui, |ui| {
                    for variant in TextHAlign::types().iter() {
                        if ui.selectable_value(&mut x, *variant, format!("{:?}", variant)).changed() {
                            change_value(tx_change_node, node_id, input_index, input, Value::TextHAlign(*variant));
                        }
                    }
                });
        }
        Value::TextVAlign(a) => if input.connection.is_some() {
            ui.label(format!("{:?}", a));
        } else {
            let mut x = a;
            egui::ComboBox::from_label("v align")
                .selected_text(format!("{:?}", x))
                .show_ui(ui, |ui| {
                    for variant in TextVAlign::types().iter() {
                        if ui.selectable_value(&mut x, *variant, format!("{:?}", variant)).changed() {
                            change_value(tx_change_node, node_id, input_index, input, Value::TextVAlign(*variant));
                        }
                    }
                });
        }
    }
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