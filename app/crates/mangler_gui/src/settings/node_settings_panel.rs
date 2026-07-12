use eframe::egui::{self, Label, Layout, RichText, TextEdit};
use epaint::{vec2, Color32};
use image::imageops::FilterType;
use mangler_core::{
    input::{Input, InputSettings},
    value::{ColorFormat, EdgeMode, ExportPreset, Value, TextHAlign, TextVAlign},
    curve::{Curve, CurveInterpolation},
    ChangeNodeMessage,
    operations::images::noise::cellular::worley_distance::NoiseWorleyDistanceFunction,
    color::{color_spaces::ColorSpace, blend::BlendMode},
};
use egui_extras::{TableBuilder, Column};
use tokio::sync::mpsc::Sender;
use crate::{
    graph::graph_node::GraphNode,
    settings::{histogram_widget, section::{section_label, section_rule}},
    themes::theme::Theme,
};

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

/// Build the hover tooltip text for an input's name label.
///
/// Combines the input's description (when the operation provides one) with
/// the "double-click to reset to default" affordance hint, separated by a
/// blank line. If the description is empty, only the reset hint is shown so
/// the existing tooltip behaviour is preserved for operations that have not
/// been given descriptions yet.
fn build_socket_hover_text(description: &str) -> String {
    if description.is_empty() {
        "Double-click to reset to default".to_string()
    } else {
        format!("{}\n\nDouble-click to reset to default", description)
    }
}

/// Draw a solid color rectangle as a swatch preview.
fn show_color_swatch(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(40.0, 18.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, 2.0, color);
    }
}

/// Paint a small solid disclosure triangle inside `rect`: right-pointing when
/// closed, down-pointing when open — the same affordance as egui's own
/// `CollapsingHeader` icon. Painted directly with `Shape::convex_polygon`
/// rather than the U+25B8/U+25BE Unicode triangle glyphs, which aren't
/// covered by the app's fonts and rendered as replacement-glyph boxes (the
/// reported "square after help").
fn paint_disclosure_triangle(ui: &egui::Ui, rect: egui::Rect, open: bool, color: Color32) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.5 * 0.7;
    let points = if open {
        // Down-pointing triangle.
        vec![c + vec2(-r, -r * 0.6), c + vec2(r, -r * 0.6), c + vec2(0.0, r * 0.8)]
    } else {
        // Right-pointing triangle.
        vec![c + vec2(-r * 0.6, -r), c + vec2(-r * 0.6, r), c + vec2(r * 0.8, 0.0)]
    };
    ui.painter().add(egui::Shape::convex_polygon(points, color, egui::Stroke::NONE));
}

