use super::*;
use epaint::{pos2, vec2, Rect};

/// A three-leaf tree: Row( Leaf(1, Graph), Column( Leaf(2, Settings), Leaf(3, NodeList) ) ).
fn three_leaf_tree() -> PanelTree {
    PanelTree {
        root: PanelNode::Split {
            direction: SplitDirection::Row,
            fraction: 0.5,
            children: [
                Box::new(PanelNode::Leaf {
                    id: 1,
                    kind: PanelKind::Graph,
                }),
                Box::new(PanelNode::Split {
                    direction: SplitDirection::Column,
                    fraction: 0.5,
                    children: [
                        Box::new(PanelNode::Leaf {
                            id: 2,
                            kind: PanelKind::Settings,
                        }),
                        Box::new(PanelNode::Leaf {
                            id: 3,
                            kind: PanelKind::NodeList,
                        }),
                    ],
                }),
            ],
        },
    }
}

#[test]
fn split_creates_sibling_with_same_kind_new_id_and_half_fraction() {
    let mut tree = PanelTree::single(PanelKind::Settings, 1);
    assert!(tree.split(1, SplitDirection::Column, 2));

    let PanelNode::Split {
        direction,
        fraction,
        children,
    } = &tree.root
    else {
        panic!("root should be a split after splitting the only leaf");
    };
    assert_eq!(*direction, SplitDirection::Column);
    assert_eq!(*fraction, 0.5);
    assert_eq!(
        *children[0],
        PanelNode::Leaf {
            id: 1,
            kind: PanelKind::Settings
        }
    );
    assert_eq!(
        *children[1],
        PanelNode::Leaf {
            id: 2,
            kind: PanelKind::Settings
        }
    );

    // Splitting an unknown id does nothing.
    assert!(!tree.split(99, SplitDirection::Row, 3));
}

#[test]
fn close_promotes_sibling_in_nested_tree() {
    let mut tree = three_leaf_tree();
    assert_eq!(tree.close(2), Ok(()));

    // Leaf 3 is promoted into the inner split's slot.
    assert_eq!(
        tree.root,
        PanelNode::Split {
            direction: SplitDirection::Row,
            fraction: 0.5,
            children: [
                Box::new(PanelNode::Leaf {
                    id: 1,
                    kind: PanelKind::Graph
                }),
                Box::new(PanelNode::Leaf {
                    id: 3,
                    kind: PanelKind::NodeList
                }),
            ],
        }
    );

    // Closing again collapses to a single leaf.
    assert_eq!(tree.close(1), Ok(()));
    assert_eq!(
        tree.root,
        PanelNode::Leaf {
            id: 3,
            kind: PanelKind::NodeList
        }
    );
}

#[test]
fn close_root_leaf_is_err_is_root() {
    let mut tree = PanelTree::single(PanelKind::Graph, 7);
    assert_eq!(tree.close(7), Err(CloseError::IsRoot));
    // Tree unchanged.
    assert_eq!(tree.leaves(), vec![(7, PanelKind::Graph)]);
}

#[test]
fn close_unknown_id_is_err_not_found() {
    let mut tree = three_leaf_tree();
    assert_eq!(tree.close(999), Err(CloseError::NotFound));
    assert_eq!(tree.leaves().len(), 3);
}

#[test]
fn leaves_are_in_order_and_first_leaf_matches() {
    let tree = three_leaf_tree();
    assert_eq!(
        tree.leaves(),
        vec![
            (1, PanelKind::Graph),
            (2, PanelKind::Settings),
            (3, PanelKind::NodeList),
        ]
    );
    assert_eq!(tree.first_leaf(), 1);
    assert!(tree.contains(2));
    assert!(!tree.contains(42));
}

#[test]
fn set_kind_changes_leaf_kind() {
    let mut tree = three_leaf_tree();
    assert!(tree.set_kind(2, PanelKind::Preview3D));
    assert_eq!(tree.leaves()[1], (2, PanelKind::Preview3D));
    assert!(!tree.set_kind(42, PanelKind::Graph));
}

