use eframe::egui;
use epaint::Vec2;
use mangler::operations::Operation;
use mangler::operations::OperationListItem;

use crate::themes::theme::Theme;

#[derive(Debug)]
pub enum MenuItem {
    Category {
        name: String,
        level: usize,
        is_collapsed: bool,
        items: Vec<MenuItem>,
    },
    OperationButton {
        name: String,
        level: usize,
        operation: Operation,
    },
    SubgraphButton {
        name: String,
        level: usize,
    }
}

impl MenuItem {
    pub fn new(operation_item: OperationListItem, level: usize) -> MenuItem {
        match operation_item {
            OperationListItem::Category {
                name,
                operation_list_items,
            } => {
                let mut items: Vec<MenuItem> = Vec::new();

                for item in operation_list_items.iter() {
                    items.push( MenuItem::new(item.clone(), level + 1));
                }

                MenuItem::Category {
                    name,
                    items,
                    is_collapsed: true,
                    level,
                }
            }
            OperationListItem::Operation { operation } => {
                MenuItem::OperationButton {
                    name: operation.settings().name,
                    operation,
                    level,
                }
            }
            OperationListItem::Subgraph => {
                MenuItem::SubgraphButton { name: "subgraph".to_string(), level }
            },
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, mut index: i32, theme: &Theme) -> (i32, MenuItemsResult) {
        let mut result = MenuItemsResult {
            operation_being_created: None,
            subgraph_being_created: false,
        };

        index += 1;

        match self {
            MenuItem::Category {
                name,
                items,
                is_collapsed,
                level,
            } => {
                let container_rect = ui.max_rect();

                let mut icon = egui_phosphor::CARET_DOWN;

                if *is_collapsed {
                    icon = egui_phosphor::CARET_RIGHT;
                }

                if ui.add(egui::Button::new(egui::RichText::new(format!("    {} {}  {}", " ".repeat(*level * 8), icon, name)).color(theme.get().text_faint).size(15.0)).frame(false).min_size(Vec2::new(container_rect.width(), 24.0))).clicked() {
                    *is_collapsed = !(*is_collapsed);
                }

                if !(*is_collapsed) {
                    for item in items.iter_mut() {
                        let (i, r) = item.show(ui, index, theme);
                        index = i;

                        if let Some(operation_being_created) = r.operation_being_created {
                            result.operation_being_created = Some(operation_being_created);
                        }
                    }
                }
                

                (index, result)
            }

            MenuItem::OperationButton {
                name,
                operation,
                level,
            } => {
                let container_rect = ui.max_rect();

                if ui.add(egui::Button::new(egui::RichText::new(format!("    {} {}", " ".repeat(*level * 8), name)).size(15.0)).frame(false).min_size(Vec2::new(container_rect.width(), 24.0))).interact(egui::Sense::drag()).drag_started() {
                    result.operation_being_created = Some(operation.clone());
                }


                (index, result)
            }
            MenuItem::SubgraphButton { name, level } => {
                let container_rect = ui.max_rect();

                if ui.add(egui::Button::new(egui::RichText::new(format!("    {} {}", " ".repeat(*level * 10), name)).size(15.0)).frame(false).min_size(Vec2::new(container_rect.width(), 24.0))).interact(egui::Sense::drag()).drag_started() {
                    result.subgraph_being_created = true;
                }

                (index, result)
            },
        }
    }
}

pub struct MenuItemsResult {
    pub operation_being_created: Option<Operation>,
    pub subgraph_being_created: bool,
}


impl Default for MenuItemsResult {
    fn default() -> Self {
        MenuItemsResult {
            operation_being_created: None,
            subgraph_being_created: false,
        }
    }
}