/// Borderless "✕" close control for the node settings panel header.
///
/// Quiet (`text_faint`) at rest, switches to the theme's selected-menu-item
/// accent (`menu_bar_button_selected` — still the rose accent in dark_green,
/// even after `widgets_active_bg_fill` stops being rose) on hover, with a
/// pointing-hand cursor so it still reads as clickable despite having no
/// button frame. Returns true if clicked this frame.
///
/// A plain `ui.add(Label::new(...))` bakes its text color in at layout time,
/// before we get a chance to know whether the pointer is hovering it. So
/// this instead lays the glyph out once, allocates+interacts the rect to get
/// an accurate `hovered()` for *this* frame (egui hit-tests against the live
/// pointer position as soon as a rect is allocated — no one-frame lag), and
/// then paints the galley with `galley_with_override_text_color` using
/// whichever color that hover state resolves to.
fn close_control(ui: &mut egui::Ui, theme: &Theme) -> bool {
    let font_id = egui::TextStyle::Body.resolve(ui.style());
    // The color baked in here doesn't matter — it's replaced below.
    let galley = ui.painter().layout_no_wrap("✕".to_owned(), font_id, theme.get().text_faint);
    let (rect, response) = ui.allocate_exact_size(galley.size(), egui::Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

    let color = if response.hovered() {
        theme.get().menu_bar_button_selected
    } else {
        theme.get().text_faint
    };

    if ui.is_rect_visible(rect) {
        ui.painter().galley_with_override_text_color(rect.min, galley, color);
    }

    response.clicked()
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
    theme: &Theme,
    // The focused graph's folder (from its save path), used to seed the
    // starting directory of a Path input's file dialog when the input doesn't
    // pin one explicitly. `None` for an unsaved graph.
    default_dir: Option<&std::path::Path>,
) -> NodeSettingsResponse {
    let mut node_settings_response = NodeSettingsResponse::new();

    // Title row: "{name} settings" as a small semibold label, plus a
    // borderless close control right-aligned. Replaces the old 22px
    // ui.heading() + framed "X" button — this panel is visible constantly
    // while a node is selected, so the new design keeps its chrome quiet.
    let display_name = node.custom_name.as_deref().unwrap_or(&node.settings.name);
    ui.horizontal(|ui| {
        // `.strong()` resolves to `widgets.active.fg_stroke`, which is a rose
        // accent in dark_green — not a weight change. Use the dedicated
        // semibold font family instead so the title is actually bold, with no
        // explicit color override (the panel's normal text color is correct).
        ui.label(RichText::new(format!("{} settings", display_name)).size(15.0).family(crate::themes::theme::semibold_family()));
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            // Leave room for the corner kind-switcher button that panel_view.rs
            // draws over the panel's top-right corner (rect.right()-26..-6,
            // top()+4..+24) — without this gap the ✕ lands underneath it.
            ui.add_space(24.0);
            if close_control(ui, theme) {
                node_settings_response.deselect_node = true;
            }
        });
    });

    // Description, as a plain wrapped label. The "help ▸/▾" toggle used to be
    // glued to the end of this text via `horizontal_wrapped`, but that made
    // it possible for the toggle to land mid-line, and it used Unicode
    // disclosure-triangle glyphs the app's fonts don't cover (rendered as
    // replacement-glyph boxes — the reported "square after help"). It now
    // lives on its own line below, with a hand-painted triangle instead.
    // Keyed by node id so the open/closed state is remembered per node —
    // without it, opening help on one node would leave it open for whatever
    // node is selected next in the same panel.
    let help_open_id = ui.id().with(&node.id).with("node_help_open");
    let mut help_open = ui.data(|d| d.get_temp::<bool>(help_open_id).unwrap_or(false));
    let has_help = !node.settings.help.is_empty();

    ui.label(
        RichText::new(&node.settings.description)
            .color(theme.get().text_faint)
            .size(12.0),
    );

    if has_help {
        // The "help" label and the disclosure triangle are allocated as
        // separate responses (a `Label` can't easily share a rect with a
        // hand-painted shape) but unioned into one response so the whole
        // row is a single clickable target with one pointing-hand cursor.
        let toggle_response = ui
            .horizontal(|ui| {
                let label_response = ui.add(
                    Label::new(
                        RichText::new("help")
                            .color(theme.get().text_link)
                            .size(12.0),
                    )
                    .sense(egui::Sense::click()),
                );
                let (icon_rect, icon_response) =
                    ui.allocate_exact_size(vec2(10.0, 10.0), egui::Sense::click());
                paint_disclosure_triangle(ui, icon_rect, help_open, theme.get().text_link);
                label_response.union(icon_response)
            })
            .inner
            .on_hover_cursor(egui::CursorIcon::PointingHand);

        if toggle_response.clicked() {
            help_open = !help_open;
        }

        // Only persist state for nodes that actually have help text, so temp
        // memory doesn't accumulate an entry per node for nothing.
        ui.data_mut(|d| d.insert_temp(help_open_id, help_open));
        if help_open {
            ui.add_space(4.0);
            ui.label(
                RichText::new(&node.settings.help)
                    .color(theme.get().text_faint)
                    .size(12.0),
            );
        }
    }

    // Subgraph nodes get a dedicated file picker in place of the old synthetic
    // "file path" input slot. The picker drives NodeType::Subgraph.path via the
    // SetSubgraphPath message; exposed inputs/outputs from the child surface
    // below in the normal inputs table.
    if node.is_subgraph {
        section_rule(ui, theme);
        section_label(ui, "subgraph");
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
                    .add_filter("NodeMangler graph", &["json"]);
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
    }

    section_rule(ui, theme);
    section_label(ui, "inputs");

    // Extract sibling image format before the mutable input loop so the
    // ColorFormat dropdown can grey out incompatible formats.
    let sibling_image_format = node.inputs.iter().find_map(|i| {
        if let Value::ImageType(fmt) = &i.value {
            Some(fmt.clone())
        } else {
            None
        }
    });

    // The transform node's `fill color` input only applies when its `edge`
    // mode is Fill; used below to hide it for the other modes.
    let sibling_edge_mode = node.inputs.iter().find_map(|i| {
        if let Value::EdgeMode(m) = &i.value { Some(*m) } else { None }
    });

    // The material node's `texture N ...` slot inputs only apply when its
    // `preset` is Custom; used below to hide them for the builtin presets.
    let sibling_export_preset = node.inputs.iter().find_map(|i| {
        if let Value::ExportPreset(p) = &i.value { Some(*p) } else { None }
    });

    // On the image output nodes, the manual `save` button only applies when
    // `auto save` is off; used below to hide it when auto-save is on.
    let sibling_auto_save = node.inputs.iter().find_map(|i| {
        if i.name == "auto save" {
            if let Value::Bool(b) = i.value { Some(b) } else { None }
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

    // Right edge (screen x) where the value column must end. Each value cell
    // is pinned to this (see the `set_max_width` below) so its widgets fill the
    // column and line up. Captured here from the panel's own (stable) width —
    // NOT from inside the table, where the remainder cell's available_width/
    // max_rect ratchet upward every frame. Reserve the 26px expose column and
    // the item spacing before it.
    let value_col_right = ui.max_rect().right() - 26.0 - ui.spacing().item_spacing.x;

    ui.push_id("inputs", |ui| {
        // No header row and no striping: the compact design relies on the
        // "inputs" section label above plus generous row height instead of a
        // name/value/exp header to explain the columns.
        // The value column is `.clip(true)`: without it, a long unclipped
        // value (e.g. a url output, a long text default) inflates the
        // table's min content width past the panel's available width, which
        // then pushes every width-derived sibling (histogram, section rules)
        // off screen too. Clipping bounds the column at its allotted width
        // regardless of content length.
        TableBuilder::new(ui)
        .column(Column::auto().at_least(50.0).at_most(130.0).resizable(false))
        .column(Column::remainder().clip(true).resizable(false))
        .column(Column::exact(26.0).resizable(false))
        .body(|mut body| {
            for (input_index, input) in node.inputs.iter_mut().enumerate() {
                // Hide encoder settings that don't apply to the selected image
                // format (only nodes with an ImageType input have these).
                // Connected or exposed inputs stay visible so they can always
                // be seen and disconnected; hidden values are preserved.
                if let Some(ref fmt) = sibling_image_format {
                    let inapplicable = match input.name.as_str() {
                        "quality" => !matches!(fmt, image::ImageFormat::Jpeg | image::ImageFormat::Avif),
                        "png compression" => *fmt != image::ImageFormat::Png,
                        _ => false,
                    };
                    if inapplicable && input.connection.is_none() && !input.is_exposed {
                        continue;
                    }
                }

                // Hide `fill color` unless the sibling `edge` mode is Fill.
                // Connected/exposed inputs stay visible so they can be managed.
                if input.name == "fill color"
                    && matches!(sibling_edge_mode, Some(m) if m != EdgeMode::Fill)
                    && input.connection.is_none()
                    && !input.is_exposed
                {
                    continue;
                }

                // Hide `texture N ...` slot inputs unless the sibling `preset`
                // is Custom. Connected/exposed inputs stay visible so they can
                // be managed.
                if input.name.starts_with("texture ")
                    && matches!(sibling_export_preset, Some(p) if p != ExportPreset::Custom)
                    && input.connection.is_none()
                    && !input.is_exposed
                {
                    continue;
                }

                // Hide the manual `save` button while `auto save` is on — there
                // is nothing to press for. Connected/exposed stay visible.
                if input.name == "save"
                    && matches!(input.settings, Some(InputSettings::Button))
                    && sibling_auto_save == Some(true)
                    && input.connection.is_none()
                    && !input.is_exposed
                {
                    continue;
                }

                body.row(24.0, |mut row| {
                    row.col(|ui| {
                        ui.horizontal_centered(|ui| {
                            // Double-clicking the input name resets the value
                            // to the operation's default. `Sense::click()` is
                            // needed because `ui.label` allocates a
                            // hover-only rect that can't detect clicks.
                            let label_response = ui.add(
                                Label::new(RichText::new(&input.name).color(theme.get().text_faint))
                                    .sense(egui::Sense::click()),
                            );
                            // Build a hover tooltip that combines the input's
                            // description (when the operation provides one)
                            // with the "double-click to reset" affordance hint.
                            let hover_text = build_socket_hover_text(&input.description);
                            let label_response = label_response.on_hover_text(hover_text);
                            if label_response.double_clicked() {
                                let default = input.default_value.clone();
                                change_value(tx_change_node, &node.id, input_index, input, default);
                            }
                        });
                    });

                    row.col(|ui| {
                        // Pin the cell to the true value-column width. This
                        // remainder cell's own reported width ratchets upward
                        // every frame (a feedback loop with egui_extras'
                        // persisted `max_used_widths`), so instead we derive the
                        // width from stable geometry: the cell's left edge (which
                        // doesn't move) and the column's right edge (captured
                        // from the panel above). With the cell pinned, every
                        // widget in `input_value` can just fill `available_width`
                        // and line up on the right — and the ratchet unwinds
                        // because content no longer exceeds the column.
                        ui.set_max_width((value_col_right - ui.max_rect().left()).max(60.0));
                        ui.horizontal_centered(|ui| {
                            input_value(ui, input.value.clone(), input, input_index, &node.id, &tx_change_node, sibling_image_format, theme, default_dir);

                            // Show error indicator if the input has a validation error.
                            // Uses the theme's error color (same one the graph
                            // editor uses for error connection dots) instead of
                            // a hardcoded red, so it stays consistent across themes.
                            if input.is_error {
                                let error_text = input.error_message.as_deref().unwrap_or("error");
                                ui.label(RichText::new(error_text).color(theme.get().grid_connection_dot_error).small());
                            }
                        });
                    });

                    row.col(|ui| {
                        ui.horizontal_centered(|ui| {
                            let mut is_exposed = input.is_exposed;
                            // The "exp" column header is gone in the compact
                            // design, so the checkbox now explains itself
                            // on hover instead.
                            if ui.add(egui::Checkbox::new(&mut is_exposed, ""))
                                .on_hover_text("expose on parent subgraph")
                                .changed()
                            {
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


    section_rule(ui, theme);
    section_label(ui, "outputs");

    ui.push_id("outputs", |ui| {
        // See the "inputs" table above: no header row, no striping.
        TableBuilder::new(ui)
            .column(Column::auto().at_least(50.0).at_most(130.0).resizable(false))
            .column(Column::remainder().clip(true).resizable(false))
            .column(Column::exact(26.0).resizable(false))
            .body(|mut body| {
                for (output_index, output) in node.outputs.iter_mut().enumerate() {
                    body.row(24.0, |mut row| {
                        row.col(|ui| {
                            ui.horizontal_centered(|ui| {
                                let label = ui.label(RichText::new(&output.name).color(theme.get().text_faint));
                                // Show the per-output description as a tooltip
                                // when the operation provides one. Outputs have
                                // no double-click-reset affordance, so empty
                                // descriptions simply suppress the tooltip.
                                if !output.description.is_empty() {
                                    label.on_hover_text(&output.description);
                                }
                            });
                        });

                        row.col(|ui| {
                            ui.horizontal_centered(|ui| {
                                output_value(ui, &output.value, theme);
                            });
                        });

                        row.col(|ui| {
                            ui.horizontal_centered(|ui| {
                                let mut is_exposed = output.is_exposed;
                                // The "exp" column header is gone in the compact
                                // design, so the checkbox now explains itself
                                // on hover instead.
                                if ui.add(egui::Checkbox::new(&mut is_exposed, ""))
                                    .on_hover_text("expose on parent subgraph")
                                    .changed()
                                {
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
        section_rule(ui, theme);
        section_label(ui, "visualizations");

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

    // Renamed from "settings" (which collided with the panel's own name) to
    // "node" — this section holds node-level (not operation-level) settings:
    // the custom display name and the enabled toggle.
    section_rule(ui, theme);
    section_label(ui, "node");

    // Right edge (screen x) of this table's value column. No expose column
    // here, so it runs to the panel's content edge. Captured from the stable
    // panel width — see the note in `input_value` on why the in-cell width is
    // unreliable.
    let node_value_col_right = ui.max_rect().right();

    ui.push_id("node_settings", |ui| {
        // See the "inputs" table above: no header row, no striping.
        TableBuilder::new(ui)
            .column(Column::auto().at_least(50.0).at_most(130.0).resizable(false))
            .column(Column::remainder().clip(true).resizable(false))
            .body(|mut body| {
                // Custom name row
                body.row(24.0, |mut row| {
                    row.col(|ui| {
                        ui.horizontal_centered(|ui| {
                            ui.label(RichText::new("name").color(theme.get().text_faint));
                        });
                    });
                    row.col(|ui| {
                        // Pin the cell to the true value-column width (this
                        // table has no expose column, so it runs to the panel
                        // edge). See the note in the inputs table above.
                        ui.set_max_width((node_value_col_right - ui.max_rect().left()).max(60.0));
                        ui.horizontal_centered(|ui| {
                            let name_width = ui.available_width();
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
                body.row(24.0, |mut row| {
                    row.col(|ui| {
                        ui.horizontal_centered(|ui| {
                            ui.label(RichText::new("enabled").color(theme.get().text_faint));
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


/// Quiet one-line summary of a curve, e.g. `"3 pts · open · smooth"`. Shared by
/// the input/output settings arms (the curve itself is edited in the 2D preview).
pub(crate) fn curve_summary(curve: &Curve) -> String {
    let closed = if curve.closed { "closed" } else { "open" };
    let interp = match curve.interpolation {
        CurveInterpolation::Linear => "linear",
        CurveInterpolation::Smooth => "smooth",
        CurveInterpolation::Bezier => "bezier",
    };
    format!("{} pts · {} · {}", curve.points.len(), closed, interp)
}

/// Display a read-only output value. Shows all Value types with appropriate formatting.
///
/// Everything except the color swatch renders as monospace `text_faint` —
/// these are read-only data values, not editable settings, so they're kept
/// visually quiet and use a fixed-width font that suits numbers/dimensions.
fn output_value(ui: &mut egui::Ui, value: &Value, theme: &Theme) {
    let faint = theme.get().text_faint;
    // Small helper so each arm below is a single call instead of repeating
    // `RichText::new(...).monospace().color(faint)` at every match arm.
    let mono = |ui: &mut egui::Ui, text: String| {
        ui.add(Label::new(RichText::new(text).monospace().color(faint)));
    };
    // Same, but for value kinds that can be arbitrarily long (urls, paths):
    // `.truncate()` ellipsizes instead of letting the label's natural width
    // inflate the (now-clipped) table column back open past the panel edge.
    let mono_truncate = |ui: &mut egui::Ui, text: String| {
        ui.add(Label::new(RichText::new(text).monospace().color(faint)).truncate());
    };

    match value {
        Value::Bool(v) => mono(ui, v.to_string()),
        Value::Integer(v) => mono(ui, v.to_string()),
        Value::Decimal(v) => mono(ui, format!("{:.4}", v)),
        Value::Text(v) => mono_truncate(ui, v.to_string()),
        Value::Color(v) => {
            let rgba = v.to_srgb_u8();
            let color = Color32::from_rgba_unmultiplied(rgba.0, rgba.1, rgba.2, rgba.3);
            show_color_swatch(ui, color);
        }
        Value::Image { data, change_id: _ } => {
            // "×" and "·" instead of "x"/parentheses — reads less like a
            // math expression and more like a compact data readout.
            mono(ui, format!("{}×{} · {}ch", data.width(), data.height(), data.channels()));
        }
        Value::Path(p) => mono_truncate(ui, p.display().to_string()),
        Value::FilterType(ft) => mono(ui, filter_type_display_name(ft)),
        Value::ColorFormat(cf) => mono(ui, format!("{:?}", cf)),
        Value::ImageType(it) => mono(ui, format!("{:?}", it)),
        Value::Trigger => mono(ui, "trigger".to_string()),
        Value::NoiseWorleyDistanceFunction(v) => mono(ui, format!("{:?}", v)),
        Value::ColorSpace(v) => mono(ui, format!("{:?}", v)),
        Value::BlendMode(v) => mono(ui, format!("{:?}", v)),
        Value::EdgeMode(v) => mono(ui, format!("{:?}", v)),
        Value::TextHAlign(v) => mono(ui, format!("{:?}", v)),
        Value::TextVAlign(v) => mono(ui, format!("{:?}", v)),
        Value::ExportPreset(v) => mono(ui, format!("{:?}", v)),
        Value::Curve(v) => mono(ui, curve_summary(v)),
    }
}


/// Render an interactive input widget appropriate for the value type.
/// Connected inputs show a read-only label; disconnected inputs show the full editor.
fn input_value(ui: &mut egui::Ui, value: Value, input: &mut Input, input_index: usize, node_id: &str, tx_change_node: &Sender<ChangeNodeMessage>, sibling_image_format: Option<image::ImageFormat>, theme: &Theme, default_dir: Option<&std::path::Path>) {
    // Size value widgets to fill the value column (name | value | expose).
    //
    // We CANNOT derive the width from `available_width()`/`max_rect()`/
    // `clip_rect()` here: this cell lives in an egui_extras `Column::remainder()`,
    // whose reported width ratchets upward a few pixels every frame (a feedback
    // loop with the table's persisted `max_used_widths` — measured climbing
    // 137 → 160 → … → 1000+ px), which is what let the value widgets creep off
    // the panel and swallow the expose column. The cell's *left* edge is stable
    // though (only the width grows, rightward), so combine it with the value
    // column's right edge (captured from the panel width before the table) to
    // get the true column width. Keeping widgets within this bound also stops
    // the ratchet: their content no longer exceeds the column, so it unwinds.
    // The caller has pinned this cell to the true value-column width (see the
    // note in `show`), so `available_width()` is now reliable here — no longer
    // the ratcheting value egui_extras reports for a remainder cell. Every
    // value widget just fills it, so their right edges line up on the column's
    // right boundary:
    //   - TextEdit: `desired_width` is the widget's full outer width → `avail`.
    //   - ComboBox: total width == `combo_width` (its internal padding cancels).
    //   - Slider: laid out as [track][gap][value box], where the value box (a
    //     DragValue) is `interact_size.x` wide for small numbers — so sizing the
    //     track to `avail - gap - value_box` puts the value box's right edge on
    //     the boundary.
    let avail = ui.available_width();
    let gap = ui.spacing().item_spacing.x;
    // Widen the slider's value box a little so multi-digit numbers (e.g. "0.50")
    // aren't cramped. The Slider draws its value as a DragValue sized to
    // `interact_size.x`, so bump that and subtract the same width from the track
    // to keep the value box's right edge on the column boundary.
    let value_box_w = ui.spacing().interact_size.x + 16.0;
    ui.spacing_mut().interact_size.x = value_box_w;
    ui.spacing_mut().slider_width = (avail - gap - value_box_w).max(40.0);
    ui.spacing_mut().combo_width = avail;
    let text_width = avail;

    match value {
        Value::Bool(a) => {
            if matches!(input.settings, Some(InputSettings::Button)) {
                // Momentary action button (e.g. the output nodes' "save"). A
                // click fires a one-shot Bool(true) pulse the operation consumes.
                if input.connection.is_some() {
                    ui.label(a.to_string());
                } else if ui.add(egui::Button::new(input.name.clone())).clicked() {
                    change_value(tx_change_node, node_id, input_index, input, Value::Bool(true));
                }
            } else if input.connection.is_some() {
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
                // Truncate: a connected text value can be arbitrarily long
                // (e.g. a template result), and the clipped value column
                // needs the label to ellipsize rather than force it open.
                ui.add(Label::new(a).truncate());
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
            // Image inputs have no inline editor (there's nothing to type —
            // the value only ever arrives via a connection), but a connected
            // socket used to render nothing at all here, leaving the value
            // column blank. Show a quiet presence indicator instead so it's
            // clear the socket is wired up.
            if input.connection.is_some() {
                // Even fainter than `text_faint` (~60% alpha) since this is
                // just a presence indicator, not a value worth reading —
                // matches the design's contrast between its "connected" gray
                // (#414D4E) and its "value" gray (#528086, ~= text_faint).
                let faint = theme.get().text_faint.gamma_multiply(0.6);
                ui.label(RichText::new("connected").color(faint));
            }
        }
        Value::Path(path) => {
            if input.connection.is_some() {
                // Truncate: a connected path can be long, and the clipped
                // value column needs the label to ellipsize rather than
                // force it open.
                ui.add(Label::new(path.into_os_string().into_string().unwrap()).truncate());
            } else {
                // Right-to-left inside a `col_w`-wide region: pin the folder
                // button to the column's right edge, then let the (disabled)
                // path field fill the remaining width. This lines the button's
                // right edge up with the other value widgets instead of leaving
                // a variable gap.
                ui.allocate_ui_with_layout(
                    vec2(avail, ui.available_height()),
                    Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        let picked = ui.button("🗀").clicked();
                        // Remaining width, minus the spacing egui inserts before
                        // the field, so field + gap + button == col_w exactly.
                        let field_w = (ui.available_width() - gap).max(40.0);
                        ui.add_enabled_ui(false, |ui| {
                            ui.add(
                                TextEdit::singleline(
                                    &mut path.clone().into_os_string().into_string().unwrap_or_default(),
                                )
                                .desired_width(field_w),
                            );
                        });

                        if picked {
                            if let Some(InputSettings::Path {
                                extension_filter,
                                set_directory,
                                set_file_name,
                                set_title,
                                file_dialog_type
                            }) = input.settings.clone() {

                                let extensions: Vec<&str> = extension_filter.iter().map(|s| s.as_str()).collect();
                                let title = set_title.unwrap_or("file".to_string());
                                let mut file_dialog = rfd::FileDialog::new().add_filter(&title, &extensions);

                                // Starting directory: an explicit per-node
                                // `set_directory` wins; otherwise fall back to
                                // the current graph's folder (if it has one).
                                if let Some(dir) = set_directory.as_ref() {
                                    file_dialog = file_dialog.set_directory(dir);
                                } else if let Some(dir) = default_dir {
                                    file_dialog = file_dialog.set_directory(dir);
                                }
                                // Pre-fill the file name if the input asks for one.
                                if let Some(file_name) = set_file_name.as_ref() {
                                    file_dialog = file_dialog.set_file_name(file_name);
                                }

                                if let Some(save_path) = match file_dialog_type {
                                    mangler_core::input::FileDialogType::PickFile => file_dialog.pick_file(),
                                    mangler_core::input::FileDialogType::PickFolder => file_dialog.pick_folder(),
                                    mangler_core::input::FileDialogType::SaveFile => file_dialog.save_file(),
                                } {
                                    change_value(tx_change_node, node_id, input_index, input, Value::Path(save_path));
                                }
                            }
                        }
                    },
                );
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
        Value::EdgeMode(a) => {
            if input.connection.is_some() {
                ui.label(format!("{:?}", a));
            } else {
                let variants = EdgeMode::types();
                show_enum_combo(
                    ui, "edge", a, &variants,
                    |v| format!("{:?}", v),
                    input, input_index, node_id, tx_change_node,
                    |v| Value::EdgeMode(*v),
                );
            }
        }
        Value::ExportPreset(a) => {
            if input.connection.is_some() {
                ui.label(format!("{:?}", a));
            } else {
                let variants = ExportPreset::types();
                show_enum_combo(
                    ui, "export preset", a, &variants,
                    |v| format!("{:?}", v),
                    input, input_index, node_id, tx_change_node,
                    |v| Value::ExportPreset(*v),
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
        Value::Curve(a) => {
            // No inline editor — a curve is drawn in the 2D preview overlay.
            // Show a quiet summary; hint where to edit when it's unconnected.
            let faint = theme.get().text_faint;
            let resp = ui.add(Label::new(RichText::new(curve_summary(&a)).color(faint)));
            if input.connection.is_none() {
                resp.on_hover_text("drawn in the 2D preview panel");
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