#[test]
fn layout_tiles_rect_without_overlap_and_covers_area() {
    let tree = three_leaf_tree();
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 720.0));
    let layout = tree.layout(rect);

    assert_eq!(layout.leaves.len(), 3);
    assert_eq!(layout.splitters.len(), 2);

    // No two leaf rects overlap (shared edges are fine).
    for (i, (_, _, a)) in layout.leaves.iter().enumerate() {
        for (_, _, b) in layout.leaves.iter().skip(i + 1) {
            let overlap = a.intersect(*b);
            assert!(
                overlap.width() <= 0.0 || overlap.height() <= 0.0,
                "leaf rects overlap: {a:?} vs {b:?}"
            );
        }
    }

    // Leaves + splitters exactly cover the area.
    let leaf_area: f32 = layout.leaves.iter().map(|(_, _, r)| r.area()).sum();
    let splitter_area: f32 = layout.splitters.iter().map(|s| s.rect.area()).sum();
    let total = rect.area();
    assert!(
        (leaf_area + splitter_area - total).abs() < 1.0,
        "areas: leaves {leaf_area} + splitters {splitter_area} != {total}"
    );

    // Spot-check edges line up: the root split is a Row at fraction 0.5.
    let (_, _, left) = layout.leaves[0];
    let (_, _, top_right) = layout.leaves[1];
    let (_, _, bottom_right) = layout.leaves[2];
    let root_splitter = &layout.splitters[0];
    assert_eq!(left.min, rect.min);
    assert_eq!(left.max.y, rect.max.y);
    assert_eq!(left.max.x, root_splitter.rect.min.x);
    assert_eq!(root_splitter.rect.width(), SPLITTER_WIDTH);
    assert_eq!(top_right.min.x, root_splitter.rect.max.x);
    assert_eq!(bottom_right.max, rect.max);
    assert_eq!(top_right.max.y, layout.splitters[1].rect.min.y);
    assert_eq!(bottom_right.min.y, layout.splitters[1].rect.max.y);
    // Column splitter spans the right column's full width.
    assert_eq!(layout.splitters[1].rect.min.x, top_right.min.x);
    assert_eq!(layout.splitters[1].rect.max.x, rect.max.x);
    // parent_rect is the split node's whole rect.
    assert_eq!(root_splitter.parent_rect, rect);
}

#[test]
fn layout_clamps_extreme_fractions_to_min_panel_size() {
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 500.0));
    for fraction in [0.001, 0.999] {
        let tree = PanelTree {
            root: PanelNode::Split {
                direction: SplitDirection::Row,
                fraction,
                children: [
                    Box::new(PanelNode::Leaf {
                        id: 1,
                        kind: PanelKind::Graph,
                    }),
                    Box::new(PanelNode::Leaf {
                        id: 2,
                        kind: PanelKind::Settings,
                    }),
                ],
            },
        };
        let layout = tree.layout(rect);
        for (_, _, r) in &layout.leaves {
            assert!(
                r.width() >= MIN_PANEL_SIZE - 0.001,
                "fraction {fraction}: leaf width {} < MIN_PANEL_SIZE",
                r.width()
            );
        }
    }
}

#[test]
fn set_fraction_by_path_clamps_and_targets_the_right_split() {
    let mut tree = three_leaf_tree();

    // Path [1] = the inner Column split.
    tree.set_fraction(&[1], 0.7);
    let PanelNode::Split { children, .. } = &tree.root else {
        panic!("root should be a split");
    };
    let PanelNode::Split { fraction, .. } = &*children[1] else {
        panic!("second child should be a split");
    };
    assert_eq!(*fraction, 0.7);

    // Root split via empty path, out-of-range value clamps.
    tree.set_fraction(&[], 1.5);
    let PanelNode::Split { fraction, .. } = &tree.root else {
        panic!("root should be a split");
    };
    assert_eq!(*fraction, 0.95);

    // Paths that do not resolve to a split are no-ops.
    tree.set_fraction(&[0], 0.3);
    tree.set_fraction(&[1, 0, 0], 0.3);
    assert_eq!(tree.leaves().len(), 3);
}

