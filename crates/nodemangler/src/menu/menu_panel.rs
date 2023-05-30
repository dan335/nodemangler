use eframe::egui;
use mangler::operation::Operation;

use mangler::OPERATION_LIST;

use super::menu_item::MenuItem;

pub struct MenuPanel {
    pub items: Vec<MenuItem>,
}

impl MenuPanel {
    pub fn new() -> MenuPanel {
        let mut items: Vec<MenuItem> = Vec::new();
        let mut index = 0;

        for op in OPERATION_LIST.iter() {
            let (returned_index, item) = MenuItem::new(op.clone(), index);
            items.push(item);
            index = returned_index;
        }
        println!("{:?}", items);
        MenuPanel { items }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> MenuResult {
        let mut dragging_menu_button: Option<Operation> = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for item in self.items.iter_mut() {
                let result = item.show(ui);

                if let Some(operation_being_created) = result.operation_being_created {
                    dragging_menu_button = Some(operation_being_created);
                }
            }

            // ui.painter().add(egui::Shape::rect_filled(
            //     ui.max_rect(),
            //     Rounding::none(),
            //     egui::Color32::from_gray(40),
            // ));

            // let mut index = 0;
            // for (category_index, category) in self.buttons.iter_mut().enumerate() {
            //     category.show(ui, index);
            //     index += 1;

            //     if !category.is_collapsed {
            //         for (button_index, menu_button) in category.buttons.iter_mut().enumerate() {
            //             let menu_button_result = menu_button.show(ui, index);

            //             if menu_button_result.is_dragging {
            //                 dragging_menu_button = Some(menu_button.operation.clone());
            //             }

            //             index += 1;
            //         }
            //     }
            // }
        });

        MenuResult {
            dragging_menu_button,
        }
    }
}

pub struct MenuResult {
    pub dragging_menu_button: Option<Operation>,
}
