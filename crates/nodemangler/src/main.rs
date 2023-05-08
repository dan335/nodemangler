#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use eframe::epaint::Rounding;
use mangler::graph::Graph;
use mangler::nodes::*;
use mangler::value::Value;

mod graph;
use graph::graph_node::GraphNode;
use graph::graph_editor::GraphEditor;

pub const WINDOW_WIDTH: f32 = 1280.0;
pub const WINDOW_HEIGHT: f32 = 800.0;


fn main() -> Result<(), eframe::Error> {
    // let mut graph = Graph::new();

    // let id = add::Add::new(&mut graph);

    // if let Some(node) = graph.nodes.get_mut(&id) {
    //     node.set_intput_value(0, Value::Decimal { value: 5.0 });
    // }

    // graph.run();

    // if let Some(v) = graph.nodes.get(&id) {
    //     println!("Hello, world! {:?}", v.print_output());
    // }

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(WINDOW_WIDTH, WINDOW_HEIGHT)),
        ..Default::default()
    };

    eframe::run_native(
        "Node Mangler",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

struct MyApp {
    graph_editor: GraphEditor
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            graph_editor: GraphEditor::new()
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        egui::CentralPanel::default().show(ctx, |ui| {

            egui::SidePanel::left("left_panel")
                .default_width(200.0)
                .resizable(false)
                .show_inside(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        if ui.add(egui::Button::new("asdf")).clicked() {
                            println!("clicked");
                        }
                    });
                });

            egui::TopBottomPanel::bottom("bottom_panel")
                .resizable(true)
                //.min_height(50.0)
                .default_height(WINDOW_HEIGHT / 2.0)
                .show_inside(ui, |ui| {
                    self.graph_editor.show(ui);
                });

            egui::CentralPanel::default().show_inside(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Central Panel");
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                    //lorem_ipsum(ui);
                });
            });
        });
    }
}