#[test]
fn serde_json_round_trip_preserves_nested_tree() {
    let tree = three_leaf_tree();
    let json = serde_json::to_string(&tree).expect("serialize");
    let back: PanelTree = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(tree, back);
}

#[test]
fn reassign_ids_gives_unique_fresh_ids_preserving_kinds_and_structure() {
    let mut tree = three_leaf_tree();
    // Force duplicate ids to prove they get fixed.
    tree.set_kind(2, PanelKind::Preview2D);
    if let PanelNode::Split { children, .. } = &mut tree.root {
        if let PanelNode::Leaf { id, .. } = &mut *children[0] {
            *id = 3;
        }
    }
    let kinds_before: Vec<PanelKind> = tree.leaves().iter().map(|(_, k)| *k).collect();

    let mut next_id: LeafId = 100;
    tree.reassign_ids(&mut next_id);

    let leaves = tree.leaves();
    assert_eq!(leaves.iter().map(|(id, _)| *id).collect::<Vec<_>>(), vec![100, 101, 102]);
    assert_eq!(next_id, 103);
    let kinds_after: Vec<PanelKind> = leaves.iter().map(|(_, k)| *k).collect();
    assert_eq!(kinds_before, kinds_after);

    // Structure (directions/fractions) unchanged.
    let PanelNode::Split {
        direction: SplitDirection::Row,
        fraction,
        children,
    } = &tree.root
    else {
        panic!("root shape changed");
    };
    assert_eq!(*fraction, 0.5);
    assert!(matches!(
        &*children[1],
        PanelNode::Split {
            direction: SplitDirection::Column,
            ..
        }
    ));
}

#[test]
fn system_default_is_three_columns_with_expected_widths() {
    let mut next_id: LeafId = 1;
    let work_width = 1280.0;
    let tree = PanelTree::system_default(work_width, &mut next_id);
    assert_eq!(next_id, 4, "three leaf ids allocated");

    let leaves = tree.leaves();
    assert_eq!(leaves.len(), 3);
    assert_eq!(
        leaves.iter().map(|(_, k)| *k).collect::<Vec<_>>(),
        vec![PanelKind::NodeList, PanelKind::Graph, PanelKind::Settings]
    );
    // Ids are unique.
    let mut ids: Vec<LeafId> = leaves.iter().map(|(id, _)| *id).collect();
    ids.dedup();
    assert_eq!(ids.len(), 3);

    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(work_width, 720.0));
    let layout = tree.layout(rect);
    let tolerance = SPLITTER_WIDTH * 2.0;

    // Left-to-right ordering with expected column widths.
    let (_, node_list_kind, node_list_rect) = layout.leaves[0];
    let (_, graph_kind, graph_rect) = layout.leaves[1];
    let (_, settings_kind, settings_rect) = layout.leaves[2];
    assert_eq!(node_list_kind, PanelKind::NodeList);
    assert_eq!(graph_kind, PanelKind::Graph);
    assert_eq!(settings_kind, PanelKind::Settings);
    assert!(node_list_rect.max.x <= graph_rect.min.x);
    assert!(graph_rect.max.x <= settings_rect.min.x);

    assert!(
        (node_list_rect.width() - crate::NODE_MENU_WIDTH).abs() <= tolerance,
        "node list width {} != ~{}",
        node_list_rect.width(),
        crate::NODE_MENU_WIDTH
    );
    assert!(
        (settings_rect.width() - crate::SETTINGS_PANEL_WIDTH).abs() <= tolerance,
        "settings width {} != ~{}",
        settings_rect.width(),
        crate::SETTINGS_PANEL_WIDTH
    );
}

/// Three columns A(1) | B(2) | C(3), built as Row(A, Row(B, C)).
fn three_columns() -> PanelTree {
    PanelTree {
        root: PanelNode::Split {
            direction: SplitDirection::Row,
            fraction: 0.25,
            children: [
                Box::new(PanelNode::Leaf {
                    id: 1,
                    kind: PanelKind::NodeList,
                }),
                Box::new(PanelNode::Split {
                    direction: SplitDirection::Row,
                    fraction: 0.7,
                    children: [
                        Box::new(PanelNode::Leaf {
                            id: 2,
                            kind: PanelKind::Graph,
                        }),
                        Box::new(PanelNode::Leaf {
                            id: 3,
                            kind: PanelKind::Settings,
                        }),
                    ],
                }),
            ],
        },
    }
}

