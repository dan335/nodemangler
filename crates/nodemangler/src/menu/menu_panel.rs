use std::vec;

use eframe::{egui, epaint::Rounding};
use mangler::nodes::{operations::{float, integer, add, subtract, image_from_url, image_resize, image_from_clipboard, text_from_clipboard}, operation::{Operation, ConnectionSettings}, node_settings::NodeSettings};

use super::{menu_button::MenuButton, menu_category::MenuCategory};

pub struct MenuPanel {
    pub buttons: Vec<MenuCategory>,
}

impl MenuPanel {
    pub fn new() -> MenuPanel {
        let buttons= vec![
            MenuCategory::new("Numbers", vec![
                MenuButton {
                    node_settings: float::SETTINGS.clone(),
                    input_settings: float::INPUT_SETTINGS.clone(),
                    output_settings: float::OUTPUT_SETTINGS.clone(),
                    operation: Operation::Float,
                },
                MenuButton {
                    node_settings: integer::SETTINGS.clone(),
                    input_settings: integer::INPUT_SETTINGS.clone(),
                    output_settings: integer::OUTPUT_SETTINGS.clone(),
                    operation: Operation::Integer,
                },
                MenuButton {
                    node_settings: add::SETTINGS.clone(),
                    input_settings: add::INPUT_SETTINGS.clone(),
                    output_settings: add::OUTPUT_SETTINGS.clone(),
                    operation: Operation::Add,
                },
                MenuButton {
                    node_settings: subtract::SETTINGS.clone(),
                    input_settings: subtract::INPUT_SETTINGS.clone(),
                    output_settings: subtract::OUTPUT_SETTINGS.clone(),
                    operation: Operation::Subtract,
                },
            ]),
            MenuCategory::new("Images", vec![
                MenuButton {
                    node_settings: image_from_url::SETTINGS.clone(),
                    input_settings: image_from_url::INPUT_SETTINGS.clone(),
                    output_settings: image_from_url::OUTPUT_SETTINGS.clone(),
                    operation: Operation::ImageFromUrl,
                },
                MenuButton {
                    node_settings: image_resize::SETTINGS.clone(),
                    input_settings: image_resize::INPUT_SETTINGS.clone(),
                    output_settings: image_resize::OUTPUT_SETTINGS.clone(),
                    operation: Operation::ImageResize,
                },
                MenuButton {
                    node_settings: image_from_clipboard::SETTINGS.clone(),
                    input_settings: image_from_clipboard::INPUT_SETTINGS.clone(),
                    output_settings: image_from_clipboard::OUTPUT_SETTINGS.clone(),
                    operation: Operation::ImageFromClipboard,
                },
            ]),
            MenuCategory::new("Text", vec![
                MenuButton {
                    node_settings: text_from_clipboard::SETTINGS.clone(),
                    input_settings: text_from_clipboard::INPUT_SETTINGS.clone(),
                    output_settings: text_from_clipboard::OUTPUT_SETTINGS.clone(),
                    operation: Operation::TextFromClipboard,
                },
            ]), 
        ];

        MenuPanel { buttons }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> MenuResult {
        ui.painter().add(egui::Shape::rect_filled(
            ui.max_rect(),
            Rounding::none(),
            egui::Color32::from_gray(40),
        ));

        let mut dragging_menu_button: Option<(
            NodeSettings,
            Vec<ConnectionSettings>,
            Vec<ConnectionSettings>,
            Operation,
        )> = None;

        for (index, menu_category) in self.buttons.iter_mut().enumerate() {
            ui.centered_and_justified(|ui| {

            });
        }

        for (menu_button_index, menu_button) in self.buttons.iter_mut().enumerate() {
            let menu_button_result = menu_button.show(ui, menu_button_index);

            if menu_button_result.is_dragging {
                dragging_menu_button = Some((
                    menu_button.node_settings.clone(),
                    menu_button.input_settings.clone(),
                    menu_button.output_settings.clone(),
                    menu_button.operation.clone(),
                ));
            }
        }

        MenuResult {
            dragging_menu_button,
        }
    }
}

pub struct MenuResult {
    pub dragging_menu_button: Option<(
        NodeSettings,
        Vec<ConnectionSettings>,
        Vec<ConnectionSettings>,
        Operation,
    )>,
}