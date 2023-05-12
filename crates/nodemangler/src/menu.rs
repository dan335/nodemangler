use eframe::egui;
use egui::Pos2;

use mangler::nodes::{*, self, node_settings::NodeSettings};
use crate::menu_button::MenuButton;

pub struct Menu {
    pub buttons: Vec<MenuButton>
}

impl Menu {
    pub fn new() -> Menu {
        let mut buttons: Vec<MenuButton> = vec![];

        buttons.push(MenuButton::new(nodes::add::SETTINGS.clone()));
        buttons.push(MenuButton::new(nodes::subtract::SETTINGS.clone()));
        
        Menu {
            buttons
        }
    }


    pub fn show(&mut self, ui: &mut egui::Ui, cursor_position: Pos2) -> MenuResult {
        let mut dragging_menu_button: Option<NodeSettings> = None;

        for (menu_button_index, menu_button) in self.buttons.iter_mut().enumerate() {
            let menu_button_result = menu_button.show(ui, menu_button_index);

            if menu_button_result.is_dragging {
                dragging_menu_button = Some(menu_button.node_settings.clone());
            }
        }

        MenuResult {
            dragging_menu_button
        }
    }
}


pub struct MenuResult {
    pub dragging_menu_button: Option<NodeSettings>,
}