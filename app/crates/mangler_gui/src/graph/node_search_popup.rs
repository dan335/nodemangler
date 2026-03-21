//! Quick node search popup for finding and adding nodes by name.
//!
//! Opens via Tab key on the graph canvas, or when a connection is dropped
//! onto empty space. Provides fuzzy substring search with keyboard navigation.

use eframe::egui;
use egui::Pos2;
use mangler_core::operations::{operation_list, Operation, OperationListItem};

use super::graph_editor::TempConnection;
use super::graph_node::ConnectionType;
use crate::themes::theme::Theme;

/// A flattened entry from the operation menu tree, ready for search/display.
#[derive(Clone)]
pub struct SearchResult {
    /// The operation variant to instantiate.
    pub operation: Operation,
    /// Display name from `settings().name`.
    pub name: String,
    /// Hierarchical category path, e.g. "images > noise".
    pub category_path: String,
}

/// The response from showing the popup this frame.
pub struct NodeSearchPopupResponse {
    /// If the user selected an operation, it's returned here.
    pub selected_operation: Option<Operation>,
    /// Whether the user selected the subgraph entry.
    pub selected_subgraph: bool,
    /// Whether the popup was closed (by Escape or clicking outside).
    pub closed: bool,
}

/// Floating search popup for quickly adding nodes to the graph.
pub struct NodeSearchPopup {
    /// Whether the popup is currently visible.
    pub is_open: bool,
    /// Current text in the search field.
    pub search_text: String,
    /// Screen-space position where the popup appears.
    pub position: Pos2,
    /// All operations flattened from the menu tree.
    all_results: Vec<SearchResult>,
    /// Currently filtered results based on search text and type filter.
    pub filtered_results: Vec<SearchResult>,
    /// Index of the keyboard-selected item in `filtered_results`.
    pub selected_index: usize,
    /// If opened from a dropped connection, stores the connection info for type filtering.
    pub from_connection: Option<TempConnection>,
    /// Whether the text field should request focus this frame.
    request_focus: bool,
}

impl NodeSearchPopup {
    /// Creates a new popup, flattening the operation list once.
    pub fn new() -> Self {
        let all_results = flatten_operations(&operation_list(), "");
        let filtered_results = all_results.clone();
        Self {
            is_open: false,
            search_text: String::new(),
            position: Pos2::ZERO,
            all_results,
            filtered_results,
            selected_index: 0,
            from_connection: None,
            request_focus: false,
        }
    }

    /// Opens the popup at the given screen position.
    ///
    /// If `from_connection` is provided, results will be type-filtered to
    /// operations compatible with the connection's value type.
    pub fn open(&mut self, position: Pos2, from_connection: Option<TempConnection>) {
        self.is_open = true;
        self.search_text.clear();
        self.position = position;
        self.from_connection = from_connection;
        self.selected_index = 0;
        self.request_focus = true;
        self.update_filtered_results();
    }

    /// Closes the popup and resets state.
    pub fn close(&mut self) {
        self.is_open = false;
        self.search_text.clear();
        self.from_connection = None;
        self.selected_index = 0;
    }

