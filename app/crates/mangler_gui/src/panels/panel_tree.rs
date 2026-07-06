//! Pure panel-tree logic: a binary tree of splits and leaves, laid out into
//! rectangles. No egui `Ui`/`Context` here — only geometry types — so the
//! whole module is unit-testable without a UI.

use epaint::{Pos2, Rect};
use serde::{Deserialize, Serialize};

use super::panel_kind::PanelKind;

/// Identifies a leaf panel within a tree. Ids are allocated from a counter
/// owned by the app and are unique across all trees (main + secondary windows).
pub type LeafId = u64;

/// Thickness of the draggable strip between two split children, in points.
pub const SPLITTER_WIDTH: f32 = 4.0;

/// Minimum extent of a panel along a split axis, in points. Layout clamps
/// split fractions so both children stay at least this large when the
/// available space allows it.
pub const MIN_PANEL_SIZE: f32 = 80.0;

/// Fractions are clamped to this range so neither child of a split can be
/// squeezed away entirely.
const FRACTION_RANGE: std::ops::RangeInclusive<f32> = 0.05..=0.95;

/// How a split arranges its two children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitDirection {
    /// Children side-by-side (split axis = x).
    Row,
    /// Children stacked (split axis = y).
    Column,
}

/// A node in the panel tree: either a leaf panel or a two-way split.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PanelNode {
    Leaf {
        id: LeafId,
        kind: PanelKind,
    },
    Split {
        direction: SplitDirection,
        /// Share of the available extent (after the splitter strip) given to
        /// the first child. Kept as a fraction — not pixels — so panels scale
        /// proportionally when the window resizes.
        fraction: f32,
        children: [Box<PanelNode>; 2],
    },
}

/// A whole panel layout for one OS window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelTree {
    pub root: PanelNode,
}

/// Why a `close` request could not be honored.
#[derive(Debug, PartialEq)]
pub enum CloseError {
    /// The leaf is the tree's root — a window must keep at least one panel.
    IsRoot,
    /// No leaf with that id exists in this tree.
    NotFound,
}

/// A draggable strip between the two children of a split, as produced by
/// [`PanelTree::layout`].
#[derive(Debug, Clone, PartialEq)]
pub struct Splitter {
    /// Child indices (0/1) from the root to the `Split` node this strip
    /// belongs to; feed it back into [`PanelTree::set_fraction`] when dragged.
    pub path: Vec<usize>,
    pub direction: SplitDirection,
    /// The strip itself.
    pub rect: Rect,
    /// The whole rect of the `Split` node, for converting a drag position
    /// back into a fraction.
    pub parent_rect: Rect,
}

/// The result of laying a tree out into a rect: one rect per leaf (in-order)
/// plus one splitter strip per split node.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TreeLayout {
    pub leaves: Vec<(LeafId, PanelKind, Rect)>,
    pub splitters: Vec<Splitter>,
}

impl PanelTree {
    /// A tree consisting of a single leaf.
    pub fn single(kind: PanelKind, id: LeafId) -> Self {
        Self {
            root: PanelNode::Leaf { id, kind },
        }
    }

    /// The system default layout: three columns matching the classic fixed
    /// layout — node list (250px) | graph (flex) | settings (300px) — built
    /// from nested `Row` splits with fractions derived from `work_width`.
    pub fn system_default(work_width: f32, next_id: &mut LeafId) -> Self {
        let mut alloc = || {
            let id = *next_id;
            *next_id += 1;
            id
        };
        let node_list_id = alloc();
        let graph_id = alloc();
        let settings_id = alloc();

        let outer_fraction =
            (crate::NODE_MENU_WIDTH / work_width.max(1.0)).clamp(*FRACTION_RANGE.start(), *FRACTION_RANGE.end());
        let remaining = (work_width - crate::NODE_MENU_WIDTH).max(1.0);
        let inner_fraction = (1.0 - crate::SETTINGS_PANEL_WIDTH / remaining)
            .clamp(*FRACTION_RANGE.start(), *FRACTION_RANGE.end());

        Self {
            root: PanelNode::Split {
                direction: SplitDirection::Row,
                fraction: outer_fraction,
                children: [
                    Box::new(PanelNode::Leaf {
                        id: node_list_id,
                        kind: PanelKind::NodeList,
                    }),
                    Box::new(PanelNode::Split {
                        direction: SplitDirection::Row,
                        fraction: inner_fraction,
                        children: [
                            Box::new(PanelNode::Leaf {
                                id: graph_id,
                                kind: PanelKind::Graph,
                            }),
                            Box::new(PanelNode::Leaf {
                                id: settings_id,
                                kind: PanelKind::Settings,
                            }),
                        ],
                    }),
                ],
            },
        }
    }

