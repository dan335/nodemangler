use eframe::egui::{self, Label, Layout, RichText, TextEdit};
use epaint::{vec2, Color32};
use image::imageops::FilterType;
use mangler_core::{
    input::{Input, InputSettings},
    value::{ColorFormat, Value, TextHAlign, TextVAlign},
    ChangeNodeMessage, operations::images::noise::worley_distance::NoiseWorleyDistanceFunction, color::{color_spaces::ColorSpace, blend::BlendMode},
};
use egui_extras::{TableBuilder, Column};
use tokio::sync::mpsc::Sender;
use crate::{graph::graph_node::GraphNode, settings::histogram_widget, themes::theme::Theme};

/// Send a value change message to the engine and update the local input.
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

/// Draw a solid color rectangle as a swatch preview.
fn show_color_swatch(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(40.0, 18.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, 2.0, color);
    }
}

/// Generic ComboBox for any enum type that has a list of variants.
/// Handles the connected (read-only label) vs disconnected (interactive dropdown) pattern.
/// Uses `from_id_salt` with the input index appended to avoid ID collisions when
/// multiple nodes with the same enum type are visible.
fn show_enum_combo<T: Clone + PartialEq>(
    ui: &mut egui::Ui,
    label: &str,
    current: T,
    variants: &[T],
    display_name: impl Fn(&T) -> String,
    input: &mut Input,
    input_index: usize,
    node_id: &str,
    tx_change_node: &Sender<ChangeNodeMessage>,
    to_value: impl Fn(&T) -> Value,
) {
    let mut selected = current;
    egui::ComboBox::from_id_salt(format!("{}_{}", label, input_index))
        .selected_text(display_name(&selected))
        .show_ui(ui, |ui| {
            for variant in variants {
                if ui.selectable_value(&mut selected, variant.clone(), display_name(variant)).changed() {
                    change_value(tx_change_node, node_id, input_index, input, to_value(variant));
                }
            }
        });
}

/// All FilterType variants with their user-friendly display names.
const FILTER_TYPES: [(FilterType, &str); 5] = [
    (FilterType::Nearest, "Nearest Neighbor"),
    (FilterType::Triangle, "Linear (Triangle)"),
    (FilterType::CatmullRom, "Cubic (CatmullRom)"),
    (FilterType::Gaussian, "Gaussian"),
    (FilterType::Lanczos3, "Lanczos3"),
];

/// Returns a friendly display name for a FilterType variant.
fn filter_type_display_name(ft: &FilterType) -> String {
    FILTER_TYPES.iter()
        .find(|(variant, _)| variant == ft)
        .map(|(_, name)| name.to_string())
        .unwrap_or_else(|| format!("{:?}", ft))
}

