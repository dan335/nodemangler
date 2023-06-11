use eframe::egui;
use epaint::{emath::Align2, Color32, FontId, PathShape, Pos2, Rect, Rounding, Stroke, Vec2};
use mangler::operation::Operation;
use mangler::OperationListItem;

use crate::theme::Theme;

const BUTTON_HEIGHT: f32 = 36.0;

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
                MenuItem::SubgraphButton { name: "Subgraph".to_string(), level }
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
                let button_top_position =
                    container_rect.top() + (BUTTON_HEIGHT * index as f32);
                let button_min = Pos2::new(container_rect.left(), button_top_position);
                let button_max =
                    Pos2::new(container_rect.right(), button_top_position + BUTTON_HEIGHT);
                let button_rect = Rect::from_two_pos(button_min, button_max);
                let rounding = Rounding::same(2.0);

                ui.centered_and_justified(|ui| {
                    let rect = Rect::from_min_max(
                        button_rect.min,
                        Pos2::new(button_rect.max.x, button_rect.max.y),
                    );

                    let response = ui.allocate_rect(rect, egui::Sense::hover());

                    let mut background_color = theme.node_menu_bg;
                    if response.hovered() {
                        background_color = theme.node_menu_bg_hover;
                    }

                    ui.painter().add(egui::Shape::rect_filled(
                        rect.shrink(1.0),
                        rounding,
                        background_color,
                    ));

                    // let mut points: Vec<Pos2> = Vec::with_capacity(2);
                    // points.push(Pos2::new(rect.left(), rect.top()));
                    // points.push(Pos2::new(rect.right(), rect.top()));
                    // ui.painter().add(egui::Shape::line(points.clone(), stroke));

                    // points.clear();
                    // points.push(Pos2::new(rect.left(), rect.bottom() + 1.0));
                    // points.push(Pos2::new(rect.right(), rect.bottom() + 1.0));
                    // ui.painter().add(egui::Shape::line(points, stroke));

                    let mut offset = Vec2::new(*level as f32 * 18.0, 0.0);

                    let mut points: Vec<Pos2> = Vec::new();

                    if *is_collapsed {
                        points.push(rect.left_center() + Vec2::new(10.0, -5.0) + offset);
                        points.push(rect.left_center() + Vec2::new(15.0, 0.0) + offset);
                        points.push(rect.left_center() + Vec2::new(10.0, 5.0) + offset);
                    } else {
                        points.push(rect.left_center() + Vec2::new(5.0, 0.0) + offset);
                        points.push(rect.left_center() + Vec2::new(15.0, 0.0) + offset);
                        points.push(rect.left_center() + Vec2::new(10.0, 5.0) + offset);
                    }

                    let triangle =
                        PathShape::convex_polygon(points, Color32::from(theme.override_text_color), Stroke::new(1.0, theme.override_text_color));

                    ui.painter().add(triangle);

                    offset.x += 25.0;

                    ui.painter().text(
                        Pos2::new(rect.left() + offset.x, rect.center().y),
                        Align2::LEFT_CENTER,
                        name.clone(),
                        FontId::default(),
                        Color32::from(theme.override_text_color),
                    );

                    let response = ui.allocate_rect(rect, egui::Sense::click());

                    if response.clicked() {
                        *is_collapsed = !(*is_collapsed);
                    }

                    
                });

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
                let button_top_position =
                    container_rect.top() + (BUTTON_HEIGHT * index as f32);
                let button_min = Pos2::new(container_rect.left(), button_top_position);
                let button_max =
                    Pos2::new(container_rect.right(), button_top_position + BUTTON_HEIGHT);
                let button_rect = Rect::from_two_pos(button_min, button_max);
                let rounding = Rounding::same(2.0);

                ui.centered_and_justified(|ui| {
                    //ui.centered(|ui| {

                    let rect = Rect::from_min_max(
                        button_rect.min,
                        Pos2::new(button_rect.max.x, button_rect.max.y),
                    );

                    let response =
                        ui.allocate_rect(rect, egui::Sense::drag().union(egui::Sense::hover()));

                    let mut background_color = theme.node_menu_bg;
                    if response.hovered() {
                        background_color = theme.node_menu_bg_hover;
                    }

                    ui.painter().add(egui::Shape::rect_filled(
                        rect.shrink(1.0),
                        rounding,
                        background_color,
                    ));

                    // let mut points: Vec<Pos2> = Vec::with_capacity(2);
                    // points.push(Pos2::new(rect.left(), rect.top()));
                    // points.push(Pos2::new(rect.right(), rect.top()));
                    // ui.painter().add(egui::Shape::line(points.clone(), stroke));

                    // points.clear();
                    // points.push(Pos2::new(rect.left(), rect.bottom() + 1.0));
                    // points.push(Pos2::new(rect.right(), rect.bottom() + 1.0));
                    // ui.painter().add(egui::Shape::line(points, stroke));

                    let indention = *level as f32 * 25.0;

                    ui.painter().text(
                        Pos2::new(rect.left() + indention, rect.center().y),
                        Align2::LEFT_CENTER,
                        name,
                        FontId::default(),
                        Color32::from(theme.override_text_color),
                    );

                    if response.clicked() {
                    } else if response.drag_started() {
                        result.operation_being_created = Some(operation.clone());
                    } else if response.drag_released() {
                    }
                });

                (index, result)
            }
            MenuItem::SubgraphButton { name, level } => {
                let container_rect = ui.max_rect();
                let button_top_position =
                    container_rect.top() + (BUTTON_HEIGHT * index as f32);
                let button_min = Pos2::new(container_rect.left(), button_top_position);
                let button_max =
                    Pos2::new(container_rect.right(), button_top_position + BUTTON_HEIGHT);
                let button_rect = Rect::from_two_pos(button_min, button_max);
                let rounding = Rounding::same(2.0);

                ui.centered_and_justified(|ui| {
                    //ui.centered(|ui| {

                    let rect = Rect::from_min_max(
                        button_rect.min,
                        Pos2::new(button_rect.max.x, button_rect.max.y),
                    );

                    let response =
                        ui.allocate_rect(rect, egui::Sense::drag().union(egui::Sense::hover()));

                    let mut background_color = theme.node_menu_bg;
                    if response.hovered() {
                        background_color = theme.node_menu_bg_hover;
                    }

                    ui.painter().add(egui::Shape::rect_filled(
                        rect.shrink(1.0),
                        rounding,
                        background_color,
                    ));

                    let indention = *level as f32 * 25.0;

                    ui.painter().text(
                        Pos2::new(rect.left() + indention, rect.center().y),
                        Align2::LEFT_CENTER,
                        name,
                        FontId::default(),
                        Color32::from(theme.override_text_color),
                    );

                    if response.clicked() {
                    } else if response.drag_started() {
                        result.subgraph_being_created = true;
                    } else if response.drag_released() {
                    }
                });

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
