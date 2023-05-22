use std::fmt::Debug;
use std::time::Duration;

use crate::graph::graph_input::draw_graph_input;
use crate::graph::graph_output::draw_graph_output;
use eframe::epaint::{Color32, FontId, Rounding};
use eframe::{egui, emath::Align2};
use egui::{Pos2, Rect, Vec2};
use mangler::input::Input;
use mangler::nodes::node_settings::NodeSettings;
use mangler::nodes::operation::ConnectionSettings;
use mangler::output::Output;
use mangler::value::Value;

use super::graph_editor::TempConnection;
use super::graph_output::draw_graph_output_highlighted;

pub const NODE_SIZE: Vec2 = Vec2::new(132.0, 132.0);
pub const THUMBNAIL_SIZE: [u32; 2] = [128, 128];
const NODE_ROUNDING: f32 = 2.0;

#[derive(Clone)]
pub struct GraphNode {
    pub id: String,
    position: egui::Pos2,
    pub settings: NodeSettings,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub time: Option<Duration>,
    is_dragging: bool,
    last_drag_position: Option<Pos2>,
    pub thumbnail: Option<egui::TextureHandle>,
    //pub is_dirty: bool,  // thumnail and image need to update
}

impl GraphNode {
    pub fn new(
        id: String,
        position: Pos2,
        settings: NodeSettings,
        input_settings: Vec<ConnectionSettings>,
        output_settings: Vec<ConnectionSettings>,
    ) -> GraphNode {
        // same as in node.rs
        // todo: improve this?
        let inputs: Vec<Input> = input_settings
            .iter()
            // .map(|settings| Input {
            //     name: settings.name.to_owned(),
            //     value: settings.default_value.clone(),
            //     connection: None,
            //     valid_types: settings.valid_types.to_vec(),
            //     ui_type: settings.ui_type.clone(),
            // })
            .map(|settings| Input::new(settings.clone()))
            .collect();

        let outputs: Vec<Output> = output_settings
            .iter()
            .map(|settings| Output {
                name: settings.name.to_owned(),
                value: settings.default_value.clone(),
                connection: None,
                value_type: settings.default_value.value_type(),
            })
            .collect();

        GraphNode {
            id,
            position,
            settings,
            is_dragging: false,
            last_drag_position: None,
            thumbnail: None,
            //is_dirty: true,
            inputs,
            outputs,
            time: None, // what it is does not matter
        }
    }

