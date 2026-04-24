use eframe::egui::{self, Label, Layout, RichText, TextEdit};
use epaint::{vec2, Color32};
use image::imageops::FilterType;
use mangler_core::{
    input::{Input, InputSettings},
    operations::Operation,
    value::{ColorFormat, Value, TextHAlign, TextVAlign, VideoCodec, VideoContainer},
    AddNodeType, ChangeGraphMessage, ChangeNodeMessage,
    operations::images::noise::worley_distance::NoiseWorleyDistanceFunction,
    color::{color_spaces::ColorSpace, blend::BlendMode},
};
use egui_extras::{TableBuilder, Column};
use tokio::sync::mpsc::Sender;
use crate::{graph::graph_node::GraphNode, program::RenderProgressUiState, settings::histogram_widget, themes::theme::Theme};

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
            // Justify so every item fills the full popup width — keeps the
            // hover highlight a consistent size as the mouse moves between
            // items that have differently-sized labels.
            ui.with_layout(
                egui::Layout::top_down(egui::Align::Min).with_cross_justify(true),
                |ui| {
                    for variant in variants {
                        if ui.selectable_value(&mut selected, variant.clone(), display_name(variant)).changed() {
                            change_value(tx_change_node, node_id, input_index, input, to_value(variant));
                        }
                    }
                },
            );
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

