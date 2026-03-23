use super::*;
use egui::{Pos2, Vec2};
use mangler_core::input::Input;
use mangler_core::node_settings::NodeSettings;
use mangler_core::output::Output;
use mangler_core::value::Value;

/// Helper to add a node at a given position with no inputs/outputs.
fn add_node(editor: &mut GraphEditor, id: &str, pos: Pos2) {
    editor.add_node(
        id.to_string(),
        NodeSettings::default(),
        vec![],
        vec![],
        pos,
        false,
        None,
        true,
        None,
    );
}

/// Helper to add a node with inputs and outputs.
fn add_node_with_io(
    editor: &mut GraphEditor,
    id: &str,
    pos: Pos2,
    num_inputs: usize,
    num_outputs: usize,
) {
    let inputs: Vec<Input> = (0..num_inputs)
        .map(|i| Input::new(format!("in_{}", i), Value::Integer(0), None, None))
        .collect();
    let outputs: Vec<Output> = (0..num_outputs)
        .map(|i| Output::new(format!("out_{}", i), Value::Integer(0), None))
        .collect();
    editor.add_node(
        id.to_string(),
        NodeSettings::default(),
        inputs,
        outputs,
        pos,
        false,
        None,
        true,
        None,
    );
}

/// Helper to add a node at origin (0,0) with configurable inputs/outputs and connections.
/// `connected_inputs` is a list of (upstream_node_id, output_index) for each input that should
/// be connected. Unconnected inputs use None.
fn add_connected_node(
    editor: &mut GraphEditor,
    id: &str,
    name: &str,
    num_inputs: usize,
    num_outputs: usize,
    connected_inputs: Vec<Option<(String, usize)>>,
) {
    let inputs: Vec<Input> = (0..num_inputs)
        .map(|i| {
            let mut input = Input::new(format!("in_{}", i), Value::Integer(0), None, None);
            if let Some(conn) = connected_inputs.get(i).cloned().flatten() {
                input.connection = Some(conn);
            }
            input
        })
        .collect();
    let outputs: Vec<Output> = (0..num_outputs)
        .map(|i| Output::new(format!("out_{}", i), Value::Integer(0), None))
        .collect();
    let mut settings = NodeSettings::default();
    settings.name = name.to_string();
    editor.add_node(
        id.to_string(),
        settings,
        inputs,
        outputs,
        Pos2::new(0.0, 0.0), // All at origin to trigger auto-layout.
        false,
        None,
        true,
        None,
    );
}

// -- auto_layout tests --

#[test]
fn test_auto_layout_linear_chain_left_to_right() {
    // A -> B -> C should lay out in three columns, left to right.
    let mut editor = GraphEditor::new();
    add_connected_node(&mut editor, "a", "A", 0, 1, vec![]);
    add_connected_node(&mut editor, "b", "B", 1, 1, vec![Some(("a".into(), 0))]);
    add_connected_node(&mut editor, "c", "C", 1, 0, vec![Some(("b".into(), 0))]);

    let moved = editor.auto_layout_if_needed();
    assert_eq!(moved.len(), 3);

    let pos_a = editor.graph_nodes["a"].position;
    let pos_b = editor.graph_nodes["b"].position;
    let pos_c = editor.graph_nodes["c"].position;

    // A is in column 0, B in column 1, C in column 2.
    assert!(pos_a.x < pos_b.x, "A should be left of B");
    assert!(pos_b.x < pos_c.x, "B should be left of C");

    // All on the same row (single node per column, centered at same y).
    assert!((pos_a.y - pos_b.y).abs() < 1.0, "A and B should be at same y");
    assert!((pos_b.y - pos_c.y).abs() < 1.0, "B and C should be at same y");
}

#[test]
fn test_auto_layout_fan_out_groups_downstream() {
    // A fans out to B and C. D is connected to B only.
    // A -> B -> D
    // A -> C
    // B should be closer to D's upstream, and C should be near B (both from A).
    let mut editor = GraphEditor::new();
    add_connected_node(&mut editor, "a", "A", 0, 1, vec![]);
    add_connected_node(&mut editor, "b", "B", 1, 1, vec![Some(("a".into(), 0))]);
    add_connected_node(&mut editor, "c", "C", 1, 0, vec![Some(("a".into(), 0))]);
    add_connected_node(&mut editor, "d", "D", 1, 0, vec![Some(("b".into(), 0))]);

    editor.auto_layout_if_needed();

    let pos_b = editor.graph_nodes["b"].position;
    let pos_c = editor.graph_nodes["c"].position;
    let pos_d = editor.graph_nodes["d"].position;

    // B and C are in the same column (column 1).
    assert!((pos_b.x - pos_c.x).abs() < 1.0, "B and C should be in same column");

    // D is in column 2, to the right of B.
    assert!(pos_d.x > pos_b.x, "D should be right of B");

    // After barycenter: B should be vertically closer to D than C is to D,
    // because B connects to D.
    let b_to_d = (pos_b.y - pos_d.y).abs();
    let c_to_d = (pos_c.y - pos_d.y).abs();
    assert!(b_to_d <= c_to_d, "B (connected to D) should be closer to D than C");
}