/// Width of the leaf with `id` in a laid-out tree.
fn width_of(layout: &TreeLayout, id: LeafId) -> f32 {
    layout
        .leaves
        .iter()
        .find(|(lid, _, _)| *lid == id)
        .map(|(_, _, r)| r.width())
        .unwrap_or_else(|| panic!("no leaf {id}"))
}

/// The splitter whose path matches `path` in a laid-out tree.
fn splitter_at<'a>(layout: &'a TreeLayout, path: &[usize]) -> &'a Splitter {
    layout
        .splitters
        .iter()
        .find(|s| s.path == path)
        .expect("splitter with that path")
}

#[test]
fn drag_first_divider_only_resizes_adjacent_columns() {
    let mut tree = three_columns();
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 720.0));

    let before = tree.layout(rect);
    let (wa0, wb0, wc0) = (width_of(&before, 1), width_of(&before, 2), width_of(&before, 3));

    // First divider = the root split (path []). Push it 50px to the right.
    let splitter = splitter_at(&before, &[]);
    let pointer = splitter.rect.min.x + 50.0;
    assert!(tree.drag_splitter(&[], splitter.parent_rect, pointer));

    let after = tree.layout(rect);
    let (wa1, wb1, wc1) = (width_of(&after, 1), width_of(&after, 2), width_of(&after, 3));

    // A grows by 50, B shrinks by 50, C keeps its exact pixel width.
    assert!((wa1 - (wa0 + 50.0)).abs() < 0.5, "A: {wa0} -> {wa1}");
    assert!((wb1 - (wb0 - 50.0)).abs() < 0.5, "B: {wb0} -> {wb1}");
    assert!((wc1 - wc0).abs() < 0.5, "C changed: {wc0} -> {wc1}");
}

#[test]
fn drag_second_divider_only_resizes_adjacent_columns() {
    let mut tree = three_columns();
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 720.0));

    let before = tree.layout(rect);
    let (wa0, wb0, wc0) = (width_of(&before, 1), width_of(&before, 2), width_of(&before, 3));

    // Second divider = the inner split (path [1]). Pull it 30px to the left.
    let splitter = splitter_at(&before, &[1]);
    let pointer = splitter.rect.min.x - 30.0;
    assert!(tree.drag_splitter(&[1], splitter.parent_rect, pointer));

    let after = tree.layout(rect);
    let (wa1, wb1, wc1) = (width_of(&after, 1), width_of(&after, 2), width_of(&after, 3));

    // Divider moves left: B shrinks by 30, C grows by 30, A untouched.
    assert!((wa1 - wa0).abs() < 0.5, "A changed: {wa0} -> {wa1}");
    assert!((wb1 - (wb0 - 30.0)).abs() < 0.5, "B: {wb0} -> {wb1}");
    assert!((wc1 - (wc0 + 30.0)).abs() < 0.5, "C: {wc0} -> {wc1}");
}

