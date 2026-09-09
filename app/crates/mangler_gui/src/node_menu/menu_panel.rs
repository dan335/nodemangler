//! The Node List panel: a search box over a collapsible category tree.
//!
//! Rows can be clicked (adds the node to the focused graph) or dragged (places
//! it under the pointer). Those two travel on separate channels of
//! [`MenuPanelOutput`] so a click can never leave a phantom drag armed.

use eframe::egui;
use mangler_core::AddNodeType;

use crate::graph::node_search_popup::{flatten_operations, SearchResult};
use crate::themes::theme::Theme;

use super::menu_filter::{filter_ranked, no_match_text, subgraph_matches, SUBGRAPH_LABEL};
use super::menu_item::{MenuItem, MenuItemsResult, RowKind, RowSpec, visible_rows};
use super::menu_row::{show_row, RowEvent, ROW_HEIGHT};

/// Horizontal room reserved at the top-right for the panel's corner
/// kind-switcher button, which `panel_view` draws on top of this panel.
const CORNER_BUTTON_CLEARANCE: f32 = 22.0;

/// Width of the search field's clear button, reserved even when hidden so the
/// field keeps a constant width.
const CLEAR_BUTTON_WIDTH: f32 = 20.0;

/// What the panel produced this frame.
pub struct MenuPanelOutput {
    /// A drag that just started. Only this is ever stored into
    /// `Program::dragging_menu_button`.
    pub drag: MenuItemsResult,
    /// A row that was clicked. Handled immediately and never stored.
    pub add_now: Option<AddNodeType>,
}

impl Default for MenuPanelOutput {
    fn default() -> Self {
        MenuPanelOutput {
            drag: MenuItemsResult::default(),
            add_now: None,
        }
    }
}

pub struct MenuPanel {
    pub items: Vec<MenuItem>,
    /// Every operation flattened once at startup, for search. Rebuilding this
    /// per keystroke would call `settings()` on all 450 operations, allocating
    /// their paragraph-long `help` strings each time.
    flat: Vec<SearchResult>,
    query: String,
    /// The query `filtered` was computed from, so filtering runs on change
    /// rather than every frame.
    last_query: String,
    filtered: Vec<usize>,
    subgraph_matches: bool,
}

impl MenuPanel {
    pub fn new() -> MenuPanel {
        let items = mangler_core::operations::operation_list()
            .iter()
            .map(|op| MenuItem::new(op.clone(), 0))
            .collect();
        let flat = flatten_operations(&mangler_core::operations::operation_list(), "");

        MenuPanel {
            items,
            flat,
            query: String::new(),
            last_query: String::new(),
            filtered: Vec::new(),
            subgraph_matches: true,
        }
    }

    /// Recomputes the filtered set if the query changed. Returns whether it
    /// actually recomputed (the tests use this to pin the caching).
    pub fn refresh_filter_if_needed(&mut self) -> bool {
        if self.query == self.last_query {
            return false;
        }
        self.filtered = filter_ranked(&self.query, &self.flat);
        self.subgraph_matches = subgraph_matches(&self.query);
        self.last_query = self.query.clone();
        true
    }