#[test]
fn test_auto_layout_orphans_at_bottom() {
    // A -> B, and an orphan node Z with no connections.
    // Z should be placed below A (both in column 0).
    let mut editor = GraphEditor::new();
    add_connected_node(&mut editor, "a", "A", 0, 1, vec![]);
    add_connected_node(&mut editor, "z", "Z_orphan", 0, 0, vec![]);
    add_connected_node(&mut editor, "b", "B", 1, 0, vec![Some(("a".into(), 0))]);

    editor.auto_layout_if_needed();

    let pos_a = editor.graph_nodes["a"].position;
    let pos_z = editor.graph_nodes["z"].position;

    // Both A and Z are in column 0 (same x).
    assert!((pos_a.x - pos_z.x).abs() < 1.0, "A and Z should be in same column");

    // Z (orphan) should be below A after barycenter sorting,
    // because A has downstream connections and gets a real barycenter,
    // while Z gets f32::MAX.
    assert!(pos_z.y > pos_a.y, "Orphan Z should be below connected node A");
}

#[test]
fn test_auto_layout_no_layout_when_not_overlapping() {
    // Nodes at different positions should not be auto-laid out.
    let mut editor = GraphEditor::new();
    add_node(&mut editor, "a", Pos2::new(100.0, 100.0));
    add_node(&mut editor, "b", Pos2::new(300.0, 300.0));

    let moved = editor.auto_layout_if_needed();
    assert!(moved.is_empty());
}

#[test]
fn test_auto_layout_single_node_no_layout() {
    // A single node should not trigger layout.
    let mut editor = GraphEditor::new();
    add_node(&mut editor, "a", Pos2::new(0.0, 0.0));

    let moved = editor.auto_layout_if_needed();
    assert!(moved.is_empty());
}

// -- apply_multi_drag tests --

#[test]
fn test_multi_drag_moves_all_selected_nodes() {
    // When multiple nodes are selected and one is dragged, all other
    // selected nodes should move by the same delta.
    let mut editor = GraphEditor::new();
    add_node(&mut editor, "a", Pos2::new(100.0, 100.0));
    add_node(&mut editor, "b", Pos2::new(200.0, 200.0));
    add_node(&mut editor, "c", Pos2::new(300.0, 300.0));

    editor.selected_node_ids.insert("a".to_string());
    editor.selected_node_ids.insert("b".to_string());
    editor.selected_node_ids.insert("c".to_string());

    let delta = Vec2::new(10.0, -5.0);
    let moved = editor.apply_multi_drag("a", delta);

    // "a" is the dragged node — it should NOT be moved by apply_multi_drag
    // (it was already moved by the node's own drag logic).
    assert_eq!(editor.graph_nodes["a"].position, Pos2::new(100.0, 100.0));

    // "b" and "c" should be moved by the delta.
    assert_eq!(editor.graph_nodes["b"].position, Pos2::new(210.0, 195.0));
    assert_eq!(editor.graph_nodes["c"].position, Pos2::new(310.0, 295.0));

    // The returned vec should contain "b" and "c" with their new positions.
    assert_eq!(moved.len(), 2);
    let moved_ids: std::collections::HashSet<String> = moved.iter().map(|(id, _)| id.clone()).collect();
    assert!(moved_ids.contains("b"));
    assert!(moved_ids.contains("c"));
}

#[test]
fn test_multi_drag_skips_dragged_node() {
    // The node that initiated the drag should not be moved a second time.
    let mut editor = GraphEditor::new();
    add_node(&mut editor, "a", Pos2::new(50.0, 50.0));

    editor.selected_node_ids.insert("a".to_string());

    let delta = Vec2::new(20.0, 20.0);
    let moved = editor.apply_multi_drag("a", delta);

    // No other nodes to move — position unchanged.
    assert_eq!(editor.graph_nodes["a"].position, Pos2::new(50.0, 50.0));
    assert!(moved.is_empty());
}

#[test]
fn test_multi_drag_unselected_nodes_not_moved() {
    // Nodes that are not in the selection should not be affected by multi-drag.
    let mut editor = GraphEditor::new();
    add_node(&mut editor, "a", Pos2::new(100.0, 100.0));
    add_node(&mut editor, "b", Pos2::new(200.0, 200.0));
    add_node(&mut editor, "c", Pos2::new(300.0, 300.0));

    // Only "a" and "b" are selected; "c" is not.
    editor.selected_node_ids.insert("a".to_string());
    editor.selected_node_ids.insert("b".to_string());

    let delta = Vec2::new(15.0, -10.0);
    let moved = editor.apply_multi_drag("a", delta);

    // "b" moved, "c" untouched.
    assert_eq!(editor.graph_nodes["b"].position, Pos2::new(215.0, 190.0));
    assert_eq!(editor.graph_nodes["c"].position, Pos2::new(300.0, 300.0));

    assert_eq!(moved.len(), 1);
    assert_eq!(moved[0].0, "b");
}