pub fn show(ui: &mut egui::Ui, node: &mut GraphNode, tx_change_node: &Sender<ChangeNodeMessage>, theme: &Theme) -> NodeSettingsResponse {
    let mut node_settings_response = NodeSettingsResponse::new();

    // Heading: show custom name if set, otherwise operation name.
    let display_name = node.custom_name.as_deref().unwrap_or(&node.settings.name);
    ui.horizontal(|ui| {
        ui.heading(format!("{} settings", display_name));
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("X").clicked() {
                node_settings_response.deselect_node = true;
            }
        });
    });
    ui.label(egui::RichText::new(format!("{}", node.settings.description)).color(theme.get().text_faint));

    ui.add_space(12.0);

    ui.heading("inputs");
    ui.add_space(12.0);

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

                            // Show error indicator if the input has a validation error.
                            if input.is_error {
                                let error_text = input.error_message.as_deref().unwrap_or("error");
                                ui.label(RichText::new(error_text).color(Color32::RED).small());
                            }
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

    // --- Visualizations section ---
    // Show for nodes that have at least one image output.
    let first_image_output_index = node.outputs.iter().position(|o| matches!(&o.value, Value::Image { .. }));
    if let Some(output_index) = first_image_output_index {
        ui.add_space(40.0);
        ui.heading("visualizations");
        ui.add_space(12.0);

        // Collapsible histogram of the first image output
        egui::CollapsingHeader::new("histogram")
            .default_open(true)
            .show(ui, |ui| {
                histogram_widget::ensure_histogram_cache(node, output_index);
                if let Some(cache) = node.histogram_cache.get(&output_index) {
                    histogram_widget::draw_histogram(ui, cache, theme);
                }
            });
    }

    ui.add_space(40.0);
    ui.heading("settings");
    ui.add_space(12.0);

    ui.push_id("settings", |ui| {
        TableBuilder::new(ui).striped(true)
            .column(Column::auto().at_least(50.0).at_most(130.0).resizable(false))
            .column(Column::remainder().resizable(false))
            .header(30.0, |mut header| {
                header.col(|ui| {
                    ui.label(RichText::new("name").color(theme.get().text_faint));
                });
                header.col(|ui| {
                    ui.label(RichText::new("value").color(theme.get().text_faint));
                });
            })
            .body(|mut body| {
                // Custom name row
                body.row(30.0, |mut row| {
                    row.col(|ui| {
                        ui.horizontal_centered(|ui| {
                            ui.label("name");
                        });
                    });
                    row.col(|ui| {
                        ui.horizontal_centered(|ui| {
                            let mut name_text = node.custom_name.clone().unwrap_or_default();
                            if ui.add(TextEdit::singleline(&mut name_text).hint_text("custom name")).changed() {
                                let new_name = if name_text.is_empty() { None } else { Some(name_text) };
                                node.custom_name = new_name.clone();
                                let message = ChangeNodeMessage::SetCustomName {
                                    node_id: node.id.clone(),
                                    name: new_name,
                                };
                                let _ = tx_change_node.try_send(message);
                            }
                        });
                    });
                });

                // Enabled row
                body.row(30.0, |mut row| {
                    row.col(|ui| {
                        ui.horizontal_centered(|ui| {
                            ui.label("enabled");
                        });
                    });
                    row.col(|ui| {
                        ui.horizontal_centered(|ui| {
                            let mut is_enabled = node.is_enabled;
                            if ui.add(egui::Checkbox::new(&mut is_enabled, "")).changed() {
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
                        });
                    });
                });
            });
    });

    node_settings_response
}


/// Display a read-only output value. Shows all Value types with appropriate formatting.
fn output_value(ui: &mut egui::Ui, value: &Value) {
    match value {
        Value::Bool(v) => { ui.add(Label::new(v.to_string())); }
        Value::Integer(v) => { ui.add(Label::new(v.to_string())); }
        Value::Decimal(v) => { ui.add(Label::new(format!("{:.4}", v))); }
        Value::Text(v) => { ui.add(Label::new(v.to_string())); }
        Value::Color(v) => {
            let rgba = v.to_srgb_u8();
            let color = Color32::from_rgba_unmultiplied(rgba.0, rgba.1, rgba.2, rgba.3);
            show_color_swatch(ui, color);
        }
        Value::Image { data, change_id: _ } => {
            ui.add(Label::new(format!("{}x{} ({}ch)", data.width(), data.height(), data.channels())));
        }
        Value::Path(p) => { ui.add(Label::new(p.display().to_string())); }
        Value::FilterType(ft) => { ui.add(Label::new(filter_type_display_name(ft))); }
        Value::ColorFormat(cf) => { ui.add(Label::new(format!("{:?}", cf))); }
        Value::ImageType(it) => { ui.add(Label::new(format!("{:?}", it))); }
        Value::Trigger => { ui.add(Label::new("trigger")); }
        Value::NoiseWorleyDistanceFunction(v) => { ui.add(Label::new(format!("{:?}", v))); }
        Value::ColorSpace(v) => { ui.add(Label::new(format!("{:?}", v))); }
        Value::BlendMode(v) => { ui.add(Label::new(format!("{:?}", v))); }
        Value::TextHAlign(v) => { ui.add(Label::new(format!("{:?}", v))); }
        Value::TextVAlign(v) => { ui.add(Label::new(format!("{:?}", v))); }
    }
}


