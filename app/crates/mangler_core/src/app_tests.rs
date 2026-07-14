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

    let path = std::env::temp_dir().join(format!("mangler_autosave_test_{}.mangler.json", get_id()));

    tx_change_graph
        .send(ChangeGraphMessage::SetSavePath(path.clone()))
        .await
        .unwrap();
    let node_id = get_id();
    tx_change_graph
        .send(ChangeGraphMessage::AddNode {
            node_id: node_id.clone(),
            node_type: AddNodeType::Operation(Operation::OpNumberInputDecimal),
            position: glam::Vec2::ZERO,
            is_enabled: true,
            custom_name: None,
            input_values: Vec::new(),
        })
        .await
        .unwrap();

    // The pending change must be written within a few debounce intervals.
    // SetSavePath itself saves synchronously now, so merely existing isn't
    // enough — wait until the debounced save containing the node lands.
    let mut saved = false;
    for _ in 0..50 {
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Ok(parsed) = serde_json::from_str::<crate::GraphSaveData>(&contents) {
                if parsed.nodes.contains_key(&node_id) {
                    saved = true;
                    break;
                }
            }
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

// ── batch run: RunBatch / CancelBatch driver ───────────────────────────────

/// Creates (or clears) a fresh temp dir for a batch test.
fn batch_temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("nodemangler_test_app_batch_{}", name));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes a tiny (1x1) real grayscale PNG so the from-folder node can decode it.
fn write_tiny_png(path: &std::path::Path, gray: u8) {
    image::GrayImage::from_pixel(1, 1, image::Luma([gray]))
        .save(path)
        .unwrap();
}

/// Spawns an engine and returns it plus all four channel ends used by the
/// batch tests.
fn batch_test_app() -> (
    App,
    mpsc::Sender<ChangeGraphMessage>,
    mpsc::Receiver<NodeChangedMessage>,
    mpsc::Receiver<GraphChangedMessage>,
) {
    let (tx_change_graph, rx_change_graph) = mpsc::channel::<ChangeGraphMessage>(32);
    let (_tx_change_node, rx_change_node) = mpsc::channel::<ChangeNodeMessage>(32);
    // Generous buffers: a batch emits OutputChanged/Busy/InputChanged floods,
    // and the engine's try_send drops messages once a buffer fills. The
    // receiver ends are handed back so they outlive the whole test. (The
    // unused change-node sender is simply dropped; the engine's try_recv
    // drain tolerates a disconnected channel.)
    let (tx_node_changed, rx_node_changed) = mpsc::channel::<NodeChangedMessage>(4096);
    let (tx_graph_changed, rx_graph_changed) = mpsc::channel::<GraphChangedMessage>(1024);
    let app = App::new(None, None, rx_change_graph, rx_change_node, tx_node_changed, tx_graph_changed)
        .expect("App::new should succeed");
    (app, tx_change_graph, rx_node_changed, rx_graph_changed)
}