    /// Recalculates `filtered_results` based on current search text and type filter.
    pub fn update_filtered_results(&mut self) {
        let search_lower = self.search_text.to_lowercase();

        self.filtered_results = self
            .all_results
            .iter()
            .filter(|r| {
                // Substring match on name (case-insensitive)
                if !search_lower.is_empty() && !r.name.to_lowercase().contains(&search_lower) {
                    return false;
                }

                // Type filter from dropped connection
                if let Some(conn) = &self.from_connection {
                    if !is_type_compatible(r, conn) {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Sort by type relevance when opened from a dropped connection.
        // Direct type matches appear before accepts_any_type matches.
        if let Some(conn) = &self.from_connection {
            let conn = conn.clone();
            self.filtered_results
                .sort_by_key(|r| type_relevance_score(r, &conn));
        }

        // Clamp selected index
        if self.filtered_results.is_empty() {
            self.selected_index = 0;
        } else if self.selected_index >= self.filtered_results.len() {
            self.selected_index = self.filtered_results.len() - 1;
        }
    }

    /// Renders the popup and returns the response for this frame.
    pub fn show(&mut self, ctx: &egui::Context, theme: &Theme) -> NodeSearchPopupResponse {
        let mut response = NodeSearchPopupResponse {
            selected_operation: None,
            selected_subgraph: false,
            closed: false,
        };

        if !self.is_open {
            return response;
        }

        let popup_width = 300.0;
        let scroll_area_max_height = 300.0;

        let popup_id = egui::Id::new("node_search_popup");

        // Estimate popup rect for outside-click detection
        let popup_rect = egui::Rect::from_min_size(
            self.position,
            egui::Vec2::new(popup_width + 20.0, scroll_area_max_height + 60.0),
        );

        let _area_response = egui::Area::new(popup_id)
            .order(egui::Order::Foreground)
            .fixed_pos(self.position)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(theme.get().panel_fill)
                    .show(ui, |ui| {
                        ui.set_width(popup_width);
                        ui.set_min_height(scroll_area_max_height);

                        // Search text field with themed background
                        let text_response = egui::Frame::NONE
                            .fill(theme.get().menu_bar)
                            .corner_radius(1.0)
                            .inner_margin(4.0)
                            .show(ui, |ui| {
                                let text_edit = egui::TextEdit::singleline(&mut self.search_text)
                                    .frame(false)
                                    .desired_width(popup_width - 24.0)
                                    .hint_text("Search nodes...");
                                ui.add(text_edit)
                            })
                            .inner;

                        // Request focus on first frame
                        if self.request_focus {
                            text_response.request_focus();
                            self.request_focus = false;
                        }

                        // Handle keyboard input on the text field
                        let should_update = text_response.changed();

                        // Check for key presses
                        let (pressed_up, pressed_down, pressed_enter, pressed_escape) =
                            ctx.input(|i| {
                                (
                                    i.key_pressed(egui::Key::ArrowUp),
                                    i.key_pressed(egui::Key::ArrowDown),
                                    i.key_pressed(egui::Key::Enter),
                                    i.key_pressed(egui::Key::Escape),
                                )
                            });

                        if pressed_escape {
                            response.closed = true;
                            return;
                        }

                        if pressed_up && self.selected_index > 0 {
                            self.selected_index -= 1;
                        }

                        if pressed_down && !self.filtered_results.is_empty() {
                            if self.selected_index < self.filtered_results.len() - 1 {
                                self.selected_index += 1;
                            }
                        }

                        if pressed_enter && !self.filtered_results.is_empty() {
                            let selected = &self.filtered_results[self.selected_index];
                            response.selected_operation = Some(selected.operation.clone());
                            response.closed = true;
                            return;
                        }

                        if should_update {
                            self.update_filtered_results();
                        }

                        // Results list
                        ui.separator();

                        egui::ScrollArea::vertical()
                            .max_height(scroll_area_max_height)
                            .show(ui, |ui| {
                                ui.with_layout(
                                    egui::Layout::top_down_justified(egui::Align::LEFT),
                                    |ui| {
                                        for (i, result) in self.filtered_results.iter().enumerate()
                                        {
                                            let is_selected = i == self.selected_index;

                                            let mut job = egui::text::LayoutJob::default();
                                            job.append(
                                                &format!("{}  ", result.name),
                                                0.0,
                                                egui::TextFormat::simple(
                                                    egui::FontId::default(),
                                                    ui.visuals().text_color(),
                                                ),
                                            );
                                            job.append(
                                                &result.category_path,
                                                0.0,
                                                egui::TextFormat::simple(
                                                    egui::FontId::default(),
                                                    ui.visuals().weak_text_color(),
                                                ),
                                            );
                                            let display_text = egui::WidgetText::from(job);

                                            let selectable =
                                                ui.selectable_label(is_selected, display_text);

                                            if selectable.clicked() {
                                                response.selected_operation =
                                                    Some(result.operation.clone());
                                                response.closed = true;
                                                return;
                                            }

                                            if selectable.hovered() {
                                                self.selected_index = i;
                                            }
                                        }

                                        if self.filtered_results.is_empty() {
                                            ui.label(
                                                egui::RichText::new("No matching nodes")
                                                    .weak()
                                                    .italics(),
                                            );
                                        }
                                    },
                                );
                            });
                    });
            });

        // Close if clicked outside
        let clicked_outside = ctx.input(|i| {
            i.pointer.any_pressed()
                && !popup_rect.contains(i.pointer.interact_pos().unwrap_or(Pos2::ZERO))
        });
        if clicked_outside && !response.closed {
            response.closed = true;
        }

        response
    }
}

/// Checks whether an operation is type-compatible with a dropped connection.
///
/// If the connection was dragged from an output, the operation must have at
/// least one input that accepts the output's value type. If dragged from an
/// input, the operation must have at least one output whose type is compatible.
fn is_type_compatible(result: &SearchResult, conn: &TempConnection) -> bool {
    match conn.from_connection_type {
        // Dragged from an output: look for operations with compatible inputs
        ConnectionType::Output => {
            let inputs = result.operation.create_inputs();
            inputs.iter().any(|input| {
                input.accepts_any_type
                    || input
                        .value
                        .value_type()
                        .valid_conversions()
                        .contains(&conn.from_value_type)
            })
        }
        // Dragged from an input: look for operations with compatible outputs
        ConnectionType::Input => {
            if conn.from_accepts_any_type {
                // If the input accepts any type, all operations are compatible
                return true;
            }
            let valid_from = conn.from_value_type.valid_conversions_from();
            let outputs = result.operation.create_outputs();
            outputs
                .iter()
                .any(|output| valid_from.contains(&output.value.value_type()))
        }
    }
}

/// Returns a sort key for how relevant an operation is to a dropped connection.
/// Lower values = more relevant (shown first).
///
/// - 0: has an input/output with an exact type match
/// - 1: has an input/output with a compatible (convertible) type match
/// - 2: only matches via accepts_any_type
fn type_relevance_score(result: &SearchResult, conn: &TempConnection) -> u8 {
    match conn.from_connection_type {
        // Dragged from an output: score based on the operation's inputs
        ConnectionType::Output => {
            let inputs = result.operation.create_inputs();
            let mut best = 2u8;
            for input in &inputs {
                if input.value.value_type() == conn.from_value_type {
                    return 0; // Exact match, can't do better
                }
                if !input.accepts_any_type
                    && input
                        .value
                        .value_type()
                        .valid_conversions()
                        .contains(&conn.from_value_type)
                {
                    best = best.min(1);
                }
            }
            best
        }
        // Dragged from an input: score based on the operation's outputs
        ConnectionType::Input => {
            let outputs = result.operation.create_outputs();
            let mut best = 2u8;
            for output in &outputs {
                if output.value.value_type() == conn.from_value_type {
                    return 0;
                }
                if conn
                    .from_value_type
                    .valid_conversions_from()
                    .contains(&output.value.value_type())
                {
                    best = best.min(1);
                }
            }
            best
        }
    }
}

/// Recursively flattens the nested `OperationListItem` tree into a flat list.
///
/// Each operation gets a `category_path` string like "images > noise" showing
/// its position in the menu hierarchy.
pub fn flatten_operations(items: &[OperationListItem], path: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();

    for item in items {
        match item {
            OperationListItem::Category {
                name,
                operation_list_items,
            } => {
                let new_path = if path.is_empty() {
                    name.clone()
                } else {
                    format!("{} > {}", path, name)
                };
                results.extend(flatten_operations(operation_list_items, &new_path));
            }
            OperationListItem::Operation { operation } => {
                let settings = operation.settings();
                results.push(SearchResult {
                    operation: operation.clone(),
                    name: settings.name,
                    category_path: path.to_string(),
                });
            }
            OperationListItem::Subgraph => {
                // Subgraphs are not searchable in the popup for now
            }
        }
    }

    results
}

#[cfg(test)]
#[path = "node_search_popup_tests.rs"]
mod tests;
