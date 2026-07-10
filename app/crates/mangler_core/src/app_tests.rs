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
            input_values: Vec::new(),
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

// ── saved-graph version compatibility: hold_saves / conflict handling ─────

/// Poll `rx` for up to `attempts * 100ms`, returning `true` as soon as a
/// message matching `predicate` is seen. Any non-matching messages received
/// along the way are dropped.
async fn wait_for_message<T>(
    rx: &mut mpsc::Receiver<T>,
    attempts: u32,
    mut predicate: impl FnMut(&T) -> bool,
) -> bool {
    for _ in 0..attempts {
        while let Ok(msg) = rx.try_recv() {
            if predicate(&msg) {
                return true;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

#[tokio::test]
async fn test_newer_version_file_holds_autosave_until_edit() {
    // A graph file stamped with a version newer than this build.
    let path = std::env::temp_dir().join(format!("mangler_hold_test_{}.mangle.json", get_id()));
    let save_data = crate::GraphSaveData {
        version: "999.0.0".to_string(),
        id: get_id(),
        name: "hold test graph".to_string(),
        nodes: std::collections::HashMap::new(),
    };
    std::fs::write(&path, serde_json::to_string(&save_data).unwrap()).unwrap();
    let original = std::fs::read_to_string(&path).unwrap();

    let (tx_change_graph, rx_change_graph) = mpsc::channel::<ChangeGraphMessage>(32);
    let (_tx_change_node, rx_change_node) = mpsc::channel::<ChangeNodeMessage>(32);
    let (tx_node_changed, _rx_node_changed) = mpsc::channel::<NodeChangedMessage>(256);
    let (tx_graph_changed, _rx_graph_changed) = mpsc::channel::<GraphChangedMessage>(256);

    let app = App::new(
        None,
        Some(path.clone()),
        rx_change_graph,
        rx_change_node,
        tx_node_changed,
        tx_graph_changed,
    )
    .expect("App::new should succeed even for a newer-version file");

    // Comfortably past a couple of debounce intervals — the held file must
    // not have been rewritten (and therefore downgraded) with no edits.
    tokio::time::sleep(Duration::from_millis(2500)).await;
    let unchanged = std::fs::read_to_string(&path).unwrap();
    assert_eq!(original, unchanged, "auto-save must stay held for a newer-version file with no edits");

    // An edit should release the hold and let the pending save go through,
    // restamping the file with this build's APP_VERSION.
    tx_change_graph
        .send(ChangeGraphMessage::AddNode {
            node_id: get_id(),
            node_type: AddNodeType::Operation(Operation::OpNumberInputDecimal),
            position: glam::Vec2::ZERO,
            is_enabled: true,
            custom_name: None,
            input_values: Vec::new(),
        })
        .await
        .unwrap();

    let mut saved_and_restamped = false;
    for _ in 0..50 {
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if contents != original {
                if let Ok(parsed) = serde_json::from_str::<crate::GraphSaveData>(&contents) {
                    if parsed.version == crate::APP_VERSION {
                        saved_and_restamped = true;
                        break;
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(saved_and_restamped, "an edit should release the hold and the next save should restamp APP_VERSION");

    app.thread_handle.abort();
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn test_external_overwrite_with_pending_edit_triggers_file_conflict() {
    let path = std::env::temp_dir().join(format!("mangler_conflict_test_{}.mangle.json", get_id()));
    let initial = crate::GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: get_id(),
        name: "conflict test".to_string(),
        nodes: std::collections::HashMap::new(),
    };
    std::fs::write(&path, serde_json::to_string(&initial).unwrap()).unwrap();

    let (tx_change_graph, rx_change_graph) = mpsc::channel::<ChangeGraphMessage>(32);
    let (_tx_change_node, rx_change_node) = mpsc::channel::<ChangeNodeMessage>(32);
    let (tx_node_changed, _rx_node_changed) = mpsc::channel::<NodeChangedMessage>(256);
    let (tx_graph_changed, mut rx_graph_changed) = mpsc::channel::<GraphChangedMessage>(256);

    let app = App::new(
        None,
        Some(path.clone()),
        rx_change_graph,
        rx_change_node,
        tx_node_changed,
        tx_graph_changed,
    )
    .expect("App::new should succeed");

    // A pending local edit that wants to auto-save on the next debounce tick.
    tx_change_graph
        .send(ChangeGraphMessage::AddNode {
            node_id: get_id(),
            node_type: AddNodeType::Operation(Operation::OpNumberInputDecimal),
            position: glam::Vec2::ZERO,
            is_enabled: true,
            custom_name: None,
            input_values: Vec::new(),
        })
        .await
        .unwrap();

    // Simulate another tab/machine overwriting the file first. Force the
    // mtime safely into the future so disk_is_newer() sees it as newer
    // regardless of filesystem mtime granularity or scheduling jitter.
    let external_content = serde_json::to_string(&crate::GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: get_id(),
        name: "written by someone else".to_string(),
        nodes: std::collections::HashMap::new(),
    })
    .unwrap();
    std::fs::write(&path, &external_content).unwrap();
    let future = std::time::SystemTime::now() + Duration::from_secs(5);
    filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(future)).unwrap();

    let saw_conflict = wait_for_message(&mut rx_graph_changed, 50, |msg| {
        matches!(msg, GraphChangedMessage::FileConflict { .. })
    })
    .await;
    assert!(saw_conflict, "external overwrite with a pending local edit should raise FileConflict");

    // The engine must not have clobbered the externally written content.
    let on_disk = std::fs::read_to_string(&path).unwrap();
    assert_eq!(on_disk, external_content, "auto-save must not overwrite a conflicting external edit");

    app.thread_handle.abort();
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn test_resolve_file_conflict_keep_ours_overwrites_disk() {
    let path = std::env::temp_dir().join(format!("mangler_keep_ours_{}.mangle.json", get_id()));
    let initial = crate::GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: get_id(),
        name: "keep ours test".to_string(),
        nodes: std::collections::HashMap::new(),
    };
    std::fs::write(&path, serde_json::to_string(&initial).unwrap()).unwrap();

    let (tx_change_graph, rx_change_graph) = mpsc::channel::<ChangeGraphMessage>(32);
    let (_tx_change_node, rx_change_node) = mpsc::channel::<ChangeNodeMessage>(32);
    let (tx_node_changed, _rx_node_changed) = mpsc::channel::<NodeChangedMessage>(256);
    let (tx_graph_changed, mut rx_graph_changed) = mpsc::channel::<GraphChangedMessage>(256);

    let app = App::new(
        None,
        Some(path.clone()),
        rx_change_graph,
        rx_change_node,
        tx_node_changed,
        tx_graph_changed,
    )
    .expect("App::new should succeed");

    // Our pending local edit: add a node with a known id. After a keep_ours
    // resolve, the overwritten file must contain this node — proof our
    // in-memory graph won, not the external content (which has no nodes).
    let our_node_id = get_id();
    tx_change_graph
        .send(ChangeGraphMessage::AddNode {
            node_id: our_node_id.clone(),
            node_type: AddNodeType::Operation(Operation::OpNumberInputDecimal),
            position: glam::Vec2::ZERO,
            is_enabled: true,
            custom_name: None,
            input_values: Vec::new(),
        })
        .await
        .unwrap();

    let external_content = serde_json::to_string(&crate::GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: get_id(),
        name: "theirs".to_string(),
        nodes: std::collections::HashMap::new(),
    })
    .unwrap();
    std::fs::write(&path, &external_content).unwrap();
    let future = std::time::SystemTime::now() + Duration::from_secs(5);
    filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(future)).unwrap();

    let saw_conflict = wait_for_message(&mut rx_graph_changed, 50, |msg| {
        matches!(msg, GraphChangedMessage::FileConflict { .. })
    })
    .await;
    assert!(saw_conflict, "precondition: conflict must be detected before resolving it");

    tx_change_graph
        .send(ChangeGraphMessage::ResolveFileConflict { keep_ours: true })
        .await
        .unwrap();

    let mut overwritten = false;
    for _ in 0..50 {
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Ok(parsed) = serde_json::from_str::<crate::GraphSaveData>(&contents) {
                if parsed.nodes.contains_key(&our_node_id) {
                    overwritten = true;
                    break;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(overwritten, "keep_ours=true should overwrite disk with the in-memory graph");

    app.thread_handle.abort();
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn test_resolve_file_conflict_keep_theirs_reloads_from_disk() {
    let path = std::env::temp_dir().join(format!("mangler_keep_theirs_{}.mangle.json", get_id()));
    let initial = crate::GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: get_id(),
        name: "keep theirs test".to_string(),
        nodes: std::collections::HashMap::new(),
    };
    std::fs::write(&path, serde_json::to_string(&initial).unwrap()).unwrap();

    let (tx_change_graph, rx_change_graph) = mpsc::channel::<ChangeGraphMessage>(32);
    let (_tx_change_node, rx_change_node) = mpsc::channel::<ChangeNodeMessage>(32);
    let (tx_node_changed, _rx_node_changed) = mpsc::channel::<NodeChangedMessage>(256);
    let (tx_graph_changed, mut rx_graph_changed) = mpsc::channel::<GraphChangedMessage>(256);

    let app = App::new(
        None,
        Some(path.clone()),
        rx_change_graph,
        rx_change_node,
        tx_node_changed,
        tx_graph_changed,
    )
    .expect("App::new should succeed");

    // A local edit that will be discarded.
    tx_change_graph
        .send(ChangeGraphMessage::AddNode {
            node_id: get_id(),
            node_type: AddNodeType::Operation(Operation::OpNumberInputDecimal),
            position: glam::Vec2::ZERO,
            is_enabled: true,
            custom_name: None,
            input_values: Vec::new(),
        })
        .await
        .unwrap();

    // Someone else writes a different graph containing one node — this is
    // what we expect to see after a keep_ours=false reload.
    let external_node_id = get_id();
    let external_node = crate::node::Node::new(
        external_node_id.clone(),
        AddNodeType::Operation(Operation::OpNumberInputInteger),
        glam::Vec2::new(42.0, 42.0),
    );
    let mut external_nodes = std::collections::HashMap::new();
    external_nodes.insert(external_node_id.clone(), external_node);
    let external_save = crate::GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: get_id(),
        name: "theirs".to_string(),
        nodes: external_nodes,
    };
    std::fs::write(&path, serde_json::to_string(&external_save).unwrap()).unwrap();
    let future = std::time::SystemTime::now() + Duration::from_secs(5);
    filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(future)).unwrap();

    let saw_conflict = wait_for_message(&mut rx_graph_changed, 50, |msg| {
        matches!(msg, GraphChangedMessage::FileConflict { .. })
    })
    .await;
    assert!(saw_conflict, "precondition: conflict must be detected before resolving it");

    tx_change_graph
        .send(ChangeGraphMessage::ResolveFileConflict { keep_ours: false })
        .await
        .unwrap();

    // Collect messages in arrival order until the reloaded external node
    // shows up, then check GraphCleared arrived strictly before it.
    let mut collected: Vec<GraphChangedMessage> = Vec::new();
    for _ in 0..50 {
        while let Ok(msg) = rx_graph_changed.try_recv() {
            collected.push(msg);
        }
        if collected.iter().any(|m| matches!(m, GraphChangedMessage::LoadedNode { node } if node.id == external_node_id)) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let cleared_idx = collected.iter().position(|m| matches!(m, GraphChangedMessage::GraphCleared));
    let loaded_idx = collected.iter().position(
        |m| matches!(m, GraphChangedMessage::LoadedNode { node } if node.id == external_node_id),
    );
    assert!(cleared_idx.is_some(), "keep_ours=false should send GraphCleared");
    assert!(loaded_idx.is_some(), "keep_ours=false should reload and replay the on-disk graph's nodes");
    assert!(
        cleared_idx.unwrap() < loaded_idx.unwrap(),
        "GraphCleared must arrive before the reloaded LoadedNode stream"
    );

    app.thread_handle.abort();
    let _ = std::fs::remove_file(&path);
}
