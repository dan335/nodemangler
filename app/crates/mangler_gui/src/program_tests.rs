use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use eframe::egui;

use mangler_core::WatchStopReason;

use super::{
    batch_finished_message, collect_selected_nodes_to_delete, detect_copy_paste, poll_preview_slot,
    watch_stopped_message, PreviewPoll,
};

// === Event::Copy ===

#[test]
fn event_copy_triggers_copy() {
    let events = vec![egui::Event::Copy];
    let (copy, paste) = detect_copy_paste(&events);
    assert!(copy);
    assert!(paste.is_none());
}

// === Event::Paste ===

#[test]
fn event_paste_returns_text() {
    let events = vec![egui::Event::Paste("hello".to_string())];
    let (copy, paste) = detect_copy_paste(&events);
    assert!(!copy);
    assert_eq!(paste.as_deref(), Some("hello"));
}

#[test]
fn event_paste_empty_string_returns_some() {
    let events = vec![egui::Event::Paste(String::new())];
    let (_, paste) = detect_copy_paste(&events);
    assert_eq!(paste.as_deref(), Some(""));
}

#[test]
fn event_paste_with_node_data_returns_text() {
    // Simulates pasting clipboard data that was serialized by our copy handler.
    let text = "NODEMANGLER:{\"nodes\":[],\"connections\":[]}";
    let events = vec![egui::Event::Paste(text.to_string())];
    let (_, paste) = detect_copy_paste(&events);
    assert_eq!(paste.as_deref(), Some(text));
}

// === No relevant events ===

#[test]
fn no_events_triggers_nothing() {
    let events: Vec<egui::Event> = vec![];
    let (copy, paste) = detect_copy_paste(&events);
    assert!(!copy);
    assert!(paste.is_none());
}

#[test]
fn unrelated_events_are_ignored() {
    let events = vec![egui::Event::Cut];
    let (copy, paste) = detect_copy_paste(&events);
    assert!(!copy);
    assert!(paste.is_none());
}

// === Both copy and paste in one frame ===

#[test]
fn copy_and_paste_in_same_frame() {
    let events = vec![
        egui::Event::Copy,
        egui::Event::Paste("text".to_string()),
    ];
    let (copy, paste) = detect_copy_paste(&events);
    assert!(copy);
    assert_eq!(paste.as_deref(), Some("text"));
}

// === Last paste wins if multiple paste events in one frame ===

#[test]
fn multiple_paste_events_last_wins() {
    let events = vec![
        egui::Event::Paste("first".to_string()),
        egui::Event::Paste("second".to_string()),
    ];
    let (_, paste) = detect_copy_paste(&events);
    assert_eq!(paste.as_deref(), Some("second"));
}

// === collect_selected_nodes_to_delete ===

#[test]
fn delete_no_selection_returns_empty() {
    let mut selected = HashSet::new();
    let mut editing = Some("a".to_string());
    let result = collect_selected_nodes_to_delete(&mut selected, &mut editing);
    assert!(result.is_empty());
    // editing_node_id is untouched when nothing is selected
    assert_eq!(editing.as_deref(), Some("a"));
}

#[test]
fn delete_single_selected_node() {
    let mut selected = HashSet::from(["a".to_string()]);
    let mut editing = Some("a".to_string());
    let result = collect_selected_nodes_to_delete(&mut selected, &mut editing);
    assert_eq!(result, vec!["a".to_string()]);
    assert!(selected.is_empty());
    assert!(editing.is_none());
}

