use std::collections::HashMap;
use std::sync::Arc;
use mangler_core::float_image::FloatImage;
use mangler_core::node_settings::NodeSettings;
use mangler_core::output::Output;
use mangler_core::value::Value;
use crate::graph::graph_node::GraphNode;
use epaint::Pos2;

use super::*;

fn make_image_value() -> Value {
    Value::Image {
        data: Arc::new(FloatImage::new(4, 4, 3)),
        change_id: "test_change".to_string(),
    }
}

fn make_output(name: &str, value: Value) -> Output {
    Output::new(name.to_string(), value, None)
}

fn make_node(id: &str, name: &str, outputs: Vec<Output>) -> GraphNode {
    GraphNode::new(
        id.to_string(),
        Pos2::ZERO,
        NodeSettings { name: name.to_string(), description: String::new() },
        vec![],
        outputs,
        false,
    )
}

fn make_graph_with_named_outputs(outputs: Vec<(&str, &str, &str)>) -> HashMap<String, GraphNode> {
    let mut nodes = HashMap::new();
    for (node_id, node_name, output_name) in outputs {
        let output = make_output(output_name, make_image_value());
        let node = make_node(node_id, node_name, vec![output]);
        nodes.insert(node_id.to_string(), node);
    }
    nodes
}

#[test]
fn set_and_get_assignment() {
    let mut assignments = MaterialChannelAssignments::new();
    assert!(assignments.get(MaterialChannel::Albedo).is_none());

    let a = MaterialAssignment {
        node_id: "node_1".to_string(),
        output_index: 0,
    };
    assignments.set(MaterialChannel::Albedo, a.clone());

    let got = assignments.get(MaterialChannel::Albedo).unwrap();
    assert_eq!(got.node_id, "node_1");
    assert_eq!(got.output_index, 0);
}

#[test]
fn clear_assignment() {
    let mut assignments = MaterialChannelAssignments::new();
    assignments.set(MaterialChannel::Normal, MaterialAssignment {
        node_id: "n".to_string(),
        output_index: 0,
    });
    assert!(assignments.get(MaterialChannel::Normal).is_some());

    assignments.clear(MaterialChannel::Normal);
    assert!(assignments.get(MaterialChannel::Normal).is_none());
}

#[test]
fn clear_all_assignments() {
    let mut assignments = MaterialChannelAssignments::new();
    for channel in MaterialChannel::ALL {
        assignments.set(channel, MaterialAssignment {
            node_id: "n".to_string(),
            output_index: 0,
        });
    }
    assignments.clear_all();
    for channel in MaterialChannel::ALL {
        assert!(assignments.get(channel).is_none());
    }
}

#[test]
fn auto_detect_by_output_name() {
    let graph = make_graph_with_named_outputs(vec![
        ("n1", "Generator", "Normal"),
        ("n2", "Generator", "Height"),
        ("n3", "Generator", "Albedo"),
    ]);

    let mut assignments = MaterialChannelAssignments::new();
    assignments.auto_detect("viewed", &graph);

    assert!(assignments.get(MaterialChannel::Normal).is_some());
    assert!(assignments.get(MaterialChannel::Height).is_some());
    assert!(assignments.get(MaterialChannel::Albedo).is_some());
    // These have no matching outputs
    assert!(assignments.get(MaterialChannel::Roughness).is_none());
    assert!(assignments.get(MaterialChannel::Metallic).is_none());
}

#[test]
fn auto_detect_by_node_name() {
    let graph = make_graph_with_named_outputs(vec![
        ("n1", "Normal From Height", "Output"),
        ("n2", "AO From Height", "Output"),
    ]);

    let mut assignments = MaterialChannelAssignments::new();
    assignments.auto_detect("viewed", &graph);

    assert!(assignments.get(MaterialChannel::Normal).is_some());
    assert!(assignments.get(MaterialChannel::AmbientOcclusion).is_some());
}

#[test]
fn auto_detect_skips_non_image_outputs() {
    let mut nodes = HashMap::new();
    let output = make_output("Normal", Value::Decimal(0.5));
    let node = make_node("n1", "Node", vec![output]);
    nodes.insert("n1".to_string(), node);

    let mut assignments = MaterialChannelAssignments::new();
    assignments.auto_detect("viewed", &nodes);

    assert!(assignments.get(MaterialChannel::Normal).is_none());
}

#[test]
fn auto_detect_only_runs_once_per_node() {
    let graph = make_graph_with_named_outputs(vec![
        ("n1", "Gen", "Albedo"),
    ]);

    let mut assignments = MaterialChannelAssignments::new();
    assignments.auto_detect("node_a", &graph);
    assert!(assignments.get(MaterialChannel::Albedo).is_some());

    // Clear manually, then auto_detect with same node_id — should not re-detect
    assignments.clear(MaterialChannel::Albedo);
    assignments.auto_detect("node_a", &graph);
    assert!(assignments.get(MaterialChannel::Albedo).is_none());

    // Different node_id — should re-detect
    assignments.auto_detect("node_b", &graph);
    assert!(assignments.get(MaterialChannel::Albedo).is_some());
}

#[test]
fn list_image_outputs_only_images() {
    let mut nodes = HashMap::new();
    let outputs = vec![
        make_output("Image Out", make_image_value()),
        make_output("Number Out", Value::Decimal(1.0)),
    ];
    let node = make_node("n1", "MyNode", outputs);
    nodes.insert("n1".to_string(), node);

    let list = list_image_outputs(&nodes);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].0, "n1");
    assert_eq!(list[0].1, 0);
    assert!(list[0].2.contains("Image Out"));
}

#[test]
fn list_image_outputs_sorted() {
    let graph = make_graph_with_named_outputs(vec![
        ("n2", "Zebra", "Output"),
        ("n1", "Alpha", "Output"),
    ]);

    let list = list_image_outputs(&graph);
    assert_eq!(list.len(), 2);
    // Sorted by label
    assert!(list[0].2 < list[1].2);
}

#[test]
fn resolve_material_returns_image_data() {
    let graph = make_graph_with_named_outputs(vec![
        ("n1", "Gen", "Albedo"),
    ]);

    let mut assignments = MaterialChannelAssignments::new();
    assignments.set(MaterialChannel::Albedo, MaterialAssignment {
        node_id: "n1".to_string(),
        output_index: 0,
    });

    let material = resolve_material(&assignments, &graph);
    assert!(material.albedo.is_some());
    assert!(material.normal.is_none());
}

#[test]
fn resolve_material_missing_node() {
    let graph: HashMap<String, GraphNode> = HashMap::new();

    let mut assignments = MaterialChannelAssignments::new();
    assignments.set(MaterialChannel::Albedo, MaterialAssignment {
        node_id: "nonexistent".to_string(),
        output_index: 0,
    });

    let material = resolve_material(&assignments, &graph);
    assert!(material.albedo.is_none());
}

#[test]
fn channel_labels() {
    assert_eq!(MaterialChannel::Albedo.label(), "Albedo");
    assert_eq!(MaterialChannel::AmbientOcclusion.label(), "AO");
    assert_eq!(MaterialChannel::Normal.label(), "Normal");
}
