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

    /// The system default layout: node list (250px) | center stack | settings
    /// (300px), built from nested splits with fractions derived from
    /// `work_width`. The center stack is itself a `Column`: the graph editor
    /// on top, and a bottom `Row` of the 2D and 3D previews side by side —
    /// i.e. `Row(NodeList, Row(Column(Graph, Row(Preview2D, Preview3D)),
    /// Settings))`.
    pub fn system_default(work_width: f32, next_id: &mut LeafId) -> Self {
        let mut alloc = || {
            let id = *next_id;
            *next_id += 1;
            id
        };
        // Allocation order is also `leaves()` traversal order (in-order over
        // the tree built below), so callers/tests can zip ids to kinds.
        let node_list_id = alloc();
        let graph_id = alloc();
        let preview_2d_id = alloc();
        let preview_3d_id = alloc();
        let settings_id = alloc();

        let outer_fraction =
            (crate::NODE_MENU_WIDTH / work_width.max(1.0)).clamp(*FRACTION_RANGE.start(), *FRACTION_RANGE.end());
        let remaining = (work_width - crate::NODE_MENU_WIDTH).max(1.0);
        let inner_fraction = (1.0 - crate::SETTINGS_PANEL_WIDTH / remaining)
            .clamp(*FRACTION_RANGE.start(), *FRACTION_RANGE.end());
        // Center-stack split: graph gets slightly more than half the vertical
        // space, since it's the primary editing surface and the previews
        // below it are secondary/supplementary.
        const CENTER_STACK_FRACTION: f32 = 0.55;
        // Bottom row of the center stack: 2D and 3D previews split evenly.
        const PREVIEW_ROW_FRACTION: f32 = 0.5;
        let center_stack_fraction =
            CENTER_STACK_FRACTION.clamp(*FRACTION_RANGE.start(), *FRACTION_RANGE.end());
        let preview_row_fraction =
            PREVIEW_ROW_FRACTION.clamp(*FRACTION_RANGE.start(), *FRACTION_RANGE.end());

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
                            Box::new(PanelNode::Split {
                                direction: SplitDirection::Column,
                                fraction: center_stack_fraction,
                                children: [
                                    Box::new(PanelNode::Leaf {
                                        id: graph_id,
                                        kind: PanelKind::Graph,
                                    }),
                                    Box::new(PanelNode::Split {
                                        direction: SplitDirection::Row,
                                        fraction: preview_row_fraction,
                                        children: [
                                            Box::new(PanelNode::Leaf {
                                                id: preview_2d_id,
                                                kind: PanelKind::Preview2D,
                                            }),
                                            Box::new(PanelNode::Leaf {
                                                id: preview_3d_id,
                                                kind: PanelKind::Preview3D,
                                            }),
                                        ],
                                    }),
                                ],
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

    /// Set the fraction of the `Split` node reached by following `path`
    /// (child indices from the root), clamped to keep both children visible.
    /// No-op if the path does not resolve to a split. Retained for tests and
    /// programmatic layout tweaks; interactive drags use [`Self::drag_splitter`].
    #[allow(dead_code)]
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

    /// Drag the divider of the `Split` at `path` (whose on-screen rect was
    /// `node_rect`) so its divider sits at `pointer` (coordinate along the
    /// split axis, absolute, same space as `node_rect`). Only the leaves
    /// adjacent to the divider change size; all other leaves keep their pixel
    /// extent. Returns `true` if anything changed.
    ///
    /// This is the Blender-style behavior: a divider trades space between the
    /// two panels touching it, rather than proportionally rescaling the whole
    /// nested group on the far side.
    pub fn drag_splitter(&mut self, path: &[usize], node_rect: Rect, pointer: f32) -> bool {
        // Navigate to the split node addressed by `path`.
        let mut node: &mut PanelNode = &mut self.root;
        for &index in path {
            match node {
                PanelNode::Split { children, .. } => match children.get_mut(index) {
                    Some(child) => node = child,
                    None => return false,
                },
                PanelNode::Leaf { .. } => return false,
            }
        }
        let PanelNode::Split {
            direction,
            fraction,
            children,
        } = node
        else {
            return false;
        };
        let axis = *direction;
        let extent = axis_extent(node_rect, axis);
        let available = (extent - SPLITTER_WIDTH).max(0.0);
        if available <= 0.0 {
            return false;
        }

        // Effective current first-child extent, mirroring `layout()`'s clamp so
        // the absorb math below matches what is actually on screen.
        let mut old_e0 = *fraction * available;
        if available >= 2.0 * MIN_PANEL_SIZE {
            old_e0 = old_e0.clamp(MIN_PANEL_SIZE, available - MIN_PANEL_SIZE);
        }

        // Where the user wants the divider, as an offset from the node's origin.
        let new_e0 = pointer - axis_min(node_rect, axis);
        let mut delta = new_e0 - old_e0;

        // Clamp the delta so neither divider-adjacent leaf drops below the
        // minimum: shrinking child1 (moving right, +delta) is bounded by the
        // leading leaves of child1; shrinking child0 (moving left, -delta) by
        // the trailing leaves of child0.
        let child1_lead = min_edge_leaf_extent(&children[1], axis, Edge::Leading, available - old_e0);
        let child0_trail = min_edge_leaf_extent(&children[0], axis, Edge::Trailing, old_e0);
        let delta_max = child1_lead - MIN_PANEL_SIZE;
        let delta_min = -(child0_trail - MIN_PANEL_SIZE);
        if delta_min > delta_max {
            return false;
        }
        delta = delta.clamp(delta_min, delta_max);
        if delta.abs() < 0.001 {
            return false;
        }

        // Move the dragged divider. `available` is unchanged (the node's own
        // rect does not move), so the new fraction is just the new extent over
        // the same available space.
        *fraction = (old_e0 + delta) / available;

        // Absorb the delta into each subtree so only the divider-adjacent
        // leaves resize and everything else keeps its pixel extent.
        absorb(&mut children[0], axis, delta, Edge::Trailing, old_e0);
        absorb(&mut children[1], axis, -delta, Edge::Leading, available - old_e0);
        true
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

/// Which side of a subtree, along a split axis, an operation touches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edge {
    /// The min side along the axis (left for `Row`, top for `Column`).
    Leading,
    /// The max side along the axis (right for `Row`, bottom for `Column`).
    Trailing,
}

/// A rect's extent along the given split axis.
fn axis_extent(rect: Rect, axis: SplitDirection) -> f32 {
    match axis {
        SplitDirection::Row => rect.width(),
        SplitDirection::Column => rect.height(),
    }
}

/// A rect's min coordinate along the given split axis.
fn axis_min(rect: Rect, axis: SplitDirection) -> f32 {
    match axis {
        SplitDirection::Row => rect.min.x,
        SplitDirection::Column => rect.min.y,
    }
}

/// The minimum pixel extent (along `axis`) among the leaves that touch `edge`
/// of this subtree, given the subtree spans `extent` along `axis`.
fn min_edge_leaf_extent(node: &PanelNode, axis: SplitDirection, edge: Edge, extent: f32) -> f32 {
    match node {
        PanelNode::Leaf { .. } => extent,
        PanelNode::Split {
            direction,
            fraction,
            children,
        } => {
            if *direction == axis {
                // Split along the axis: only one child touches `edge`.
                let avail = (extent - SPLITTER_WIDTH).max(0.0);
                let e0 = fraction * avail;
                let e1 = avail - e0;
                match edge {
                    Edge::Leading => min_edge_leaf_extent(&children[0], axis, edge, e0),
                    Edge::Trailing => min_edge_leaf_extent(&children[1], axis, edge, e1),
                }
            } else {
                // Split perpendicular to the axis: both children span the full
                // extent and both touch `edge`.
                min_edge_leaf_extent(&children[0], axis, edge, extent)
                    .min(min_edge_leaf_extent(&children[1], axis, edge, extent))
            }
        }
    }
}

/// Absorb a `delta` change in this subtree's extent along `axis` into the leaf
/// that touches `edge`, keeping every other leaf's pixel extent fixed.
/// `old_extent` is the subtree's extent along `axis` before the change.
fn absorb(node: &mut PanelNode, axis: SplitDirection, delta: f32, edge: Edge, old_extent: f32) {
    let PanelNode::Split {
        direction,
        fraction,
        children,
    } = node
    else {
        // Leaf: it simply resizes with its rect.
        return;
    };
    if *direction == axis {
        // Split along the axis: the child on `edge` absorbs the delta; the
        // other child keeps its pixel extent.
        let avail_old = (old_extent - SPLITTER_WIDTH).max(0.0);
        let e0 = *fraction * avail_old;
        let e1 = avail_old - e0;
        let avail_new = avail_old + delta;
        if avail_new <= 0.0 {
            return;
        }
        match edge {
            Edge::Leading => {
                *fraction = (e0 + delta) / avail_new;
                absorb(&mut children[0], axis, delta, edge, e0);
            }
            Edge::Trailing => {
                *fraction = e0 / avail_new;
                absorb(&mut children[1], axis, delta, edge, e1);
            }
        }
    } else {
        // Split perpendicular to the axis: both children span the full extent
        // along `axis`, so both absorb the same delta on the same edge.
        absorb(&mut children[0], axis, delta, edge, old_extent);
        absorb(&mut children[1], axis, delta, edge, old_extent);
    }
}

#[cfg(test)]
#[path = "panel_tree_tests.rs"]
mod tests;