/// A full batch over 3 images must run the graph once per file (3 in-order
/// BatchProgress messages), force the `to file` node to write one distinctly
/// named file per source image (literal stem + `_` + item stem — the phase-2
/// naming rule), restore the from-folder node's `index` input afterwards, and
/// finish with `cancelled: false`.
#[tokio::test]
async fn test_batch_run_processes_all_files_and_restores_index() {
    use crate::operations::images::inputs::from_folder;

    let img_dir = batch_temp_dir("run_images");
    // Case-insensitive sort order: apple, banana, cherry.
    write_tiny_png(&img_dir.join("banana.png"), 100);
    write_tiny_png(&img_dir.join("apple.png"), 50);
    write_tiny_png(&img_dir.join("cherry.png"), 200);
    let out_dir = batch_temp_dir("run_outputs");

    let (app, tx_change_graph, mut rx_node_changed, mut rx_graph_changed) = batch_test_app();

    // from-folder (index deliberately NOT the default, to prove the restore)
    // wired into a `to file` node with an absolute output folder, a literal
    // file name, and png format. No SetSavePath: absolute paths everywhere.
    let source_id = get_id();
    tx_change_graph
        .send(ChangeGraphMessage::AddNode {
            node_id: source_id.clone(),
            node_type: AddNodeType::Operation(Operation::OpImageInputFromFolder),
            position: glam::Vec2::ZERO,
            is_enabled: true,
            custom_name: None,
            input_values: vec![
                (from_folder::FOLDER, crate::value::Value::Path(img_dir.clone())),
                (from_folder::INDEX, crate::value::Value::Integer(2)),
            ],
        })
        .await
        .unwrap();
    let sink_id = get_id();
    tx_change_graph
        .send(ChangeGraphMessage::AddNode {
            node_id: sink_id.clone(),
            node_type: AddNodeType::Operation(Operation::OpImageOutputFile),
            position: glam::Vec2::ZERO,
            is_enabled: true,
            custom_name: None,
            input_values: vec![
                (1, crate::value::Value::Path(out_dir.clone())),          // folder
                (2, crate::value::Value::Text("out".to_string())),        // file name
                (3, crate::value::Value::ImageType(image::ImageFormat::Png)), // format
            ],
        })
        .await
        .unwrap();
    tx_change_graph
        .send(ChangeGraphMessage::AddConnection {
            input_node_id: sink_id.clone(),
            input_connection_index: 0,
            output_node_id: source_id.clone(),
            output_connection_index: 0,
        })
        .await
        .unwrap();
    tx_change_graph
        .send(ChangeGraphMessage::RunBatch { node_id: source_id.clone() })
        .await
        .unwrap();

    // Collect graph-changed messages until BatchFinished (or timeout).
    let mut progress: Vec<(usize, usize)> = Vec::new();
    let mut finished: Option<(usize, usize, bool)> = None;
    for _ in 0..100 {
        while let Ok(msg) = rx_graph_changed.try_recv() {
            match msg {
                GraphChangedMessage::BatchProgress { completed, total, .. } => progress.push((completed, total)),
                GraphChangedMessage::BatchFinished { completed, total, cancelled, .. } => {
                    finished = Some((completed, total, cancelled))
                }
                _ => {}
            }
        }
        if finished.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    assert_eq!(finished, Some((3, 3, false)), "batch over 3 files must finish uncancelled");
    assert_eq!(progress, vec![(1, 3), (2, 3), (3, 3)], "one in-order progress message per file");

    // Phase-2 naming: unwired literal name "out" + per-item stem.
    for stem in ["apple", "banana", "cherry"] {
        let expected = out_dir.join(format!("out_{stem}.png"));
        assert!(expected.exists(), "batch should have written {}", expected.display());
    }
    assert_eq!(
        std::fs::read_dir(&out_dir).unwrap().count(),
        3,
        "exactly one output per source image (nothing overwritten or extra)"
    );

    // The last InputChanged echoed for the index input must be the restored
    // pre-batch value (2), after the stepped 0, 1, 2.
    let mut last_index_value = None;
    while let Ok(msg) = rx_node_changed.try_recv() {
        if let NodeChangedMessage::InputChanged { node_id, input_index, value } = msg {
            if node_id == source_id && input_index == from_folder::INDEX {
                last_index_value = Some(value);
            }
        }
    }
    assert!(
        matches!(last_index_value, Some(crate::value::Value::Integer(2))),
        "the index input must be restored to its pre-batch value, got {:?}",
        last_index_value
    );

    app.thread_handle.abort();
    let _ = std::fs::remove_dir_all(&img_dir);
    let _ = std::fs::remove_dir_all(&out_dir);
}

/// A batch whose from-folder node uses a *relative* folder path must resolve
/// it against the graph's save directory — in both the driver's snapshot
/// (`start_batch`) and the node's own `run()` (via its RunContext). The
/// output node's `file name` is deliberately left for `Graph::add_node` to
/// pre-fill with the usual `{graph name}_{N}` stem, so this also covers the
/// realistic GUI flow: the pre-filled literal gets each item's stem appended
/// (`graph_1_first.png`, ...). (The unwired-*empty* name → bare item stem
/// rule is unit-tested in `outputs/mod_tests.rs`.)
#[tokio::test]
async fn test_batch_run_resolves_relative_folder_against_graph_dir() {
    use crate::operations::images::inputs::from_folder;

    // Layout: {graph_dir}/graph.mangler.json + {graph_dir}/images/*.png,
    // with the node's folder input set to just "images".
    let graph_dir = batch_temp_dir("relative_graph");
    let img_dir = graph_dir.join("images");
    std::fs::create_dir_all(&img_dir).unwrap();
    write_tiny_png(&img_dir.join("first.png"), 10);
    write_tiny_png(&img_dir.join("second.png"), 20);
    let out_dir = batch_temp_dir("relative_outputs");

    let (app, tx_change_graph, _rx_node_changed, mut rx_graph_changed) = batch_test_app();

    let graph_path = graph_dir.join("graph.mangler.json");
    tx_change_graph
        .send(ChangeGraphMessage::SetSavePath(graph_path))
        .await
        .unwrap();

    let source_id = get_id();
    tx_change_graph
        .send(ChangeGraphMessage::AddNode {
            node_id: source_id.clone(),
            node_type: AddNodeType::Operation(Operation::OpImageInputFromFolder),
            position: glam::Vec2::ZERO,
            is_enabled: true,
            custom_name: None,
            input_values: vec![(
                from_folder::FOLDER,
                crate::value::Value::Path(std::path::PathBuf::from("images")),
            )],
        })
        .await
        .unwrap();
    let sink_id = get_id();
    tx_change_graph
        .send(ChangeGraphMessage::AddNode {
            node_id: sink_id.clone(),
            node_type: AddNodeType::Operation(Operation::OpImageOutputFile),
            position: glam::Vec2::ZERO,
            is_enabled: true,
            custom_name: None,
            input_values: vec![
                (1, crate::value::Value::Path(out_dir.clone())), // folder (absolute)
                // file name deliberately not set: add_node pre-fills it with
                // the unique `{graph name}_{N}` stem, like a GUI-created node
                (3, crate::value::Value::ImageType(image::ImageFormat::Png)),
            ],
        })
        .await
        .unwrap();
    tx_change_graph
        .send(ChangeGraphMessage::AddConnection {
            input_node_id: sink_id.clone(),
            input_connection_index: 0,
            output_node_id: source_id.clone(),
            output_connection_index: 0,
        })
        .await
        .unwrap();
    tx_change_graph
        .send(ChangeGraphMessage::RunBatch { node_id: source_id.clone() })
        .await
        .unwrap();

    let mut finished: Option<(usize, usize, bool)> = None;
    for _ in 0..100 {
        while let Ok(msg) = rx_graph_changed.try_recv() {
            if let GraphChangedMessage::BatchFinished { completed, total, cancelled, .. } = msg {
                finished = Some((completed, total, cancelled));
            }
        }
        if finished.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    assert_eq!(
        finished,
        Some((2, 2, false)),
        "a relative folder must resolve against the graph's directory"
    );
    // Two files, each carrying the pre-filled literal stem plus its item stem.
    let written: Vec<String> = std::fs::read_dir(&out_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(written.len(), 2, "one output per source image, got {written:?}");
    for stem in ["first", "second"] {
        assert!(
            written.iter().any(|name| name.ends_with(&format!("_{stem}.png"))),
            "expected a file named `<literal>_{stem}.png` among {written:?}"
        );
    }

    app.thread_handle.abort();
    let _ = std::fs::remove_dir_all(&graph_dir);
    let _ = std::fs::remove_dir_all(&out_dir);
}

/// CancelBatch must stop the run between iterations: fewer files complete
/// than exist, and BatchFinished reports `cancelled: true`.
#[tokio::test]
async fn test_batch_run_cancel_stops_early() {
    use crate::operations::images::inputs::from_folder;

    let img_dir = batch_temp_dir("cancel_images");
    // Enough files that the cancel (sent right after the first progress
    // message) always lands well before the batch would naturally finish:
    // iterations are one per ~16ms engine tick.
    const TOTAL: usize = 30;
    for i in 0..TOTAL {
        write_tiny_png(&img_dir.join(format!("img_{i:02}.png")), i as u8);
    }

    let (app, tx_change_graph, _rx_node_changed, mut rx_graph_changed) = batch_test_app();

    let source_id = get_id();
    tx_change_graph
        .send(ChangeGraphMessage::AddNode {
            node_id: source_id.clone(),
            node_type: AddNodeType::Operation(Operation::OpImageInputFromFolder),
            position: glam::Vec2::ZERO,
            is_enabled: true,
            custom_name: None,
            input_values: vec![(from_folder::FOLDER, crate::value::Value::Path(img_dir.clone()))],
        })
        .await
        .unwrap();
    tx_change_graph
        .send(ChangeGraphMessage::RunBatch { node_id: source_id.clone() })
        .await
        .unwrap();

    // Wait for the first progress message, then cancel. Short poll interval
    // so the cancel goes out while plenty of files remain.
    let mut saw_progress = false;
    for _ in 0..500 {
        while let Ok(msg) = rx_graph_changed.try_recv() {
            if matches!(msg, GraphChangedMessage::BatchProgress { .. }) {
                saw_progress = true;
            }
        }
        if saw_progress {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(saw_progress, "precondition: the batch must have started");
    tx_change_graph.send(ChangeGraphMessage::CancelBatch).await.unwrap();

    let mut finished: Option<(usize, usize, bool)> = None;
    for _ in 0..100 {
        while let Ok(msg) = rx_graph_changed.try_recv() {
            if let GraphChangedMessage::BatchFinished { completed, total, cancelled, .. } = msg {
                finished = Some((completed, total, cancelled));
            }
        }
        if finished.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let (completed, total, cancelled) = finished.expect("cancel must produce BatchFinished");
    assert!(cancelled, "BatchFinished must report cancelled: true");
    assert_eq!(total, TOTAL);
    assert!(
        completed < TOTAL,
        "cancel should stop the batch early (completed {completed} of {total})"
    );

    app.thread_handle.abort();
    let _ = std::fs::remove_dir_all(&img_dir);
}

/// A RunBatch that cannot start — the target isn't a from-folder node, or the
/// folder has no images — must reply immediately with
/// `BatchFinished { total: 0, cancelled: true }` instead of silently doing
/// nothing.
#[tokio::test]
async fn test_batch_run_invalid_start_reports_finished() {
    use crate::operations::images::inputs::from_folder;

    let empty_dir = batch_temp_dir("invalid_empty");
    let (app, tx_change_graph, _rx_node_changed, mut rx_graph_changed) = batch_test_app();

    // Case 1: not a from-folder node.
    let decimal_id = get_id();
    tx_change_graph
        .send(ChangeGraphMessage::AddNode {
            node_id: decimal_id.clone(),
            node_type: AddNodeType::Operation(Operation::OpNumberInputDecimal),
            position: glam::Vec2::ZERO,
            is_enabled: true,
            custom_name: None,
            input_values: Vec::new(),
        })
        .await
        .unwrap();
    tx_change_graph
        .send(ChangeGraphMessage::RunBatch { node_id: decimal_id.clone() })
        .await
        .unwrap();
    let refused = wait_for_message(&mut rx_graph_changed, 50, |msg| {
        matches!(
            msg,
            GraphChangedMessage::BatchFinished { node_id, completed: 0, total: 0, cancelled: true }
                if *node_id == decimal_id
        )
    })
    .await;
    assert!(refused, "RunBatch on a non-from-folder node must report a failed start");

    // Case 2: a from-folder node whose folder contains no images.
    let source_id = get_id();
    tx_change_graph
        .send(ChangeGraphMessage::AddNode {
            node_id: source_id.clone(),
            node_type: AddNodeType::Operation(Operation::OpImageInputFromFolder),
            position: glam::Vec2::ZERO,
            is_enabled: true,
            custom_name: None,
            input_values: vec![(from_folder::FOLDER, crate::value::Value::Path(empty_dir.clone()))],
        })
        .await
        .unwrap();
    tx_change_graph
        .send(ChangeGraphMessage::RunBatch { node_id: source_id.clone() })
        .await
        .unwrap();
    let refused = wait_for_message(&mut rx_graph_changed, 50, |msg| {
        matches!(
            msg,
            GraphChangedMessage::BatchFinished { node_id, completed: 0, total: 0, cancelled: true }
                if *node_id == source_id
        )
    })
    .await;
    assert!(refused, "RunBatch over an empty folder must report a failed start");

    app.thread_handle.abort();
    let _ = std::fs::remove_dir_all(&empty_dir);
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
    let path = std::env::temp_dir().join(format!("mangler_hold_test_{}.mangler.json", get_id()));
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

/// Regression test for graphs saved in cloud-synced folders (Google Drive,
/// OneDrive, Dropbox): the sync client re-stamps the file's mtime after
/// uploading our own auto-save, without changing its content. That touch must
/// NOT raise FileConflict — the next pending edit must simply auto-save.
/// (Before content-aware `disk_conflicts`, every edit in a synced folder
/// popped the "file changed on disk" dialog.)
#[tokio::test]
async fn test_sync_client_mtime_touch_does_not_trigger_file_conflict() {
    let path = std::env::temp_dir().join(format!("mangler_sync_touch_{}.mangler.json", get_id()));
    let initial = crate::GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: get_id(),
        name: "sync touch test".to_string(),
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

    // Simulate the sync client touching the file BEFORE any edit: identical
    // bytes, future mtime.
    let future = std::time::SystemTime::now() + Duration::from_secs(5);
    filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(future)).unwrap();

    // A local edit that wants to auto-save on the next debounce tick.
    let node_id = get_id();
    tx_change_graph
        .send(ChangeGraphMessage::AddNode {
            node_id: node_id.clone(),
            node_type: AddNodeType::Operation(Operation::OpNumberInputDecimal),
            position: glam::Vec2::ZERO,
            is_enabled: true,
            custom_name: None,
            input_values: Vec::new(),
        })
        .await
        .unwrap();

    // The edit must land on disk with NO FileConflict along the way.
    let mut saved = false;
    let mut saw_conflict = false;
    for _ in 0..50 {
        while let Ok(msg) = rx_graph_changed.try_recv() {
            if matches!(msg, GraphChangedMessage::FileConflict { .. }) {
                saw_conflict = true;
            }
        }
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Ok(parsed) = serde_json::from_str::<crate::GraphSaveData>(&contents) {
                if parsed.nodes.contains_key(&node_id) {
                    saved = true;
                    break;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(!saw_conflict, "a same-content mtime touch must not raise FileConflict");
    assert!(saved, "the pending edit must auto-save straight through the touch");

    app.thread_handle.abort();
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn test_external_overwrite_with_pending_edit_triggers_file_conflict() {
    let path = std::env::temp_dir().join(format!("mangler_conflict_test_{}.mangler.json", get_id()));
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
    let path = std::env::temp_dir().join(format!("mangler_keep_ours_{}.mangler.json", get_id()));
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

// ── unsaved-until-saved lifecycle: SetSavePath saves synchronously ─────────

/// A pathless engine (brand-new unsaved tab) that receives a SetSavePath must
/// write the file immediately — not on the ~1s debounce — and ack with
/// SavedTo, because the GUI's "save then close this tab" flow aborts the
/// engine task right after the ack and a deferred write would be lost.
#[tokio::test]
async fn test_set_save_path_saves_immediately_and_acks() {
    let (tx_change_graph, rx_change_graph) = mpsc::channel::<ChangeGraphMessage>(32);
    let (_tx_change_node, rx_change_node) = mpsc::channel::<ChangeNodeMessage>(32);
    let (tx_node_changed, _rx_node_changed) = mpsc::channel::<NodeChangedMessage>(256);
    let (tx_graph_changed, mut rx_graph_changed) = mpsc::channel::<GraphChangedMessage>(256);

    let app = App::new(
        None,
        None,
        rx_change_graph,
        rx_change_node,
        tx_node_changed,
        tx_graph_changed,
    )
    .expect("App::new should succeed");

    // Content first, path second — the unsaved-tab order.
    let node_id = get_id();
    tx_change_graph
        .send(ChangeGraphMessage::AddNode {
            node_id: node_id.clone(),
            node_type: AddNodeType::Operation(Operation::OpNumberInputDecimal),
            position: glam::Vec2::ZERO,
            is_enabled: true,
            custom_name: None,
            input_values: Vec::new(),
        })
        .await
        .unwrap();

    let path = std::env::temp_dir().join(format!("mangler_first_save_{}.mangler.json", get_id()));
    tx_change_graph
        .send(ChangeGraphMessage::SetSavePath(path.clone()))
        .await
        .unwrap();

    let acked = wait_for_message(&mut rx_graph_changed, 50, |msg| {
        matches!(msg, GraphChangedMessage::SavedTo { path: p } if *p == path)
    })
    .await;
    assert!(acked, "SetSavePath should ack with SavedTo for the chosen path");

    let contents = std::fs::read_to_string(&path).expect("file must exist once SavedTo arrived");
    let parsed: crate::GraphSaveData = serde_json::from_str(&contents).unwrap();
    assert!(parsed.nodes.contains_key(&node_id), "the saved file must contain the pre-save edit");

    // The immediate save must reset the debounce: with no further changes the
    // file must not be rewritten by the auto-save loop.
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

/// A pathless engine with pending edits must idle cleanly: no SaveError, no
/// FileConflict — the auto-save cycle is a silent no-op until the graph gets
/// a save path (new tabs are in-memory until the user saves them).
#[tokio::test]
async fn test_pathless_graph_never_writes_or_errors() {
    let (tx_change_graph, rx_change_graph) = mpsc::channel::<ChangeGraphMessage>(32);
    let (_tx_change_node, rx_change_node) = mpsc::channel::<ChangeNodeMessage>(32);
    let (tx_node_changed, _rx_node_changed) = mpsc::channel::<NodeChangedMessage>(256);
    let (tx_graph_changed, mut rx_graph_changed) = mpsc::channel::<GraphChangedMessage>(256);

    let app = App::new(
        None,
        None,
        rx_change_graph,
        rx_change_node,
        tx_node_changed,
        tx_graph_changed,
    )
    .expect("App::new should succeed");

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

    // Comfortably past several debounce intervals.
    tokio::time::sleep(Duration::from_millis(2500)).await;
    let mut errored = false;
    while let Ok(msg) = rx_graph_changed.try_recv() {
        if matches!(
            msg,
            GraphChangedMessage::SaveError { .. } | GraphChangedMessage::FileConflict { .. }
        ) {
            errored = true;
        }
    }
    assert!(!errored, "a pathless graph must not raise SaveError or FileConflict");

    app.thread_handle.abort();
}

/// SetSavePath must be refused while a file conflict is unresolved — the same
/// guard RenameFile has. Re-targeting the save path mid-conflict would muddy
/// the resolution (which file are we overwriting or reloading?).
#[tokio::test]
async fn test_set_save_path_refused_while_conflict_pending() {
    let path = std::env::temp_dir().join(format!("mangler_conflict_sp_{}.mangler.json", get_id()));
    let initial = crate::GraphSaveData {
        version: crate::APP_VERSION.to_string(),
        id: get_id(),
        name: "conflict setsavepath test".to_string(),
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

    // Pending local edit + external overwrite with a future mtime → conflict.
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
    assert!(saw_conflict, "precondition: conflict must be detected first");

    let other = std::env::temp_dir().join(format!("mangler_conflict_sp_other_{}.mangler.json", get_id()));
    tx_change_graph
        .send(ChangeGraphMessage::SetSavePath(other.clone()))
        .await
        .unwrap();

    let refused = wait_for_message(&mut rx_graph_changed, 50, |msg| {
        matches!(
            msg,
            GraphChangedMessage::SaveError { message, .. } if message == "resolve the file conflict first"
        )
    })
    .await;
    assert!(refused, "SetSavePath during an unresolved conflict must reply SaveError");
    assert!(!other.exists(), "the refused SetSavePath must not have written the new path");

    app.thread_handle.abort();
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&other);
}

#[tokio::test]
async fn test_resolve_file_conflict_keep_theirs_reloads_from_disk() {
    let path = std::env::temp_dir().join(format!("mangler_keep_theirs_{}.mangler.json", get_id()));
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
