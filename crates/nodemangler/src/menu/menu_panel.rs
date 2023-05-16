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
        let mut buttons: Vec<MenuButton> = vec![];

        buttons.push(MenuButton {
            node_settings: nodes::float::SETTINGS.clone(),
            input_settings: nodes::float::INPUT_SETTINGS.clone(),
            output_settings: nodes::float::OUTPUT_SETTINGS.clone(),
            operation: Box::new(nodes::float::Float {}),
        });

        buttons.push(MenuButton {
            node_settings: nodes::integer::SETTINGS.clone(),
            input_settings: nodes::integer::INPUT_SETTINGS.clone(),
            output_settings: nodes::integer::OUTPUT_SETTINGS.clone(),
            operation: Box::new(nodes::integer::Integer {}),
        });

        buttons.push(MenuButton {
            node_settings: nodes::add::SETTINGS.clone(),
            input_settings: nodes::add::INPUT_SETTINGS.clone(),
            output_settings: nodes::add::OUTPUT_SETTINGS.clone(),
            operation: Box::new(nodes::add::Add {}),
        });

        buttons.push(MenuButton {
            node_settings: nodes::subtract::SETTINGS.clone(),
            input_settings: nodes::subtract::INPUT_SETTINGS.clone(),
            output_settings: nodes::subtract::OUTPUT_SETTINGS.clone(),
            operation: Box::new(nodes::subtract::Subtract {}),
        });

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
            Box<dyn Operation>,
        )> = None;

        for (menu_button_index, menu_button) in self.buttons.iter_mut().enumerate() {
            let menu_button_result = menu_button.show(ui, menu_button_index);

            if menu_button_result.is_dragging {
                dragging_menu_button = Some((
                    menu_button.node_settings.clone(),
                    menu_button.input_settings.clone(),
                    menu_button.output_settings.clone(),
                    menu_button.operation,
                ));
            }
        }

        MenuResult {
            dragging_menu_button,
        }
    }
}

pub struct MenuResult {
    pub dragging_menu_button: Option<(NodeSettings, Vec<ConnectionSettings>, Vec<ConnectionSettings>, Box<dyn Operation>)>,
}
