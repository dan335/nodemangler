#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use eframe::epaint::Rounding;
use mangler::graph::Graph;
use mangler::nodes::*;
use mangler::value::Value;

const WINDOW_WIDTH: f32 = 1280.0;
const WINDOW_HEIGHT: f32 = 800.0;
pub const MIN_NODE_SIZE: [f32; 2] = [200.0, 200.0];

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
        "My egui App",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

struct MyApp {
    name: String,
    age: u32,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            name: "Arthur".to_owned(),
            age: 42,
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
                    ui.set_clip_rect(ui.max_rect());
                    let r = ui.allocate_rect(
                        ui.min_rect(),
                        egui::Sense::click().union(egui::Sense::drag()),
                    );

                    if r.clicked() {
                        println!("clicked");
                    } else if r.drag_started() {
                        println!("drag started");
                    } else if r.drag_released() {
                        println!("drag released");
                    }

                    ui.vertical_centered(|ui| {
                        let node = GraphNode {
                            position: egui::Pos2::new(0.0, WINDOW_HEIGHT * 0.6),
                        };
                        node.show(ui);
                    });
                    let editor_rect = ui.max_rect();
                    ui.allocate_rect(editor_rect, egui::Sense::hover());
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

pub struct GraphNode {
    pub position: egui::Pos2,
}

impl GraphNode {
    pub fn show(self, ui: &mut egui::Ui) {
        ui.painter().add(egui::Shape::rect_filled(
            egui::Rect::from_two_pos(self.position, egui::pos2(200.0, 200.0)),
            Rounding::same(4.0),
            egui::Color32::from_gray(70),
        ));
        ui.child_ui(
            egui::Rect::from_min_max(egui::Pos2::new(0.0, 0.0), egui::Pos2::new(100.0, 100.0)),
            egui::Layout::default(),
        )
        .vertical(|ui| {
            ui.child_ui_with_id_source(
                egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), MIN_NODE_SIZE.into()),
                egui::Layout::default(),
                0,
            )
            .vertical(|ui| {
                ui.add(egui::Label::new(
                    egui::RichText::new("asdf")
                        .text_style(egui::TextStyle::Button)
                        .color(egui::Color32::RED),
                ));
            });
        });
    }
}