#[test]
fn test_multi_drag_no_selection() {
    // If no nodes are selected, apply_multi_drag should move nothing.
    let mut editor = GraphEditor::new();
    add_node(&mut editor, "a", Pos2::new(100.0, 100.0));
    add_node(&mut editor, "b", Pos2::new(200.0, 200.0));

    let delta = Vec2::new(50.0, 50.0);
    let moved = editor.apply_multi_drag("a", delta);

    assert!(moved.is_empty());
    assert_eq!(editor.graph_nodes["a"].position, Pos2::new(100.0, 100.0));
    assert_eq!(editor.graph_nodes["b"].position, Pos2::new(200.0, 200.0));
}

#[test]
fn test_multi_drag_selected_node_not_in_graph() {
    // If a selected node ID doesn't exist in graph_nodes (e.g. stale selection),
    // it should be silently skipped without panicking.
    let mut editor = GraphEditor::new();
    add_node(&mut editor, "a", Pos2::new(100.0, 100.0));

    editor.selected_node_ids.insert("a".to_string());
    editor.selected_node_ids.insert("nonexistent".to_string());

    let delta = Vec2::new(10.0, 10.0);
    let moved = editor.apply_multi_drag("a", delta);

    // Only "nonexistent" would be a candidate but it doesn't exist, so nothing moves.
    assert!(moved.is_empty());
    assert_eq!(editor.graph_nodes["a"].position, Pos2::new(100.0, 100.0));
}

#[test]
fn test_multi_drag_accumulates_over_multiple_frames() {
    // Simulates multiple drag frames — delta applied repeatedly should accumulate.
    let mut editor = GraphEditor::new();
    add_node(&mut editor, "a", Pos2::new(0.0, 0.0));
    add_node(&mut editor, "b", Pos2::new(100.0, 100.0));

    editor.selected_node_ids.insert("a".to_string());
    editor.selected_node_ids.insert("b".to_string());

    let delta = Vec2::new(5.0, 5.0);

    // Frame 1
    editor.apply_multi_drag("a", delta);
    assert_eq!(editor.graph_nodes["b"].position, Pos2::new(105.0, 105.0));

    // Frame 2
    editor.apply_multi_drag("a", delta);
    assert_eq!(editor.graph_nodes["b"].position, Pos2::new(110.0, 110.0));

    // Frame 3
    editor.apply_multi_drag("a", delta);
    assert_eq!(editor.graph_nodes["b"].position, Pos2::new(115.0, 115.0));
}

#[test]
fn test_multi_drag_zero_delta() {
    // A zero delta should not change any positions.
    let mut editor = GraphEditor::new();
    add_node(&mut editor, "a", Pos2::new(100.0, 200.0));
    add_node(&mut editor, "b", Pos2::new(300.0, 400.0));

    editor.selected_node_ids.insert("a".to_string());
    editor.selected_node_ids.insert("b".to_string());

    let moved = editor.apply_multi_drag("a", Vec2::ZERO);

    // "b" is returned (it was processed) but position is unchanged.
    assert_eq!(moved.len(), 1);
    assert_eq!(editor.graph_nodes["b"].position, Pos2::new(300.0, 400.0));
}

#[test]
fn test_multi_drag_preserves_relative_positions() {
    // After a multi-drag, the relative positions between all selected nodes
    // should be preserved (same offsets as before).
    let mut editor = GraphEditor::new();
    add_node(&mut editor, "a", Pos2::new(10.0, 20.0));
    add_node(&mut editor, "b", Pos2::new(50.0, 80.0));
    add_node(&mut editor, "c", Pos2::new(100.0, 30.0));

    // Relative offsets from "a": b=(40,60), c=(90,10)
    editor.selected_node_ids.insert("a".to_string());
    editor.selected_node_ids.insert("b".to_string());
    editor.selected_node_ids.insert("c".to_string());

    let delta = Vec2::new(25.0, -15.0);

    // Simulate: "a" was already moved by its own drag logic.
    editor.graph_nodes.get_mut("a").unwrap().position += delta;
    // Now apply to others.
    editor.apply_multi_drag("a", delta);

    let a_pos = editor.graph_nodes["a"].position;
    let b_pos = editor.graph_nodes["b"].position;
    let c_pos = editor.graph_nodes["c"].position;

    // Relative offsets should be the same: b-a=(40,60), c-a=(90,10).
    let b_offset = b_pos - a_pos;
    let c_offset = c_pos - a_pos;
    assert!((b_offset.x - 40.0).abs() < 0.001);
    assert!((b_offset.y - 60.0).abs() < 0.001);
    assert!((c_offset.x - 90.0).abs() < 0.001);
    assert!((c_offset.y - 10.0).abs() < 0.001);
}