pub fn show(
    ui: &mut egui::Ui,
    node: &mut GraphNode,
    tx_change_node: &Sender<ChangeNodeMessage>,
    tx_change_graph: &Sender<ChangeGraphMessage>,
    render_state: &mut Option<RenderProgressUiState>,
    theme: &Theme,
) -> NodeSettingsResponse {
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

    // Video output nodes get a dedicated Render button + progress bar. The
    // button sends a StartRender message to the engine, which spawns the
    // render on a separate task; progress comes back via GraphChangedMessage
    // and is plumbed into `render_state` by Program.
    if matches!(&node.node_type, Some(AddNodeType::Operation(Operation::OpVideoOutputFile))) {
        show_video_compat_warning(ui, node, theme);
        show_render_section(ui, node, tx_change_graph, render_state, theme);
        ui.add_space(12.0);
    }

    // Subgraph nodes get a dedicated file picker in place of the old synthetic
    // "file path" input slot. The picker drives NodeType::Subgraph.path via the
    // SetSubgraphPath message; exposed inputs/outputs from the child surface
    // below in the normal inputs table.
    if node.is_subgraph {
        ui.heading("subgraph");
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let label = match &node.subgraph_path {
                Some(p) => p.file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| p.display().to_string()),
                None => "(no file selected)".to_string(),
            };
            ui.label(label);
            if ui.button("🗀").clicked() {
                let file_dialog = rfd::FileDialog::new()
                    .add_filter("subgraph (*.mangle.json)", &["json"]);
                if let Some(picked) = file_dialog.pick_file() {
                    node.subgraph_path = Some(picked.clone());
                    let message = ChangeNodeMessage::SetSubgraphPath {
                        node_id: node.id.clone(),
                        path: picked,
                    };
                    if let Err(err) = tx_change_node.try_send(message) {
                        println!("Error sending SetSubgraphPath: {:?}", err);
                    }
                }
            }
        });
        ui.add_space(12.0);
    }

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
        .column(Column::exact(26.0).resizable(false))
        .header(30.0, |mut header| {
            header.col(|ui| {
                ui.label(RichText::new("name").color(theme.get().text_faint));
            });
            header.col(|ui| {
                ui.label(RichText::new("value").color(theme.get().text_faint));
            });
            header.col(|ui| {
                ui.label(RichText::new("exp").color(theme.get().text_faint));
            });
        })
        .body(|mut body| {
            for (input_index, input) in node.inputs.iter_mut().enumerate() {

                body.row(30.0, |mut row| {
                    row.col(|ui| {
                        ui.horizontal_centered(|ui| {
                            // Double-clicking the input name resets the value
                            // to the operation's default. `Sense::click()` is
                            // needed because `ui.label` allocates a
                            // hover-only rect that can't detect clicks.
                            let label_response = ui
                                .add(Label::new(&input.name).sense(egui::Sense::click()))
                                .on_hover_text("Double-click to reset to default");
                            if label_response.double_clicked() {
                                let default = input.default_value.clone();
                                change_value(tx_change_node, &node.id, input_index, input, default);
                            }
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

                    row.col(|ui| {
                        ui.horizontal_centered(|ui| {
                            let mut is_exposed = input.is_exposed;
                            if ui.add(egui::Checkbox::new(&mut is_exposed, "")).changed() {
                                let message = ChangeNodeMessage::SetExposeInput {
                                    node_id: node.id.clone(),
                                    input_index,
                                    set_to: is_exposed,
                                };
                                match tx_change_node.try_send(message) {
                                    Ok(_) => { input.is_exposed = is_exposed; }
                                    Err(err) => {
                                        println!("Error sending SetExposeInput: {:?}", err);
                                    }
                                }
                            }
                        });
                    });
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
            .column(Column::exact(26.0).resizable(false))
            .header(30.0, |mut header| {
                header.col(|ui| {
                    ui.label(RichText::new("name").color(theme.get().text_faint));
                });
                header.col(|ui| {
                    ui.label(RichText::new("value").color(theme.get().text_faint));
                });
                header.col(|ui| {
                    ui.label(RichText::new("exp").color(theme.get().text_faint));
                });
            })
            .body(|mut body| {
                for (output_index, output) in node.outputs.iter_mut().enumerate() {
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

                        row.col(|ui| {
                            ui.horizontal_centered(|ui| {
                                let mut is_exposed = output.is_exposed;
                                if ui.add(egui::Checkbox::new(&mut is_exposed, "")).changed() {
                                    let message = ChangeNodeMessage::SetExposeOutput {
                                        node_id: node.id.clone(),
                                        output_index,
                                        set_to: is_exposed,
                                    };
                                    match tx_change_node.try_send(message) {
                                        Ok(_) => { output.is_exposed = is_exposed; }
                                        Err(err) => {
                                            println!("Error sending SetExposeOutput: {:?}", err);
                                        }
                                    }
                                }
                            });
                        });
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
                            let name_width = (ui.available_width() - 10.0).clamp(60.0, 200.0);
                            let mut name_text = node.custom_name.clone().unwrap_or_default();
                            if ui.add(TextEdit::singleline(&mut name_text).hint_text("custom name").desired_width(name_width)).changed() {
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
        Value::VideoContainer(v) => { ui.add(Label::new(format!("{:?}", v))); }
        Value::VideoCodec(v) => { ui.add(Label::new(format!("{:?}", v))); }
        Value::Video(v) => {
            // Just show the filename — full path can be long and the
            // thumbnail handles visual identification.
            let name = v.path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("(no video)");
            ui.add(Label::new(name));
        }
    }
}


/// Render an interactive input widget appropriate for the value type.
/// Connected inputs show a read-only label; disconnected inputs show the full editor.
fn input_value(ui: &mut egui::Ui, value: Value, input: &mut Input, input_index: usize, node_id: &str, tx_change_node: &Sender<ChangeNodeMessage>, sibling_image_format: Option<image::ImageFormat>) {
    // Size interactive widgets to the available column width, but clamp them:
    // in egui_extras tables `Column::remainder()` can report a very large
    // `available_width` for unconstrained parent layouts, which would push
    // widgets past the visible panel. An off-screen slider track also captures
    // stray pointer events, so the value appears to drift on its own and the
    // knob ends up out of sight.
    ui.spacing_mut().slider_width = (ui.available_width() - 80.0).clamp(80.0, 140.0);
    let text_width = (ui.available_width() - 10.0).clamp(60.0, 140.0);

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
                            // update_while_editing(false): keyboard typing only
                            // commits on Enter / blur, not per keystroke. Dragging
                            // still streams continuously — that path ignores this
                            // flag. Without this, typing "500" fires three
                            // SetInputs (5, 50, 500) and each runs a full decode.
                            let mut drag = egui::DragValue::new(&mut x)
                                .update_while_editing(false);

                            drag = if let Some(clamp) = clamp {
                                drag.range(clamp.0..=clamp.1)
                            } else {
                                drag
                            };

                            ui.add(drag);
                            // Compare against the captured starting value rather
                            // than trusting response.changed() — TextEdit inside
                            // DragValue fires .changed() on every keystroke even
                            // when the committed value hasn't moved.
                            if x != a {
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
                            // See Integer DragValue above for why update_while_editing(false)
                            // and x-compare instead of response.changed().
                            let mut drag = egui::DragValue::new(&mut x)
                                .update_while_editing(false);

                            if let Some(speed) = *speed {
                                drag = drag.speed(speed);
                            }
                            if let Some(clamp) = clamp {
                                drag = drag.range(clamp.0..=clamp.1);
                            }

                            ui.add(drag);
                            if x != a {
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
            } else if let Some(InputSettings::Dropdown { options }) = &input.settings {
                // Dropdown selector for predefined text options.
                let options = options.clone();
                let mut selected = a.clone();
                egui::ComboBox::from_id_salt(format!("text_dropdown_{}", input_index))
                    .selected_text(&selected)
                    .show_ui(ui, |ui| {
                        ui.with_layout(
                            egui::Layout::top_down(egui::Align::Min).with_cross_justify(true),
                            |ui| {
                                for option in &options {
                                    if ui.selectable_value(&mut selected, option.clone(), option).changed() {
                                        change_value(tx_change_node, node_id, input_index, input, Value::Text(selected.clone()));
                                    }
                                }
                            },
                        );
                    });
            } else if let Some(InputSettings::MultiLineText) = &input.settings {
                // Multi-line text area.
                let mut x = a;
                if ui.add(TextEdit::multiline(&mut x).hint_text("text").desired_width(text_width)).changed() {
                    change_value(tx_change_node, node_id, input_index, input, Value::Text(x));
                }
            } else {
                // Single-line text field (default).
                let mut x = a;
                if ui.add(TextEdit::singleline(&mut x).hint_text("text").desired_width(text_width)).changed() {
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
                        ui.with_layout(
                            egui::Layout::top_down(egui::Align::Min).with_cross_justify(true),
                            |ui| {
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
                            },
                        );
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
                // Leave room for the sibling folder-picker button.
                let path_width = (text_width - 30.0).max(40.0);
                ui.add_enabled_ui(false, |ui| {
                    ui.add(
                        TextEdit::singleline(
                            &mut path.clone().into_os_string().into_string().unwrap_or_default(),
                        )
                        .desired_width(path_width),
                    );
                });

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
                        ui.with_layout(
                            egui::Layout::top_down(egui::Align::Min).with_cross_justify(true),
                            |ui| {
                                for image_type in mangler_core::value::ImageType::types().iter() {
                                    if ui.selectable_value(&mut x, image_type.format(), image_type.format().extensions_str()[0].to_string()).changed() {
                                        change_value(tx_change_node, node_id, input_index, input, Value::ImageType(image_type.format()));
                                    }
                                }
                            },
                        );
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
        Value::VideoContainer(a) => {
            if input.connection.is_some() {
                ui.label(format!("{:?}", a));
            } else {
                let variants = VideoContainer::types();
                show_enum_combo(
                    ui, "container", a, &variants,
                    |v| format!("{:?}", v),
                    input, input_index, node_id, tx_change_node,
                    |v| Value::VideoContainer(*v),
                );
            }
        }
        Value::VideoCodec(a) => {
            if input.connection.is_some() {
                ui.label(format!("{:?}", a));
            } else {
                let variants = VideoCodec::types();
                show_enum_combo(
                    ui, "codec", a, &variants,
                    |v| format!("{:?}", v),
                    input, input_index, node_id, tx_change_node,
                    |v| Value::VideoCodec(*v),
                );
            }
        }
        Value::Video(v) => {
            // Read-only: Video values are produced by the `video from file`
            // node; inputs only take them via connection. Show the filename
            // (or "(no video)" for an unconnected/default handle).
            let name = v.path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("(no video)");
            ui.label(name);
        }
    }
}


/// Soft warning row shown on `OpVideoOutputFile` nodes when the currently
/// selected `(container, codec)` pair is either (a) invalid per the
/// compatibility matrix, or (b) valid but not yet implemented by the encoder.
///
/// Doesn't block render or save — the hard stop is at `VideoEncoder::open`.
/// Silent when the combo is fine.
fn show_video_compat_warning(ui: &mut egui::Ui, node: &GraphNode, theme: &Theme) {
    let container = node.inputs.iter().find_map(|i| match &i.value {
        Value::VideoContainer(v) => Some(*v),
        _ => None,
    });
    let codec = node.inputs.iter().find_map(|i| match &i.value {
        Value::VideoCodec(v) => Some(*v),
        _ => None,
    });

    let (Some(container), Some(codec)) = (container, codec) else { return };

    // Pick the more specific message: illegal combos take priority over
    // not-yet-implemented, since the user's first fix is to change the combo.
    let message = if !codec.is_supported_in(container) {
        let supported: Vec<_> = container
            .supported_codecs()
            .iter()
            .map(|c| format!("{:?}", c))
            .collect();
        Some(format!(
            "⚠ {:?} is not valid in a {:?} container. Supported in {:?}: {}.",
            codec,
            container,
            container,
            supported.join(", "),
        ))
    } else if !mangler_core::video::VideoEncoder::has_encoder_preset(container, codec) {
        Some(format!(
            "⚠ Encoder for {:?} + {:?} is not yet implemented. Try {:?} + H264 for now.",
            container, codec, container,
        ))
    } else {
        None
    };

    if let Some(text) = message {
        // Use the theme's "error" color (currently reused for connection-dot
        // errors). If a dedicated warning swatch is added later, swap this in.
        let warning_color = theme.get().grid_connection_dot_error;
        ui.label(egui::RichText::new(text).color(warning_color).size(12.0));
        ui.add_space(6.0);
    }
}


/// Render button + progress UI for an `OpVideoOutputFile` node.
///
/// Shows a Render button unless a render is already in progress (tracked in
/// `render_state`), in which case it's replaced by a progress bar and a
/// frame-counter readout. The button clicks send `StartRender` addressed to
/// this node's id.
fn show_render_section(
    ui: &mut egui::Ui,
    node: &GraphNode,
    tx_change_graph: &Sender<ChangeGraphMessage>,
    render_state: &mut Option<RenderProgressUiState>,
    theme: &Theme,
) {
    ui.heading("render");
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new("Render the graph frame-by-frame into the output video file.")
            .color(theme.get().text_faint),
    );
    ui.add_space(8.0);

    if let Some(rs) = render_state {
        // `total == 0` is the "preparing" sentinel — set locally on the
        // button click before the engine's first RenderProgress arrives so
        // the UI reacts immediately instead of sitting on a Render button
        // for a few seconds while the warm-up graph run finishes.
        let is_preparing = rs.total == 0;
        let total = rs.total.max(1) as f32;
        let progress = rs.frame as f32 / total;
        let label = if is_preparing {
            "Starting render…".to_string()
        } else {
            format!("{} / {}", rs.frame, rs.total)
        };
        // egui's ProgressBar always left-aligns its label (`outer_rect.
        // left_center()` in the widget impl). To centre, render the bar
        // without text and paint a centred galley on top of the response
        // rect ourselves — same text style & colour the widget would use.
        let bar = egui::ProgressBar::new(progress.clamp(0.0, 1.0)).corner_radius(2.0);
        let response = ui.add(bar);
        let visuals = ui.visuals().clone();
        let text_color = visuals
            .override_text_color
            .unwrap_or(visuals.selection.stroke.color);
        let font_id = egui::TextStyle::Button.resolve(ui.style());
        ui.painter().with_clip_rect(response.rect).text(
            response.rect.center(),
            egui::Align2::CENTER_CENTER,
            &label,
            font_id,
            text_color,
        );
        if !is_preparing {
            let eta = if rs.frame > 0 {
                let elapsed = rs.started_at.elapsed().as_secs_f32();
                let remaining_frames = rs.total.saturating_sub(rs.frame) as f32;
                let per_frame = elapsed / rs.frame as f32;
                (remaining_frames * per_frame).max(0.0)
            } else {
                0.0
            };
            ui.label(format!("rendering: {:.1}s remaining", eta));
        }
    } else {
        // Full-width render button with centred text. egui's AtomLayout
        // pushes atoms flush-left by default; wrapping the text between two
        // `Atom::grow()` spacers makes the layout distribute extra space
        // equally on both sides, centring the label in the button's min_size.
        let button = egui::Button::new((
            egui::Atom::grow(),
            RichText::new("Render").size(16.0),
            egui::Atom::grow(),
        ))
        .min_size(egui::vec2(ui.available_width(), 30.0));
        if ui.add(button).clicked() {
            let msg = ChangeGraphMessage::StartRender {
                output_node_id: node.id.clone(),
            };
            if let Err(err) = tx_change_graph.try_send(msg) {
                println!("Error sending StartRender: {:?}", err);
            } else {
                // Immediate UI feedback: flip into the "preparing" state.
                // The engine tick + warm-up graph.run() can take seconds on
                // a big clip, so without this the button just sits there
                // unresponsive. Cleared when the first RenderProgress
                // arrives (real totals replace the 0/0 placeholder) or when
                // RenderFinished / RenderFailed resets render_state.
                *render_state = Some(RenderProgressUiState {
                    frame: 0,
                    total: 0,
                    started_at: std::time::Instant::now(),
                });
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
