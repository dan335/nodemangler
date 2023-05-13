use eframe::{egui, epaint::Rounding};
use egui::Pos2;

use mangler::nodes::{node_settings::NodeSettings, operation::ConnectionSettings, self};

use super::menu_button::MenuButton;

pub struct MenuPanel {
    pub buttons: Vec<MenuButton>
}

impl MenuPanel {
    pub fn new() -> MenuPanel {
        let mut buttons: Vec<MenuButton> = vec![];

        buttons.push(MenuButton::new(
            nodes::float::SETTINGS.clone(),
            nodes::float::INPUT_SETTINGS.clone(),
            nodes::float::OUTPUT_SETTINGS.clone(),
        ));
        buttons.push(MenuButton::new(
            nodes::integer::SETTINGS.clone(),
            nodes::integer::INPUT_SETTINGS.clone(),
            nodes::integer::OUTPUT_SETTINGS.clone(),
        ));
        buttons.push(MenuButton::new(
            nodes::add::SETTINGS.clone(),
            nodes::add::INPUT_SETTINGS.clone(),
            nodes::add::OUTPUT_SETTINGS.clone(),
        ));
        buttons.push(MenuButton::new(
            nodes::subtract::SETTINGS.clone(),
            nodes::subtract::INPUT_SETTINGS.clone(),
            nodes::subtract::OUTPUT_SETTINGS.clone(),
        ));
        
        MenuPanel {
            buttons
        }
    }


    pub fn show(&mut self, ui: &mut egui::Ui, cursor_position: Pos2) -> MenuResult {
        ui.painter().add(egui::Shape::rect_filled(
            ui.max_rect(),
            Rounding::none(),
            egui::Color32::from_gray(40),
        ));
        
        let mut dragging_menu_button: Option<(NodeSettings, Vec<ConnectionSettings>, Vec<ConnectionSettings>)> = None;

        for (menu_button_index, menu_button) in self.buttons.iter_mut().enumerate() {
            let menu_button_result = menu_button.show(ui, menu_button_index);

            if menu_button_result.is_dragging {
                dragging_menu_button = Some((menu_button.node_settings.clone(), menu_button.input_settings.clone(), menu_button.output_settings.clone()));
            }
        }

        MenuResult {
            dragging_menu_button
        }
    }
}


pub struct MenuResult {
    pub dragging_menu_button: Option<(NodeSettings, Vec<ConnectionSettings>, Vec<ConnectionSettings>)>,
}