/// Render an interactive input widget appropriate for the value type.
/// Connected inputs show a read-only label; disconnected inputs show the full editor.
fn input_value(ui: &mut egui::Ui, value: Value, input: &mut Input, input_index: usize, node_id: &str, tx_change_node: &Sender<ChangeNodeMessage>, sibling_image_format: Option<image::ImageFormat>) {
    // Expand sliders and drag values to fill the available column width.
    ui.spacing_mut().slider_width = ui.available_width() - 60.0;

    match value {
        Value::Bool(a) => {
            if input.connection.is_some() {
                ui.label(a.to_string());
            } else {
                let mut x = a;
                if ui.add(egui::Checkbox::new(&mut x, "")).changed() {
                    change_value(tx_change_node, node_id, input_index, input, Value::Bool(x));
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
                                change_value(tx_change_node, node_id, input_index, input, Value::Integer(x));
                            }
                        },
                        InputSettings::Slider { range, step_by: _, clamp_to_range } => {
                            let clamping = if *clamp_to_range { egui::SliderClamping::Always } else { egui::SliderClamping::Never };
                            if ui.add(egui::Slider::new(&mut x, range.0 as i32..=range.1 as i32).clamping(clamping)).changed() {
                                change_value(tx_change_node, node_id, input_index, input, Value::Integer(x));
                            }
                        },
                        _ => {}
                    }
                }
            }
        }
        Value::Decimal(a) => {
            if input.connection.is_some() {
                ui.label(format!("{:.4}", a));
            } else {
                let mut x: f32 = a;

                if let Some(input_type) = &input.settings {
                    match input_type {
                        InputSettings::DragValue { speed, clamp } => {
                            let mut drag = egui::DragValue::new(&mut x);

                            if let Some(speed) = *speed {
                                drag = drag.speed(speed);
                            }
                            if let Some(clamp) = clamp {
                                drag = drag.range(clamp.0..=clamp.1);
                            }

                            if ui.add(drag).changed() {
                                change_value(tx_change_node, node_id, input_index, input, Value::Decimal(x));
                            }
                        },
                        InputSettings::Slider { range, step_by: _, clamp_to_range } => {
                            let clamping = if *clamp_to_range { egui::SliderClamping::Always } else { egui::SliderClamping::Never };
                            if ui.add(egui::Slider::new(&mut x, range.0..=range.1).clamping(clamping)).changed() {
                                change_value(tx_change_node, node_id, input_index, input, Value::Decimal(x));
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
                if ui.add(TextEdit::singleline(&mut x).hint_text("text")).changed() {
                    change_value(tx_change_node, node_id, input_index, input, Value::Text(x));
                }
            }
        }
        Value::Color(a) => {
            if input.connection.is_some() {
                let rgba = a.to_srgb_u8();
                let color = Color32::from_rgba_unmultiplied(rgba.0, rgba.1, rgba.2, rgba.3);
                show_color_swatch(ui, color);
            } else {
                let rgba = a.to_srgb_u8();
                let mut x = [rgba.0, rgba.1, rgba.2, rgba.3];
                if ui.color_edit_button_srgba_unmultiplied(&mut x).changed() {
                    let value = Value::Color(mangler_core::color::Color::from_srgb_u8(x[0], x[1], x[2], x[3]));
                    change_value(tx_change_node, node_id, input_index, input, value);
                }
            }
        },
        Value::FilterType(a) => {
            if input.connection.is_some() {
                ui.label(filter_type_display_name(&a));
            } else {
                // FilterType uses friendly display names instead of Debug format.
                let variants: Vec<FilterType> = FILTER_TYPES.iter().map(|(ft, _)| *ft).collect();
                show_enum_combo(
                    ui, "filter type", a, &variants,
                    |ft| filter_type_display_name(ft),
                    input, input_index, node_id, tx_change_node,
                    |ft| Value::FilterType(*ft),
                );
            }
        }
        Value::ColorFormat(a) => {
            if input.connection.is_some() {
                ui.label(format!("{:?}", a));
            } else {
                // ColorFormat needs special handling to grey out formats incompatible
                // with the sibling image format on this node.
                let mut x = a;
                egui::ComboBox::from_id_salt(format!("color_format_{}", input_index))
                    .selected_text(format!("{:?}", x))
                    .show_ui(ui, |ui| {
                        for color_format in ColorFormat::types().iter() {
                            let compatible = sibling_image_format
                                .as_ref()
                                .map(|fmt| color_format.is_compatible_with_image_format(fmt))
                                .unwrap_or(true);
                            ui.add_enabled_ui(compatible, |ui| {
                                if ui.selectable_value(&mut x, color_format.clone(), format!("{:?}", color_format)).changed() {
                                    change_value(tx_change_node, node_id, input_index, input, Value::ColorFormat(color_format.clone()));
                                }
                            });
                        }
                    });
            }
        }
        Value::Trigger => {
            if input.connection.is_some() {
                ui.label("trigger");
            } else if ui.add(egui::Button::new(input.name.clone())).clicked() {
                change_value(tx_change_node, node_id, input_index, input, Value::Trigger);
            }
        }
        Value::Image{data:_, change_id:_} => {

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

                        let extensions: Vec<&str> = extension_filter.iter().map(|s| s.as_str()).collect();
                        let title = set_title.unwrap_or("file".to_string());
                        let file_dialog = rfd::FileDialog::new().add_filter(&title, &extensions);

                        if let Some(save_path) = match file_dialog_type {
                            mangler_core::input::FileDialogType::PickFile => file_dialog.pick_file(),
                            mangler_core::input::FileDialogType::PickFolder => file_dialog.pick_folder(),
                            mangler_core::input::FileDialogType::SaveFile => file_dialog.save_file(),
                        } {
                            change_value(tx_change_node, node_id, input_index, input, Value::Path(save_path));
                        }
                    }
                }
            }
        }
        Value::ImageType(a) => {
            if input.connection.is_some() {
                ui.label(format!("{:?}", a));
            } else {
                // ImageType maps through ImageType::format() so it needs special handling.
                let mut x = a;
                egui::ComboBox::from_id_salt(format!("image_format_{}", input_index))
                    .selected_text(format!("{:?}", x))
                    .show_ui(ui, |ui| {
                        for image_type in mangler_core::value::ImageType::types().iter() {
                            if ui.selectable_value(&mut x, image_type.format(), image_type.format().extensions_str()[0].to_string()).changed() {
                                change_value(tx_change_node, node_id, input_index, input, Value::ImageType(image_type.format()));
                            }
                        }
                    });
            }
        },
        Value::NoiseWorleyDistanceFunction(a) => {
            if input.connection.is_some() {
                ui.label(format!("{:?}", a));
            } else {
                let variants = NoiseWorleyDistanceFunction::types();
                show_enum_combo(
                    ui, "distance function", a, &variants,
                    |v| format!("{:?}", v),
                    input, input_index, node_id, tx_change_node,
                    |v| Value::NoiseWorleyDistanceFunction(v.clone()),
                );
            }
        },
        Value::ColorSpace(a) => {
            if input.connection.is_some() {
                ui.label(format!("{:?}", a));
            } else {
                let variants = ColorSpace::types();
                show_enum_combo(
                    ui, "color space", a, &variants,
                    |v| format!("{:?}", v),
                    input, input_index, node_id, tx_change_node,
                    |v| Value::ColorSpace(v.clone()),
                );
            }
        }
        Value::BlendMode(a) => {
            if input.connection.is_some() {
                ui.label(format!("{:?}", a));
            } else {
                let variants = BlendMode::types();
                show_enum_combo(
                    ui, "blend mode", a, &variants,
                    |v| format!("{:?}", v),
                    input, input_index, node_id, tx_change_node,
                    |v| Value::BlendMode(v.clone()),
                );
            }
        }
        Value::TextHAlign(a) => {
            if input.connection.is_some() {
                ui.label(format!("{:?}", a));
            } else {
                let variants = TextHAlign::types();
                show_enum_combo(
                    ui, "h align", a, &variants,
                    |v| format!("{:?}", v),
                    input, input_index, node_id, tx_change_node,
                    |v| Value::TextHAlign(*v),
                );
            }
        }
        Value::TextVAlign(a) => {
            if input.connection.is_some() {
                ui.label(format!("{:?}", a));
            } else {
                let variants = TextVAlign::types();
                show_enum_combo(
                    ui, "v align", a, &variants,
                    |v| format!("{:?}", v),
                    input, input_index, node_id, tx_change_node,
                    |v| Value::TextVAlign(*v),
                );
            }
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
