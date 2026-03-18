use eframe::egui;

use crate::themes::theme::Theme;

use super::menu_item::{MenuItem, MenuItemsResult};

pub struct MenuPanel {
    pub items: Vec<MenuItem>,
}

impl MenuPanel {
    pub fn new() -> MenuPanel {
        let mut items: Vec<MenuItem> = Vec::new();
        let level = 0;

        for op in mangler_core::operations::operation_list().iter() {
            let item = MenuItem::new(op.clone(), level);
            items.push(item);
        }

        MenuPanel { items }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, theme: &Theme) -> MenuItemsResult {
        let mut menu_result = MenuItemsResult::default();
        let mut index = -1;

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(5.0);
            
            for item in self.items.iter_mut() {
                let (i, result) = item.show(ui, index, theme);
                index = i;

                if result.operation_being_created.is_some() {
                    menu_result.operation_being_created = result.operation_being_created;
                }

                if result.subgraph_being_created {
                    menu_result.subgraph_being_created = true;
                }
            }
        });

        menu_result
    }
}

// pub struct MenuResult {
//     pub dragging_menu_button: Option<Operation>,
// }