    pub fn show(&mut self, ui: &mut egui::Ui, theme: &Theme) -> MenuPanelOutput {
        let mut output = MenuPanelOutput::default();
        let colors = theme.get();

        // Inset the panel body, matching the Libraries panel, so the search
        // field and rows don't sit flush against the panel edges.
        egui::Frame::new()
            .inner_margin(egui::Margin {
                left: 8,
                right: 8,
                top: 5,
                bottom: 8,
            })
            .show(ui, |ui| {
                self.show_search_field(ui, &colors);
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("click a node to add it, or drag it onto a graph")
                        .color(colors.text_faint)
                        .size(11.0),
                );
                ui.add_space(8.0);

                let searching = !self.query.trim().is_empty();
                if searching {
                    self.refresh_filter_if_needed();
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    if searching {
                        self.show_search_results(ui, theme, &mut output, &colors);
                    } else {
                        self.show_tree(ui, theme, &mut output);
                    }
                });
            });

        output
    }

    fn show_search_field(
        &mut self,
        ui: &mut egui::Ui,
        colors: &crate::themes::theme::ThemeValues,
    ) {
        ui.horizontal(|ui| {
            // Right-to-left so the field can claim whatever is left after the
            // fixed-width bits on the right.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Leave room for the panel's corner kind-switcher button, which
                // panel_view draws over this panel's top-right corner. Without
                // it the search field runs underneath the button.
                ui.add_space(CORNER_BUTTON_CLEARANCE);

                // Reserve the clear button's slot whether or not it is showing,
                // so the field doesn't change width on the first keystroke.
                let (clear_rect, _) = ui.allocate_exact_size(
                    egui::vec2(CLEAR_BUTTON_WIDTH, ROW_HEIGHT),
                    egui::Sense::hover(),
                );
                if !self.query.is_empty()
                    && ui
                        .put(
                            clear_rect,
                            egui::Button::new(
                                egui::RichText::new(egui_phosphor::regular::X)
                                    .color(colors.text_faint),
                            )
                            .frame(false),
                        )
                        .on_hover_text("clear search")
                        .clicked()
                {
                    self.query.clear();
                }

                // Pin the id. The clear button above appears only once the
                // query is non-empty, and it is laid out *before* this field —
                // so without a stable salt the field's auto-generated id would
                // shift on the first keystroke and egui would drop focus after
                // a single character. `id_salt` resolves through
                // `ui.make_persistent_id`, so duplicate Node List panels still
                // get distinct ids.
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .id_salt("node_menu_search")
                        .desired_width(ui.available_width())
                        .hint_text("search nodes…"),
                );

                // Escape clears and releases focus, so the graph's own
                // shortcuts (Tab search, F to focus) come back without a
                // mouse trip.
                if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.query.clear();
                    response.surrender_focus();
                }
            });
        });
    }

    fn show_tree(&mut self, ui: &mut egui::Ui, theme: &Theme, output: &mut MenuPanelOutput) {
        // Collected first so the tree can be mutated (collapse toggles) after
        // the borrow the rows hold is released.
        let mut toggle: Option<Vec<usize>> = None;
        {
            let rows = visible_rows(&self.items);
            for row in &rows {
                match show_row(ui, row, theme) {
                    RowEvent::ToggleCategory => toggle = Some(row.path.clone()),
                    RowEvent::Add => output.add_now = add_node_type(row),
                    RowEvent::DragStart => arm_drag(row, output),
                    RowEvent::None => {}
                }
            }
        }
        if let Some(path) = toggle {
            MenuItem::toggle_at(&mut self.items, &path);
        }
    }

    fn show_search_results(
        &self,
        ui: &mut egui::Ui,
        theme: &Theme,
        output: &mut MenuPanelOutput,
        colors: &crate::themes::theme::ThemeValues,
    ) {
        if self.filtered.is_empty() && !self.subgraph_matches {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(no_match_text(&self.query))
                    .color(colors.text_faint),
            );
            return;
        }

        for &index in &self.filtered {
            let result = &self.flat[index];
            let row = RowSpec {
                kind: RowKind::Operation(&result.operation),
                label: &result.name,
                level: 0,
                description: &result.description,
                help: "",
                secondary: Some(&result.category_path),
                path: Vec::new(),
            };
            match show_row(ui, &row, theme) {
                RowEvent::Add => output.add_now = add_node_type(&row),
                RowEvent::DragStart => arm_drag(&row, output),
                _ => {}
            }
        }

        // `flatten_operations` excludes subgraphs, so this row is appended here.
        if self.subgraph_matches {
            let row = RowSpec {
                kind: RowKind::Subgraph,
                label: SUBGRAPH_LABEL,
                level: 0,
                description: "",
                help: "",
                secondary: None,
                path: Vec::new(),
            };
            match show_row(ui, &row, theme) {
                RowEvent::Add => output.add_now = add_node_type(&row),
                RowEvent::DragStart => arm_drag(&row, output),
                _ => {}
            }
        }
    }
}

fn add_node_type(row: &RowSpec) -> Option<AddNodeType> {
    match row.kind {
        RowKind::Operation(operation) => Some(AddNodeType::Operation(operation.clone())),
        RowKind::Subgraph => Some(AddNodeType::Subgraph),
        RowKind::Category { .. } => None,
    }
}

fn arm_drag(row: &RowSpec, output: &mut MenuPanelOutput) {
    match row.kind {
        RowKind::Operation(operation) => {
            output.drag.operation_being_created = Some(operation.clone())
        }
        RowKind::Subgraph => output.drag.subgraph_being_created = true,
        RowKind::Category { .. } => {}
    }
}

#[cfg(test)]
#[path = "menu_panel_tests.rs"]
mod tests;