#[test]
fn drag_outer_divider_with_nested_perpendicular_group() {
    // Row(A, Column(B, Row(C, D))): the outer divider's far side is a Column
    // whose leading-edge leaves along x are B and C; D does not touch it.
    let mut tree = PanelTree {
        root: PanelNode::Split {
            direction: SplitDirection::Row,
            fraction: 0.25,
            children: [
                Box::new(PanelNode::Leaf {
                    id: 1,
                    kind: PanelKind::NodeList,
                }),
                Box::new(PanelNode::Split {
                    direction: SplitDirection::Column,
                    fraction: 0.5,
                    children: [
                        Box::new(PanelNode::Leaf {
                            id: 2,
                            kind: PanelKind::Graph,
                        }),
                        Box::new(PanelNode::Split {
                            direction: SplitDirection::Row,
                            fraction: 0.5,
                            children: [
                                Box::new(PanelNode::Leaf {
                                    id: 3,
                                    kind: PanelKind::Settings,
                                }),
                                Box::new(PanelNode::Leaf {
                                    id: 4,
                                    kind: PanelKind::NodeList,
                                }),
                            ],
                        }),
                    ],
                }),
            ],
        },
    };
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 720.0));

    let before = tree.layout(rect);
    let (wa0, wb0, wc0, wd0) = (
        width_of(&before, 1),
        width_of(&before, 2),
        width_of(&before, 3),
        width_of(&before, 4),
    );

    let splitter = splitter_at(&before, &[]);
    let pointer = splitter.rect.min.x + 40.0;
    assert!(tree.drag_splitter(&[], splitter.parent_rect, pointer));

    let after = tree.layout(rect);
    let (wa1, wb1, wc1, wd1) = (
        width_of(&after, 1),
        width_of(&after, 2),
        width_of(&after, 3),
        width_of(&after, 4),
    );

    // A grows by 40; the divider-adjacent leaves B and C each shrink by 40;
    // D (not touching the divider) keeps its exact pixel width.
    assert!((wa1 - (wa0 + 40.0)).abs() < 0.5, "A: {wa0} -> {wa1}");
    assert!((wb1 - (wb0 - 40.0)).abs() < 0.5, "B: {wb0} -> {wb1}");
    assert!((wc1 - (wc0 - 40.0)).abs() < 0.5, "C: {wc0} -> {wc1}");
    assert!((wd1 - wd0).abs() < 0.5, "D changed: {wd0} -> {wd1}");
}

#[test]
fn drag_is_noop_when_adjacent_panel_already_at_min() {
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 720.0));
    // Root first extent = 0.25 * (1280 - 4) = 319, so the inner split spans
    // 957px -> inner available 953px. Set B to exactly MIN_PANEL_SIZE.
    let inner_fraction = MIN_PANEL_SIZE / 953.0;
    let mut tree = PanelTree {
        root: PanelNode::Split {
            direction: SplitDirection::Row,
            fraction: 0.25,
            children: [
                Box::new(PanelNode::Leaf {
                    id: 1,
                    kind: PanelKind::NodeList,
                }),
                Box::new(PanelNode::Split {
                    direction: SplitDirection::Row,
                    fraction: inner_fraction,
                    children: [
                        Box::new(PanelNode::Leaf {
                            id: 2,
                            kind: PanelKind::Graph,
                        }),
                        Box::new(PanelNode::Leaf {
                            id: 3,
                            kind: PanelKind::Settings,
                        }),
                    ],
                }),
            ],
        },
    };

    let before = tree.layout(rect);
    assert!(
        (width_of(&before, 2) - MIN_PANEL_SIZE).abs() < 0.5,
        "B should start at MIN_PANEL_SIZE, got {}",
        width_of(&before, 2)
    );
    let (wa0, wb0, wc0) = (width_of(&before, 1), width_of(&before, 2), width_of(&before, 3));

    // Dragging the first divider further right would shrink B below MIN: no-op.
    let splitter = splitter_at(&before, &[]);
    let pointer = splitter.rect.min.x + 50.0;
    assert!(!tree.drag_splitter(&[], splitter.parent_rect, pointer));

    let after = tree.layout(rect);
    assert!((width_of(&after, 1) - wa0).abs() < 0.5);
    assert!((width_of(&after, 2) - wb0).abs() < 0.5);
    assert!((width_of(&after, 3) - wc0).abs() < 0.5);
}

#[test]
fn system_default_survives_tiny_work_width() {
    let mut next_id: LeafId = 1;
    for width in [0.0, 10.0, 100.0] {
        let tree = PanelTree::system_default(width, &mut next_id);
        assert_eq!(tree.leaves().len(), 3);
        // All fractions stay in the clamped range.
        fn check(node: &PanelNode) {
            if let PanelNode::Split {
                fraction, children, ..
            } = node
            {
                assert!(
                    (0.05..=0.95).contains(fraction),
                    "fraction {fraction} out of range for tiny width"
                );
                for child in children {
                    check(child);
                }
            }
        }
        check(&tree.root);
    }
}
