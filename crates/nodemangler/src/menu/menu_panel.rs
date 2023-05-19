use std::vec;

use eframe::{egui, epaint::Rounding};
use mangler::nodes::{
    self,
    node_settings::NodeSettings,
    operation::{ConnectionSettings, Operation},
};

use super::menu_button::MenuButton;

pub struct MenuPanel {
    pub buttons: Vec<MenuButton>,
}

impl MenuPanel {
    pub fn new() -> MenuPanel {
        let buttons: Vec<MenuButton> = vec![
            MenuButton {
                node_settings: nodes::float::SETTINGS.clone(),
                input_settings: nodes::float::INPUT_SETTINGS.clone(),
                output_settings: nodes::float::OUTPUT_SETTINGS.clone(),
                operation: Operation::Float,
            },
            MenuButton {
                node_settings: nodes::integer::SETTINGS.clone(),
                input_settings: nodes::integer::INPUT_SETTINGS.clone(),
                output_settings: nodes::integer::OUTPUT_SETTINGS.clone(),
                operation: Operation::Integer,
            },
            MenuButton {
                node_settings: nodes::add::SETTINGS.clone(),
                input_settings: nodes::add::INPUT_SETTINGS.clone(),
                output_settings: nodes::add::OUTPUT_SETTINGS.clone(),
                operation: Operation::Add,
            },
            MenuButton {
                node_settings: nodes::subtract::SETTINGS.clone(),
                input_settings: nodes::subtract::INPUT_SETTINGS.clone(),
                output_settings: nodes::subtract::OUTPUT_SETTINGS.clone(),
                operation: Operation::Subtract,
            },
            MenuButton {
                node_settings: nodes::image_from_url::SETTINGS.clone(),
                input_settings: nodes::image_from_url::INPUT_SETTINGS.clone(),
                output_settings: nodes::image_from_url::OUTPUT_SETTINGS.clone(),
                operation: Operation::ImageFromUrl,
            },
            MenuButton {
                node_settings: nodes::image_resize::SETTINGS.clone(),
                input_settings: nodes::image_resize::INPUT_SETTINGS.clone(),
                output_settings: nodes::image_resize::OUTPUT_SETTINGS.clone(),
                operation: Operation::ImageResize,
            },
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
