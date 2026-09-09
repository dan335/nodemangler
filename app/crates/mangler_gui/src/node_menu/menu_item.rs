//! The Node List's data model: the category/operation tree and the flattening
//! that turns it into the flat run of rows actually drawn.
//!
//! Everything here is pure — no egui — so the tree walk and the click/drag
//! distance rule can be unit-tested. Painting lives in [`super::menu_row`] and
//! search lives in [`super::menu_filter`].

use epaint::Pos2;
use mangler_core::operations::Operation;
use mangler_core::operations::OperationListItem;

/// How far the pointer may travel between press and release and still count as
/// a click rather than a drag. Matches egui's default `max_click_dist`.
pub const CLICK_TRAVEL_MAX: f32 = 6.0;

/// Whether a drag that ended outside any graph panel should be treated as a
/// click instead of a discarded drop.
///
/// egui decides "click vs drag" on a `click_and_drag` widget via
/// `is_decidedly_dragging()`, which goes true either when the pointer moves past
/// `max_click_dist` **or** simply when the press outlasts `max_click_duration`
/// (0.8s). So a slow, motionless press on a node row reports `drag_started()`
/// and never `clicked()` — and since it is released over the node list rather
/// than a graph panel, the drop finds no target and nothing happens. Measuring
/// the actual travel recovers the user's intent: a press that went nowhere was
/// a click, however long they held it.
///
/// Both points are in screen coordinates, so this stays correct when the press
/// and release happen in different OS windows.
pub fn abandoned_drag_is_click(press_screen: Pos2, release_screen: Pos2) -> bool {
    press_screen.distance(release_screen) <= CLICK_TRAVEL_MAX
}

/// One node in the menu tree: a collapsible category, an operation, or the
/// single "subgraph" entry that `operation_list()` appends last.
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
        description: String,
        help: String,
        level: usize,
        operation: Operation,
    },
    SubgraphButton {
        name: String,
        level: usize,
    },
}

impl MenuItem {
    pub fn new(operation_item: OperationListItem, level: usize) -> MenuItem {
        match operation_item {
            OperationListItem::Category {
                name,
                operation_list_items,
            } => {
                let items = operation_list_items
                    .iter()
                    .map(|item| MenuItem::new(item.clone(), level + 1))
                    .collect();

                MenuItem::Category {
                    name,
                    items,
                    is_collapsed: true,
                    level,
                }
            }
            OperationListItem::Operation { operation } => {
                // One `settings()` call, not one per field: it rebuilds the
                // whole `NodeSettings` each time, including the paragraph-long
                // `help` string, for all 450 operations.
                let settings = operation.settings();
                MenuItem::OperationButton {
                    name: settings.name,
                    description: settings.description,
                    help: settings.help,
                    operation,
                    level,
                }
            }
            OperationListItem::Subgraph => MenuItem::SubgraphButton {
                name: "subgraph".to_string(),
                level,
            },
        }
    }

    /// Toggles the category reached by following `path` as a sequence of child
    /// indices. No-op if the path doesn't resolve to a category.
    pub fn toggle_at(items: &mut [MenuItem], path: &[usize]) {
        let Some((first, rest)) = path.split_first() else {
            return;
        };
        let Some(item) = items.get_mut(*first) else {
            return;
        };
        match item {
            MenuItem::Category {
                is_collapsed,
                items,
                ..
            } => {
                if rest.is_empty() {
                    *is_collapsed = !*is_collapsed;
                } else {
                    MenuItem::toggle_at(items, rest);
                }
            }
            _ => {}
        }
    }
}

/// What a [`RowSpec`] will create or toggle.
pub enum RowKind<'a> {
    Category { collapsed: bool },
    Operation(&'a Operation),
    Subgraph,
}

/// One row as it will be drawn. Borrows its strings from the tree (or from the
/// search cache), so building a frame's worth of rows allocates no strings.
pub struct RowSpec<'a> {
    pub kind: RowKind<'a>,
    pub label: &'a str,
    pub level: usize,
    pub description: &'a str,
    pub help: &'a str,
    /// Category path, shown only in search mode where the tree isn't visible.
    pub secondary: Option<&'a str>,
    /// Child-index path to this row, for `toggle_at`. Empty for non-categories.
    pub path: Vec<usize>,
}

/// Flattens the tree into the rows actually visible, honouring collapse state.
pub fn visible_rows(items: &[MenuItem]) -> Vec<RowSpec<'_>> {
    let mut rows = Vec::new();
    let mut path = Vec::new();
    collect_rows(items, &mut path, &mut rows);
    rows
}

fn collect_rows<'a>(items: &'a [MenuItem], path: &mut Vec<usize>, rows: &mut Vec<RowSpec<'a>>) {
    for (index, item) in items.iter().enumerate() {
        path.push(index);
        match item {
            MenuItem::Category {
                name,
                level,
                is_collapsed,
                items,
            } => {
                rows.push(RowSpec {
                    kind: RowKind::Category {
                        collapsed: *is_collapsed,
                    },
                    label: name,
                    level: *level,
                    description: "",
                    help: "",
                    secondary: None,
                    path: path.clone(),
                });
                if !*is_collapsed {
                    collect_rows(items, path, rows);
                }
            }
            MenuItem::OperationButton {
                name,
                description,
                help,
                level,
                operation,
            } => rows.push(RowSpec {
                kind: RowKind::Operation(operation),
                label: name,
                level: *level,
                description,
                help,
                secondary: None,
                path: Vec::new(),
            }),
            MenuItem::SubgraphButton { name, level } => rows.push(RowSpec {
                kind: RowKind::Subgraph,
                label: name,
                level: *level,
                description: "",
                help: "",
                secondary: None,
                path: Vec::new(),
            }),
        }
        path.pop();
    }
}

/// The node-list drag payload, stored on `Program::dragging_menu_button` and
/// consumed by `Program::show_menu_drag`. Deliberately carries only what a drop
/// needs — a click travels on its own channel (`MenuPanelOutput::add_now`) so a
/// click can never leave a phantom drag armed.
pub struct MenuItemsResult {
    pub operation_being_created: Option<Operation>,
    pub subgraph_being_created: bool,
}

impl MenuItemsResult {
    /// Whether no drag is armed.
    pub fn is_empty(&self) -> bool {
        self.operation_being_created.is_none() && !self.subgraph_being_created
    }
}

impl Default for MenuItemsResult {
    fn default() -> Self {
        MenuItemsResult {
            operation_being_created: None,
            subgraph_being_created: false,
        }
    }
}

#[cfg(test)]
#[path = "menu_item_tests.rs"]
mod tests;
