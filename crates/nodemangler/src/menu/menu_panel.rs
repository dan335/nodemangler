use eframe::{egui};
use mangler::{operation::{Operation, ConnectionSettings}, node_settings::NodeSettings, OperationListItem};

use super::menu_item::MenuItem;
use mangler::OPERATION_LIST;

pub struct MenuPanel {
    pub items: Vec<OperationListItem>,
}

impl MenuPanel {
    pub fn new(&mut self) {
        // let mut items: Vec<MenuItem> = Vec::new();

        // for list_item in OPERATION_LIST.iter() {
        //     match list_item {
        //         mangler::OperationListItem::Category { name, operations } => {


        //             items.push(MenuItem::Category { name: name.clone(), items: vec![], is_collapsed: true })
        //         },
        //         mangler::OperationListItem::Operation { operation } => {
        //             items.push(MenuItem::Button { name: operation.settings().name, operation: operation.clone() })
        //         },
        //     }
        // }

        // MenuPanel { items }
        self.items = OPERATION_LIST.to_vec();
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> MenuResult {
        let mut dragging_menu_button: Option<Operation> = None;
        
        egui::ScrollArea::vertical().show(ui, |ui| {

            for (index, item) in self.items.iter_mut().enumerate() {
                let result = item.show(ui, index);

                match result {
                    super::menu_item::MenuItemResult::Category => {},
                    super::menu_item::MenuItemResult::Button { operation_being_created } => {
                        dragging_menu_button = operation_being_created;
                    },
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