    pub fn get_rect(&self, graph_position: Pos2) -> Rect {
        Rect::from_center_size(self.position + graph_position.to_vec2(), NODE_SIZE)
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        graph_position: Pos2,
        cursor_position: Pos2,
        is_editing: bool,
        is_viewing: bool,
    ) -> GraphNodeResponse {
        puffin::profile_scope!("graph node.show()");
        let mut graph_node_response = GraphNodeResponse::default();
        let rounding = Rounding::same(NODE_ROUNDING);

        let node_rect = self.get_rect(graph_position);

        let bg_response = ui.allocate_rect(
            node_rect,
            egui::Sense::click()
                .union(egui::Sense::drag())
                .union(egui::Sense::hover()),
        );

        if bg_response.clicked_by(egui::PointerButton::Primary) {
            self.stop_dragging();
            graph_node_response.is_left_click = true;
        } else if bg_response.clicked_by(egui::PointerButton::Secondary) {
            self.stop_dragging();
            graph_node_response.is_right_click = true;
        } else if bg_response.drag_started_by(egui::PointerButton::Primary) {
            self.start_dragging();
        } else if bg_response.drag_released_by(egui::PointerButton::Primary) {
            self.stop_dragging();
        }

        graph_node_response.is_cursor_inside = bg_response.hovered();

        if self.is_dragging {
            if let Some(last_drag_position) = self.last_drag_position {
                self.position += cursor_position - last_drag_position;
            }

            self.last_drag_position = Some(cursor_position);
        }

        // bg
        ui.painter().add(egui::Shape::rect_filled(
            node_rect,
            rounding,
            egui::Color32::from_gray(70),
        ));

        // ------------
        // inputs
        for (index, input) in self.inputs.iter().enumerate() {
            puffin::profile_scope!("graph node.inputs.iter()");
            // draw input
            let input_output_response = draw_graph_input(
                input,
                self.get_input_position(index, node_rect),
                self.get_input_rect(index, node_rect),
                index,
                node_rect,
                ui,
                bg_response.hovered(),
            );

            if input_output_response.has_started_creating_connection {
                graph_node_response.temp_connection = Some(TempConnection {
                    from_position: input_output_response.connection_from_position,
                    from_node_id: self.id.clone(),
                    from_connection_index: index,
                    from_connection_type: ConnectionType::Input,
                });
            }

            if input_output_response.has_stopped_creating_connection {
                graph_node_response.has_stopped_creating_connection = true;
                graph_node_response.connection_to_position =
                    input_output_response.connection_to_position;
            }

            if input_output_response.is_cursor_over {
                graph_node_response.is_cursor_inside = true;
            }
        }

        // outputs
        for (index, output) in self.outputs.iter().enumerate() {
            puffin::profile_scope!("graph node.outputs.iter()");
            let input_output_response = draw_graph_output(
                &output.name,
                &output.value.value_name(),
                self.get_output_position(index, node_rect),
                self.get_output_rect(index, node_rect),
                index,
                node_rect,
                ui,
                bg_response.hovered(),
            );

            // started dragging from connection
            // create temp connection object
            if input_output_response.has_started_creating_connection {
                graph_node_response.temp_connection = Some(TempConnection {
                    from_position: input_output_response.connection_from_position,
                    from_node_id: self.id.clone(),
                    from_connection_index: index,
                    from_connection_type: ConnectionType::Output,
                });
            }

            if input_output_response.has_stopped_creating_connection {
                graph_node_response.has_stopped_creating_connection = true;
                graph_node_response.connection_to_position =
                    input_output_response.connection_to_position;
            }

            if is_viewing && index == 0 {
                draw_graph_output_highlighted(self.get_output_position(index, node_rect), ui);
            }

            if input_output_response.is_cursor_over {
                graph_node_response.is_cursor_inside = true;
            }
        }

        // ms
        if let Some(time) = self.time {
            puffin::profile_scope!("graph node.inputs show time");
            let pos = Pos2 {
                x: node_rect.right_bottom().x,
                y: node_rect.right_bottom().y + 5.0,
            };
            let text = format!("{:.4} ms", time.as_nanos() as f64 / 1_000_000.0);
            ui.painter().text(
                pos,
                Align2::RIGHT_TOP,
                text,
                egui::FontId::monospace(10.0),
                egui::Color32::from_gray(200),
            );
        }

        // show output result on node
        if let Some(thumbnail) = &self.thumbnail {
            ui.painter().image(
                thumbnail.id(),
                Rect::from_center_size(
                    self.position + graph_position.to_vec2(),
                    thumbnail.size_vec2(),
                ),
                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            match &self.outputs[0].value {
                mangler::value::Value::Bool(value) => {
                    show_output_text(ui, node_rect.center(), value.to_string())
                }
                mangler::value::Value::Integer(value) => {
                    show_output_text(ui, node_rect.center(), value.to_string())
                }
                mangler::value::Value::Decimal(value) => {
                    show_output_text(ui, node_rect.center(), value.to_string())
                }
                mangler::value::Value::String(value) => {
                    show_output_text(ui, node_rect.center(), value.to_string())
                }

                mangler::value::Value::ImageRgba32F(_)
                | mangler::value::Value::ImageRgba8(_)
                | mangler::value::Value::ImageGray8(_)
                | mangler::value::Value::ImageRgb32F(_)
                | mangler::value::Value::ImageRgba16(_)
                | mangler::value::Value::ImageRgb16(_)
                | mangler::value::Value::ImageGrayA16(_)
                | mangler::value::Value::ImageGray16(_)
                | mangler::value::Value::ImageRgb8(_)
                | mangler::value::Value::ImageGrayA8(_) => {}

                mangler::value::Value::FilterType(value) => {
                    show_output_text(ui, node_rect.center(), format!("{:?}", value))
                }
                mangler::value::Value::ImageFormat(value) => {
                    show_output_text(ui, node_rect.center(), format!("{:?}", value))
                }
            }
        }

        fn show_output_text(ui: &mut egui::Ui, position: Pos2, txt: String) {
            puffin::profile_scope!("graph node.show_output_text()");
            ui.painter().text(
                position,
                Align2::CENTER_CENTER,
                txt,
                FontId::proportional(20.0),
                Color32::from_gray(200),
            );
        }

        // outline
        if is_editing {
            puffin::profile_scope!("graph node.show_is_editing");
            ui.painter().add(egui::Shape::rect_stroke(
                node_rect,
                rounding,
                egui::Stroke::new(4.0, Color32::from_rgb(30, 150, 90)),
            ));
        }

        //if is_viewing {
        //ui.painter().add(egui::Shape::rect_stroke(self.get_rect(graph_position).expand(10.0), rounding, egui::Stroke::new(2.0, Color32::GREEN)));
        //}
        // ui.painter().add(egui::Shape::rect_stroke(
        //     rect,
        //     rounding,
        //     stroke
        // ));

        // text - name
        ui.painter().text(
            Pos2::new(node_rect.center().x, node_rect.top() - 20.0),
            Align2::CENTER_TOP,
            self.settings.name.clone(),
            egui::FontId::default(),
            egui::Color32::from_gray(220),
        );

        graph_node_response
    }

    fn start_dragging(&mut self) {
        self.is_dragging = true;
    }

    fn stop_dragging(&mut self) {
        self.is_dragging = false;
        self.last_drag_position = None;
    }

    pub fn get_input_position(&self, index: usize, node_rect: Rect) -> Pos2 {
        puffin::profile_scope!("graph node.get_input_position()");
        Pos2::new(
            node_rect.left() - 14.0,
            node_rect.top() + 12.0 + 20.0 * index as f32,
        )
    }

    pub fn get_output_position(&self, index: usize, node_rect: Rect) -> Pos2 {
        puffin::profile_scope!("graph node.get_output_position()");
        Pos2::new(
            node_rect.right() + 14.0,
            node_rect.top() + 12.0 + 20.0 * index as f32,
        )
    }

    pub fn get_input_rect(&self, index: usize, node_rect: Rect) -> Rect {
        puffin::profile_scope!("graph node.get_input_rect()");
        Rect::from_center_size(
            self.get_input_position(index, node_rect),
            Vec2::new(12.0, 12.0),
        )
    }

    pub fn get_output_rect(&self, index: usize, node_rect: Rect) -> Rect {
        puffin::profile_scope!("graph node.get_output_rect()");
        Rect::from_center_size(
            self.get_output_position(index, node_rect),
            Vec2::new(12.0, 12.0),
        )
    }

    pub fn set_input_connection(
        &mut self,
        input_index: usize,
        output_id: String,
        output_index: usize,
    ) {
        puffin::profile_scope!("graph node.set_input_connection()");
        self.inputs[input_index].connection = Some((output_id, output_index));
    }

    pub fn clear_input_connection(&mut self, input_index: usize) {
        self.inputs[input_index].connection = None;
    }

    pub fn set_output_connection(
        &mut self,
        output_index: usize,
        input_id: String,
        input_index: usize,
    ) {
        puffin::profile_scope!("graph node.set_output_connection()");
        if self.outputs[output_index].connection.is_some() {
            self.outputs[output_index]
                .connection
                .as_mut()
                .unwrap()
                .push((input_id, input_index));
        } else {
            self.outputs[output_index].connection = Some(vec![(input_id, input_index)]);
        }
    }
}

#[derive(Debug)]
pub struct GraphNodeResponse {
    pub temp_connection: Option<TempConnection>,
    pub has_stopped_creating_connection: bool,
    pub connection_to_position: Pos2,
    pub edit_node: bool,
    pub view_node: bool,
    pub is_right_click: bool,
    pub is_left_click: bool,
    pub is_cursor_inside: bool,
}

impl GraphNodeResponse {
    pub fn default() -> GraphNodeResponse {
        GraphNodeResponse {
            temp_connection: None,
            has_stopped_creating_connection: false,
            connection_to_position: Pos2::ZERO,
            edit_node: false,
            view_node: false,
            is_right_click: false,
            is_left_click: false,
            is_cursor_inside: false,
        }
    }
}

pub struct InputOutputResponse {
    pub has_started_creating_connection: bool,
    pub connection_from_position: Pos2,
    pub has_stopped_creating_connection: bool,
    pub connection_to_position: Pos2,
    pub is_cursor_over: bool,
}

impl InputOutputResponse {
    pub fn new() -> InputOutputResponse {
        InputOutputResponse {
            has_started_creating_connection: false,
            connection_from_position: Pos2::ZERO,
            has_stopped_creating_connection: false,
            connection_to_position: Pos2::ZERO,
            is_cursor_over: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConnectionType {
    Input,
    Output,
}