#[test]
fn delete_multiple_selected_nodes() {
    let mut selected = HashSet::from([
        "a".to_string(),
        "b".to_string(),
        "c".to_string(),
    ]);
    let mut editing = Some("b".to_string());
    let mut result = collect_selected_nodes_to_delete(&mut selected, &mut editing);
    result.sort();
    assert_eq!(result, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    assert!(selected.is_empty());
    assert!(editing.is_none());
}

#[test]
fn delete_selected_clears_editing_even_if_not_in_selection() {
    // editing_node_id might differ from selected set; it should still be cleared.
    let mut selected = HashSet::from(["a".to_string()]);
    let mut editing = Some("z".to_string());
    let result = collect_selected_nodes_to_delete(&mut selected, &mut editing);
    assert_eq!(result, vec!["a".to_string()]);
    assert!(editing.is_none());
}

#[test]
fn delete_selected_with_no_editing_node() {
    let mut selected = HashSet::from(["a".to_string(), "b".to_string()]);
    let mut editing: Option<String> = None;
    let mut result = collect_selected_nodes_to_delete(&mut selected, &mut editing);
    result.sort();
    assert_eq!(result, vec!["a".to_string(), "b".to_string()]);
    assert!(selected.is_empty());
    assert!(editing.is_none());
}

// === poll_preview_slot (background library-image decode) ===

#[test]
fn preview_slot_empty_is_pending() {
    let slot: Mutex<Option<String>> = Mutex::new(None);
    assert_eq!(poll_preview_slot(&slot, 1, 1), PreviewPoll::Pending);
}

#[test]
fn preview_slot_current_generation_is_ready() {
    let slot = Mutex::new(Some("image".to_string()));
    assert_eq!(
        poll_preview_slot(&slot, 3, 3),
        PreviewPoll::Ready("image".to_string())
    );
}

#[test]
fn preview_slot_older_generation_is_stale() {
    // The user clicked another image while this one was still decoding.
    let slot = Mutex::new(Some("image".to_string()));
    assert_eq!(poll_preview_slot(&slot, 2, 5), PreviewPoll::Stale);
}

#[test]
fn preview_slot_ready_is_taken_out_of_the_slot() {
    // Promotion consumes the result: a second poll must not re-deliver it
    // (which would re-fit the view every frame).
    let slot = Mutex::new(Some(7));
    assert_eq!(poll_preview_slot(&slot, 1, 1), PreviewPoll::Ready(7));
    assert_eq!(poll_preview_slot(&slot, 1, 1), PreviewPoll::Pending);
}

#[test]
fn preview_slot_stale_result_is_discarded_not_left_behind() {
    let slot = Mutex::new(Some(7));
    assert_eq!(poll_preview_slot(&slot, 1, 2), PreviewPoll::Stale);
    assert!(slot.lock().unwrap().is_none());
}

#[test]
fn preview_slot_carries_decode_errors() {
    let slot: Mutex<Option<Result<u8, String>>> = Mutex::new(Some(Err("bad file".to_string())));
    assert_eq!(
        poll_preview_slot(&slot, 4, 4),
        PreviewPoll::Ready(Err("bad file".to_string()))
    );
}

#[test]
fn preview_slot_last_click_wins_across_two_decodes() {
    // Two decodes in flight, finishing out of order: only the newest one is
    // promoted, whichever lands first.
    let current = 9u64;
    let old: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let new: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    *new.lock().unwrap() = Some("new".to_string());
    assert_eq!(
        poll_preview_slot(&new, current, current),
        PreviewPoll::Ready("new".to_string())
    );

    *old.lock().unwrap() = Some("old".to_string());
    assert_eq!(poll_preview_slot(&old, current - 1, current), PreviewPoll::Stale);
}

// === watch_stopped_message ===

#[test]
fn watch_stopped_reports_the_capture_count() {
    assert_eq!(
        watch_stopped_message(12, 0, WatchStopReason::Stopped),
        "watch stopped: 12 frames captured"
    );
}

#[test]
fn watch_stopped_appends_skipped_only_when_non_zero() {
    assert_eq!(
        watch_stopped_message(12, 3, WatchStopReason::Stopped),
        "watch stopped: 12 frames captured, 3 skipped"
    );
}

#[test]
fn watch_stopped_by_node_deletion_ignores_counts() {
    assert_eq!(
        watch_stopped_message(5, 2, WatchStopReason::NodeDeleted),
        "watch stopped: node deleted"
    );
}

#[test]
fn watch_stopped_by_folder_change() {
    assert_eq!(
        watch_stopped_message(0, 0, WatchStopReason::FolderChanged),
        "watch stopped: the folder input changed"
    );
}

#[test]
fn watch_refused_explains_both_causes() {
    assert_eq!(
        watch_stopped_message(0, 0, WatchStopReason::Refused),
        "can't watch: check the node's folder, or stop the running batch"
    );
}

// === batch_finished_message ===

#[test]
fn batch_finished_reports_the_total_processed() {
    assert_eq!(
        batch_finished_message(8, 8, false, false),
        "batch finished: 8 images"
    );
}

#[test]
fn batch_cancelled_mid_run_reports_progress() {
    assert_eq!(
        batch_finished_message(3, 8, true, false),
        "batch cancelled at 3/8"
    );
}

#[test]
fn batch_refused_with_no_watch_blames_the_folder() {
    assert_eq!(
        batch_finished_message(0, 0, true, false),
        "batch: no images found in the folder"
    );
}

#[test]
fn batch_refused_while_watching_blames_the_watch() {
    // Same message shape from the engine; only the local watch state
    // distinguishes "empty folder" from "the watch already owns this node".
    assert_eq!(
        batch_finished_message(0, 0, true, true),
        "can't run a batch while watching a folder"
    );
}

#[test]
fn batch_completion_is_never_confused_by_an_active_watch() {
    // A watch can't be running during a real batch, but a stale flag must not
    // hijack a successful finish.
    assert_eq!(
        batch_finished_message(4, 4, false, true),
        "batch finished: 4 images"
    );
}


// === gizmo backdrop + preview editor resolution ===

mod preview_editor {
    use std::collections::HashMap;
    use std::sync::Arc;

    use eframe::egui::Pos2;
    use mangler_core::curve::Curve;
    use mangler_core::float_image::FloatImage;
    use mangler_core::input::{Input, InputSettings};
    use mangler_core::node_settings::NodeSettings;
    use mangler_core::operations::Operation;
    use mangler_core::output::Output;
    use mangler_core::value::Value;
    use mangler_core::AddNodeType;

    use crate::graph::graph_node::GraphNode;
    use crate::program::{gizmo_backdrop_source, resolve_preview_editor, PreviewEditor};

    fn image_value() -> Value {
        Value::Image { data: Arc::new(FloatImage::new(4, 4, 3)), change_id: "c".to_string() }
    }

    fn node(id: &str, op: Option<Operation>, inputs: Vec<Input>, outputs: Vec<Output>) -> GraphNode {
        GraphNode::new(
            id.to_string(),
            Pos2::ZERO,
            NodeSettings {
                name: id.to_string(),
                description: String::new(),
                help: String::new(),
            },
            inputs,
            outputs,
            false,
            op.map(AddNodeType::Operation),
            true,
            None,
        )
    }

    fn image_input(connection: Option<(&str, usize)>) -> Input {
        let mut input = Input::new("image".to_string(), image_value(), None, None);
        input.connection = connection.map(|(id, idx)| (id.to_string(), idx));
        input
    }

    fn number_input(name: &str) -> Input {
        Input::new(name.to_string(), Value::Decimal(0.0), None, None)
    }

    fn img_out() -> Output {
        Output::new("output".to_string(), image_value(), None)
    }

    fn curve_input(name: &str, settings: Option<InputSettings>) -> Input {
        Input::new(name.to_string(), Value::Curve(Curve::default()), settings, None)
    }

    fn graph(nodes: Vec<GraphNode>) -> HashMap<String, GraphNode> {
        nodes.into_iter().map(|n| (n.id.clone(), n)).collect()
    }

    /// A `crop`-shaped consumer: image input 0 plus four spatial numbers.
    fn crop_node(id: &str, connection: Option<(&str, usize)>) -> GraphNode {
        node(
            id,
            Some(Operation::OpImageTransformCrop),
            vec![
                image_input(connection),
                number_input("x"),
                number_input("y"),
                number_input("width"),
                number_input("height"),
            ],
            vec![img_out()],
        )
    }

    fn label(editor: &Option<PreviewEditor>) -> &'static str {
        match editor {
            Some(PreviewEditor::Curve { .. }) => "curve",
            Some(PreviewEditor::Gizmos { .. }) => "gizmos",
            None => "none",
        }
    }

    // --- backdrop resolution ---

    #[test]
    fn consumer_resolves_its_upstream_source() {
        let nodes = graph(vec![
            node("src", None, vec![], vec![img_out()]),
            crop_node("crop", Some(("src", 0))),
        ]);
        assert_eq!(gizmo_backdrop_source(&nodes, "crop"), Some(("src".to_string(), 0)));
    }

    #[test]
    fn consumer_with_nothing_connected_has_no_backdrop() {
        // Must NOT fall back to its own output: that is the 1x1 white
        // `default_image()` placeholder, and drawing a crop box on it would be
        // actively misleading. This is why the rule is a dichotomy, not a chain.
        let nodes = graph(vec![crop_node("crop", None)]);
        assert_eq!(gizmo_backdrop_source(&nodes, "crop"), None);
    }

    #[test]
    fn producer_uses_its_own_first_image_output() {
        // A shape node makes its image from nothing, so its own output IS the
        // coordinate space its handles live in.
        let nodes = graph(vec![node(
            "line",
            None,
            vec![number_input("start x")],
            vec![Output::new("count".to_string(), Value::Integer(0), None), img_out()],
        )]);
        assert_eq!(gizmo_backdrop_source(&nodes, "line"), Some(("line".to_string(), 1)));
    }

    #[test]
    fn the_first_connected_image_input_wins() {
        // `blit`-shaped: two image inputs, only the second wired. An earlier but
        // unconnected input must not shadow the one that actually has pixels.
        let mut n = node(
            "blit",
            None,
            vec![image_input(None), image_input(Some(("src", 0)))],
            vec![img_out()],
        );
        n.inputs[1].name = "foreground".to_string();
        let nodes = graph(vec![node("src", None, vec![], vec![img_out()]), n]);
        assert_eq!(gizmo_backdrop_source(&nodes, "blit"), Some(("src".to_string(), 0)));
    }

    #[test]
    fn a_dangling_connection_resolves_to_nothing() {
        let nodes = graph(vec![crop_node("crop", Some(("missing", 0)))]);
        assert_eq!(gizmo_backdrop_source(&nodes, "crop"), None);
    }

    #[test]
    fn an_out_of_range_output_index_resolves_to_nothing() {
        let nodes = graph(vec![
            node("src", None, vec![], vec![img_out()]),
            crop_node("crop", Some(("src", 7))),
        ]);
        assert_eq!(gizmo_backdrop_source(&nodes, "crop"), None);
    }

    #[test]
    fn a_non_image_upstream_output_resolves_to_nothing() {
        let nodes = graph(vec![
            node(
                "num",
                None,
                vec![],
                vec![Output::new("value".to_string(), Value::Decimal(1.0), None)],
            ),
            crop_node("crop", Some(("num", 0))),
        ]);
        assert_eq!(gizmo_backdrop_source(&nodes, "crop"), None);
    }

    #[test]
    fn a_missing_node_resolves_to_nothing() {
        assert_eq!(gizmo_backdrop_source(&graph(vec![]), "nope"), None);
    }

    // --- editor resolution ---

    #[test]
    fn resolve_picks_gizmos_for_an_op_that_declares_them() {
        let nodes = graph(vec![crop_node("crop", None)]);
        match resolve_preview_editor(&nodes, Some("crop")) {
            Some(PreviewEditor::Gizmos { node_id, specs }) => {
                assert_eq!(node_id, "crop");
                assert_eq!(specs.len(), 1);
            }
            other => panic!("expected gizmos, got {}", label(&other)),
        }
    }

    #[test]
    fn curve_wins_over_gizmos_when_a_node_has_both() {
        // The curve overlay's empty-space catcher covers the whole panel and
        // would swallow gizmo clicks, so exactly one editor may be active.
        let mut n = crop_node("crop", None);
        n.inputs.push(curve_input("path", None));
        let nodes = graph(vec![n]);
        match resolve_preview_editor(&nodes, Some("crop")) {
            Some(PreviewEditor::Curve { input_index, .. }) => assert_eq!(input_index, 5),
            other => panic!("expected curve, got {}", label(&other)),
        }
    }

    #[test]
    fn a_connected_curve_input_does_not_claim_the_overlay() {
        // A driven curve can't be hand-edited, so the node's gizmos still win.
        let mut n = crop_node("crop", None);
        let mut curve = curve_input("path", None);
        curve.connection = Some(("elsewhere".to_string(), 0));
        n.inputs.push(curve);
        let nodes = graph(vec![n]);
        assert!(matches!(
            resolve_preview_editor(&nodes, Some("crop")),
            Some(PreviewEditor::Gizmos { .. })
        ));
    }

    #[test]
    fn a_tone_curve_input_is_never_a_spatial_editor() {
        // Tone curves map values, not space; they are edited in the settings
        // panel's embedded box instead.
        let n = node(
            "curves",
            None,
            vec![curve_input("master", Some(InputSettings::ToneCurve))],
            vec![img_out()],
        );
        assert!(resolve_preview_editor(&graph(vec![n]), Some("curves")).is_none());
    }

    #[test]
    fn an_op_without_gizmos_gets_no_editor() {
        let nodes = graph(vec![node(
            "resize",
            Some(Operation::OpImageTransformResize),
            vec![image_input(None)],
            vec![img_out()],
        )]);
        assert!(resolve_preview_editor(&nodes, Some("resize")).is_none());
    }

    #[test]
    fn an_unknown_node_type_gets_no_editor() {
        // `NodeType::Unknown` placeholders carry no operation, so they can never
        // reach the gizmo table.
        let nodes = graph(vec![node("mystery", None, vec![], vec![img_out()])]);
        assert!(resolve_preview_editor(&nodes, Some("mystery")).is_none());
    }

    #[test]
    fn no_selection_and_missing_nodes_get_no_editor() {
        let nodes = graph(vec![crop_node("crop", None)]);
        assert!(resolve_preview_editor(&nodes, None).is_none());
        assert!(resolve_preview_editor(&nodes, Some("gone")).is_none());
    }
}
