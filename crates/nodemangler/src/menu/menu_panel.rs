use eframe::egui;
use mangler::operation::Operation;

use super::menu_item::MenuItem;

pub struct MenuPanel {
    pub items: Vec<MenuItem>,
}

impl MenuPanel {
    pub fn new() -> MenuPanel {
        let mut items: Vec<MenuItem> = Vec::new();
        let level = 0;

        for op in mangler::operation_list().iter() {
            let item = MenuItem::new(op.clone(), level);
            items.push(item);
        }

        MenuPanel { items }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> MenuResult {
        let mut dragging_menu_button: Option<Operation> = None;
        let mut index = -1;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for item in self.items.iter_mut() {
                let (i, result) = item.show(ui, index);
                index = i;

                if let Some(operation_being_created) = result.operation_being_created {
                    dragging_menu_button = Some(operation_being_created);
                }
            }
        });

        MenuResult {
            dragging_menu_button,
        }
    }
}

pub struct MenuResult {
    pub dragging_menu_button: Option<Operation>,
}