    /// Give every leaf a fresh id from the counter. Used when loading a saved
    /// layout so ids never collide with live ones.
    pub fn reassign_ids(&mut self, next_id: &mut LeafId) {
        fn walk(node: &mut PanelNode, next_id: &mut LeafId) {
            match node {
                PanelNode::Leaf { id, .. } => {
                    *id = *next_id;
                    *next_id += 1;
                }
                PanelNode::Split { children, .. } => {
                    for child in children {
                        walk(child, next_id);
                    }
                }
            }
        }
        walk(&mut self.root, next_id);
    }

    /// Replace the leaf with a 50/50 split whose second child is a new leaf of
    /// the same kind. Returns `false` if no leaf with that id exists.
    pub fn split(&mut self, leaf_id: LeafId, direction: SplitDirection, new_id: LeafId) -> bool {
        fn walk(
            node: &mut PanelNode,
            leaf_id: LeafId,
            direction: SplitDirection,
            new_id: LeafId,
        ) -> bool {
            match node {
                PanelNode::Leaf { id, kind } => {
                    if *id == leaf_id {
                        let old = PanelNode::Leaf {
                            id: *id,
                            kind: *kind,
                        };
                        let sibling = PanelNode::Leaf {
                            id: new_id,
                            kind: *kind,
                        };
                        *node = PanelNode::Split {
                            direction,
                            fraction: 0.5,
                            children: [Box::new(old), Box::new(sibling)],
                        };
                        true
                    } else {
                        false
                    }
                }
                PanelNode::Split { children, .. } => children
                    .iter_mut()
                    .any(|child| walk(child, leaf_id, direction, new_id)),
            }
        }
        walk(&mut self.root, leaf_id, direction, new_id)
    }

    /// Remove the leaf, promoting its sibling into their parent's slot.
    pub fn close(&mut self, leaf_id: LeafId) -> Result<(), CloseError> {
        if !self.contains(leaf_id) {
            return Err(CloseError::NotFound);
        }
        if matches!(self.root, PanelNode::Leaf { id, .. } if id == leaf_id) {
            return Err(CloseError::IsRoot);
        }

        fn is_target(node: &PanelNode, leaf_id: LeafId) -> bool {
            matches!(node, PanelNode::Leaf { id, .. } if *id == leaf_id)
        }

        /// Returns true once the leaf has been removed.
        fn walk(node: &mut PanelNode, leaf_id: LeafId) -> bool {
            let PanelNode::Split { children, .. } = node else {
                return false;
            };
            let promote = if is_target(&children[0], leaf_id) {
                Some(1)
            } else if is_target(&children[1], leaf_id) {
                Some(0)
            } else {
                None
            };
            if let Some(keep) = promote {
                // Move the surviving sibling into the parent's slot.
                let sibling =
                    std::mem::replace(&mut *children[keep], PanelNode::Leaf { id: 0, kind: PanelKind::Graph });
                *node = sibling;
                return true;
            }
            children.iter_mut().any(|child| walk(child, leaf_id))
        }

        if walk(&mut self.root, leaf_id) {
            Ok(())
        } else {
            // `contains` said it exists and it is not the root, so a parent
            // split must have matched above.
            Err(CloseError::NotFound)
        }
    }

    /// Change the content kind shown by a leaf. Returns `false` if not found.
    pub fn set_kind(&mut self, leaf_id: LeafId, kind: PanelKind) -> bool {
        fn walk(node: &mut PanelNode, leaf_id: LeafId, kind: PanelKind) -> bool {
            match node {
                PanelNode::Leaf { id, kind: k } => {
                    if *id == leaf_id {
                        *k = kind;
                        true
                    } else {
                        false
                    }
                }
                PanelNode::Split { children, .. } => {
                    children.iter_mut().any(|child| walk(child, leaf_id, kind))
                }
            }
        }
        walk(&mut self.root, leaf_id, kind)
    }

