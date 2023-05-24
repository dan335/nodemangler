use eframe::{egui};
use mangler::{operation::{Operation, ConnectionSettings}, node_settings::NodeSettings};

use super::{menu_category::MenuCategory};
use mangler::OPERATION_LIST;

pub struct MenuPanel {
    pub buttons: Vec<MenuCategory>,
}

impl MenuPanel {
    pub fn new() -> MenuPanel {
        let mut buttons: Vec<MenuCategory> = Vec::new();

        for category in OPERATION_LIST.iter() {
            buttons.push(MenuCategory::new(category));
        }

        MenuPanel { buttons }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> MenuResult {
        let mut dragging_menu_button: Option<(
            NodeSettings,
            Vec<ConnectionSettings>,
            Vec<ConnectionSettings>,
            Operation,
        )> = None;
        
        egui::ScrollArea::vertical().show(ui, |ui| {

            // ui.painter().add(egui::Shape::rect_filled(
            //     ui.max_rect(),
            //     Rounding::none(),
            //     egui::Color32::from_gray(40),
            // ));
    
            
    
            let mut index = 0;
            for (category_index, category) in self.buttons.iter_mut().enumerate() {
                category.show(ui, index);
                index += 1;
    
                if !category.is_collapsed {
                    for (button_index, menu_button) in category.buttons.iter_mut().enumerate() {
                        let menu_button_result = menu_button.show(ui, index);
        
                        if menu_button_result.is_dragging {
                            dragging_menu_button = Some((
                                menu_button.node_settings.clone(),
                                menu_button.input_settings.clone(),
                                menu_button.output_settings.clone(),
                                menu_button.operation.clone(),
                            ));
                        }
    
                        index += 1;
                    }
                }
            }
        });
        

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