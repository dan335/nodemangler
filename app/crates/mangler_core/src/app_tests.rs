use std::time::Duration;
use tokio::sync::mpsc;

use crate::{
    app::App, get_id, operations::Operation, AddNodeType, ChangeGraphMessage,
    ChangeNodeMessage, GraphChangedMessage, NodeChangedMessage,
};

/// Regression test for the auto-save loop: a pending change must eventually
/// be written to disk (debounce never drops the final save), and once saved
/// with no further changes the engine must NOT keep rewriting the file every
/// tick (the original bug never reset the needs-to-save flag).
#[tokio::test]
async fn test_auto_save_is_debounced_and_final_save_not_lost() {
    let (tx_change_graph, rx_change_graph) = mpsc::channel::<ChangeGraphMessage>(32);
    let (_tx_change_node, rx_change_node) = mpsc::channel::<ChangeNodeMessage>(32);
    let (tx_node_changed, _rx_node_changed) = mpsc::channel::<NodeChangedMessage>(256);
    let (tx_graph_changed, _rx_graph_changed) = mpsc::channel::<GraphChangedMessage>(256);

    let app = App::new(
        None,
        None,
        rx_change_graph,
        rx_change_node,
        tx_node_changed,
        tx_graph_changed,
    )
    .expect("App::new should succeed");

    let path = std::env::temp_dir().join(format!("mangler_autosave_test_{}.mangle.json", get_id()));

    tx_change_graph
        .send(ChangeGraphMessage::SetSavePath(path.clone()))
        .await
        .unwrap();
    tx_change_graph
        .send(ChangeGraphMessage::AddNode {
            node_id: get_id(),
            node_type: AddNodeType::Operation(Operation::OpNumberInputDecimal),
            position: glam::Vec2::ZERO,
            is_enabled: true,
            custom_name: None,
        })
        .await
        .unwrap();

    // The pending change must be written within a few debounce intervals.
    let mut saved = false;
    for _ in 0..50 {
        if path.exists() {
            saved = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(saved, "pending auto-save was never written to disk");

    // With no further changes the file must not be rewritten every tick.
    let mtime_after_save = std::fs::metadata(&path).unwrap().modified().unwrap();
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let mtime_later = std::fs::metadata(&path).unwrap().modified().unwrap();
    assert_eq!(
        mtime_after_save, mtime_later,
        "graph file was rewritten while no changes were pending"
    );

    app.thread_handle.abort();
    let _ = std::fs::remove_file(&path);
}
