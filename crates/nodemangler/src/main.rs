#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use mangler::graph::Graph;
use mangler::nodes::*;
use mangler::value::Value;
use eframe::egui;

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
            // ui.heading("My egui Application");
            // ui.horizontal(|ui| {
            //     let name_label = ui.label("Your name: ");
            //     ui.text_edit_singleline(&mut self.name)
            //         .labelled_by(name_label.id);
            // });
            // ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
            // if ui.button("Click each year").clicked() {
            //     self.age += 1;
            // }
            // ui.label(format!("Hello '{}', age {}", self.name, self.age));
            // egui::TopBottomPanel::top("top_panel")
            //     .resizable(true)
            //     .min_height(32.0)
            //     .show_inside(ui, |ui| {
            //         egui::ScrollArea::vertical().show(ui, |ui| {
            //             ui.vertical_centered(|ui| {
            //                 ui.heading("Expandable Upper Panel");
            //             });
            //             //lorem_ipsum(ui);
            //         });
            //     });

            egui::SidePanel::left("left_panel")
                .default_width(150.0)
                .resizable(false)
                //.default_width(150.0)
                //.width_range(80.0..=200.0)
                .show_inside(ui, |ui| {
                    // ui.vertical_centered(|ui| {
                    //     ui.heading("Left Panel");
                    // });
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        //lorem_ipsum(ui);
                    });
                });

            // egui::SidePanel::right("right_panel")
            //     .resizable(true)
            //     .default_width(150.0)
            //     .width_range(80.0..=200.0)
            //     .show_inside(ui, |ui| {
            //         ui.vertical_centered(|ui| {
            //             ui.heading("Right Panel");
            //         });
            //         egui::ScrollArea::vertical().show(ui, |ui| {
            //             //lorem_ipsum(ui);
            //         });
            //     });

            egui::TopBottomPanel::bottom("bottom_panel")
                .resizable(true)
                //.min_height(50.0)
                .default_height(WINDOW_HEIGHT / 2.0)
                .show_inside(ui, |ui| {
                    // ui.vertical_centered(|ui| {
                    //     ui.heading("Bottom Panel");
                    // });
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
    pub position: egui::Vec2
}

impl GraphNode {
    pub fn show(self, ui: &mut egui::Ui) {
        ui.child_ui_with_id_source(
            egui::Rect::from_min_size(
                egui::Pos2::new(0.0, 0.0),
                MIN_NODE_SIZE.into(),
            ),
            egui::Layout::default(),
            0
        ).vertical(ui, |ui| {
            ui.add(Label::new(
                RichText::new(&self.graph[self.node_id].label)
                    .text_style(TextStyle::Button)
                    .color(text_color),
            ));
            ui.add_space(8.0);
        });
    }
}