    /// Whether a leaf with this id exists anywhere in the tree.
    pub fn contains(&self, leaf_id: LeafId) -> bool {
        fn walk(node: &PanelNode, leaf_id: LeafId) -> bool {
            match node {
                PanelNode::Leaf { id, .. } => *id == leaf_id,
                PanelNode::Split { children, .. } => {
                    children.iter().any(|child| walk(child, leaf_id))
                }
            }
        }
        walk(&self.root, leaf_id)
    }

    /// All leaves in-order (first child before second child).
    pub fn leaves(&self) -> Vec<(LeafId, PanelKind)> {
        fn walk(node: &PanelNode, out: &mut Vec<(LeafId, PanelKind)>) {
            match node {
                PanelNode::Leaf { id, kind } => out.push((*id, *kind)),
                PanelNode::Split { children, .. } => {
                    for child in children {
                        walk(child, out);
                    }
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.root, &mut out);
        out
    }

    /// The first leaf in-order — the fallback target when no panel is focused.
    pub fn first_leaf(&self) -> LeafId {
        let mut node = &self.root;
        loop {
            match node {
                PanelNode::Leaf { id, .. } => return *id,
                PanelNode::Split { children, .. } => node = &children[0],
            }
        }
    }

    /// Set the fraction of the `Split` node reached by following `path`
    /// (child indices from the root), clamped to keep both children visible.
    /// No-op if the path does not resolve to a split.
    pub fn set_fraction(&mut self, path: &[usize], fraction: f32) {
        let mut node = &mut self.root;
        for &index in path {
            match node {
                PanelNode::Split { children, .. } => match children.get_mut(index) {
                    Some(child) => node = child,
                    None => return,
                },
                PanelNode::Leaf { .. } => return,
            }
        }
        if let PanelNode::Split { fraction: f, .. } = node {
            *f = fraction.clamp(*FRACTION_RANGE.start(), *FRACTION_RANGE.end());
        }
    }

    /// Recursively subdivide `rect`, producing a rect for every leaf and a
    /// draggable strip for every split.
    pub fn layout(&self, rect: Rect) -> TreeLayout {
        fn walk(node: &PanelNode, rect: Rect, path: &mut Vec<usize>, out: &mut TreeLayout) {
            match node {
                PanelNode::Leaf { id, kind } => out.leaves.push((*id, *kind, rect)),
                PanelNode::Split {
                    direction,
                    fraction,
                    children,
                } => {
                    let extent = match direction {
                        SplitDirection::Row => rect.width(),
                        SplitDirection::Column => rect.height(),
                    };
                    let available = (extent - SPLITTER_WIDTH).max(0.0);
                    let mut first = fraction * available;
                    if available >= 2.0 * MIN_PANEL_SIZE {
                        first = first.clamp(MIN_PANEL_SIZE, available - MIN_PANEL_SIZE);
                    }
                    let (first_rect, splitter_rect, second_rect) = match direction {
                        SplitDirection::Row => {
                            let split_x = rect.min.x + first;
                            (
                                Rect::from_min_max(rect.min, Pos2::new(split_x, rect.max.y)),
                                Rect::from_min_max(
                                    Pos2::new(split_x, rect.min.y),
                                    Pos2::new(split_x + SPLITTER_WIDTH, rect.max.y),
                                ),
                                Rect::from_min_max(
                                    Pos2::new(split_x + SPLITTER_WIDTH, rect.min.y),
                                    rect.max,
                                ),
                            )
                        }
                        SplitDirection::Column => {
                            let split_y = rect.min.y + first;
                            (
                                Rect::from_min_max(rect.min, Pos2::new(rect.max.x, split_y)),
                                Rect::from_min_max(
                                    Pos2::new(rect.min.x, split_y),
                                    Pos2::new(rect.max.x, split_y + SPLITTER_WIDTH),
                                ),
                                Rect::from_min_max(
                                    Pos2::new(rect.min.x, split_y + SPLITTER_WIDTH),
                                    rect.max,
                                ),
                            )
                        }
                    };
                    out.splitters.push(Splitter {
                        path: path.clone(),
                        direction: *direction,
                        rect: splitter_rect,
                        parent_rect: rect,
                    });
                    path.push(0);
                    walk(&children[0], first_rect, path, out);
                    path.pop();
                    path.push(1);
                    walk(&children[1], second_rect, path, out);
                    path.pop();
                }
            }
        }
        let mut out = TreeLayout::default();
        walk(&self.root, rect, &mut Vec::new(), &mut out);
        out
    }
}

#[cfg(test)]
#[path = "panel_tree_tests.rs"]
mod tests;
