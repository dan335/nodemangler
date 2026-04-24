use super::*;
use crate::float_image::FloatImage;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;

/// Small helper: build an Arc<FloatImage> of the given size filled with zeros.
fn tiny_image(w: u32, h: u32) -> Arc<FloatImage> {
    let n = (w * h * 4) as usize;
    Arc::new(FloatImage::from_raw(w, h, 4, vec![0.0; n]).unwrap())
}

/// Drain up to `n` messages with a bounded timeout. Returns whatever landed.
async fn drain(
    rx: &mut mpsc::Receiver<NodeChangedMessage>,
    n: usize,
    ms: u64,
) -> Vec<NodeChangedMessage> {
    let mut out = Vec::with_capacity(n);
    let deadline = Duration::from_millis(ms);
    for _ in 0..n {
        match timeout(deadline, rx.recv()).await {
            Ok(Some(m)) => out.push(m),
            _ => break,
        }
    }
    out
}

#[tokio::test]
async fn test_request_produces_thumbnail_ready() {
    let (tx, mut rx) = mpsc::channel::<NodeChangedMessage>(32);
    let svc = ThumbnailService::try_spawn(tx).expect("tokio::test provides a runtime");

    svc.request(
        "nodeA".to_string(),
        0,
        "change-1".to_string(),
        tiny_image(32, 32),
    );

    // Should land within a healthy budget even on slow CI.
    let msgs = drain(&mut rx, 1, 2000).await;
    assert_eq!(msgs.len(), 1, "expected one ThumbnailReady");
    match &msgs[0] {
        NodeChangedMessage::ThumbnailReady {
            node_id,
            output_index,
            change_id,
            thumbnail,
        } => {
            assert_eq!(node_id, "nodeA");
            assert_eq!(*output_index, 0);
            assert_eq!(change_id, "change-1");
            match thumbnail {
                Thumbnail::Image(img) => {
                    assert!(img.width() > 0 && img.height() > 0);
                }
                _ => panic!("expected Thumbnail::Image, got {:?}", thumbnail),
            }
        }
        other => panic!("unexpected message {:?}", other),
    }
}

#[tokio::test]
async fn test_superseded_request_replaces_earlier() {
    let (tx, mut rx) = mpsc::channel::<NodeChangedMessage>(32);
    let svc = ThumbnailService::try_spawn(tx).expect("tokio::test provides a runtime");

    // Enqueue twice against the same (node,output). The second supersedes
    // the first. Under coalescing, only one ThumbnailReady should arrive —
    // and it should carry change_id "v2".
    svc.request("n".into(), 0, "v1".into(), tiny_image(64, 64));
    svc.request("n".into(), 0, "v2".into(), tiny_image(64, 64));

    let msgs = drain(&mut rx, 2, 500).await;
    // Either:
    //   - exactly one message with change_id == "v2" (clean supersede), or
    //   - two messages where the v1 one was fast enough to complete before
    //     v2 superseded. We assert the last one is v2.
    assert!(!msgs.is_empty(), "expected at least one ThumbnailReady");
    match msgs.last().unwrap() {
        NodeChangedMessage::ThumbnailReady { change_id, .. } => {
            assert_eq!(change_id, "v2", "last thumbnail must be the newest request");
        }
        other => panic!("unexpected message {:?}", other),
    }
    // And no thumbnail with change_id != v1/v2 should appear.
    for m in &msgs {
        if let NodeChangedMessage::ThumbnailReady { change_id, .. } = m {
            assert!(
                change_id == "v1" || change_id == "v2",
                "unexpected change_id {}",
                change_id
            );
        }
    }
}

#[tokio::test]
async fn test_forget_node_drops_future_deliveries() {
    let (tx, mut rx) = mpsc::channel::<NodeChangedMessage>(32);
    let svc = ThumbnailService::try_spawn(tx).expect("tokio::test provides a runtime");

    svc.request("doomed".into(), 0, "c".into(), tiny_image(256, 256));
    // Immediately forget before the worker gets a chance to compute+send.
    svc.forget_node("doomed");

    // Give the worker time; nothing for "doomed" should land.
    let msgs = drain(&mut rx, 1, 200).await;
    for m in &msgs {
        if let NodeChangedMessage::ThumbnailReady { node_id, .. } = m {
            assert_ne!(
                node_id, "doomed",
                "forget_node should have dropped this request"
            );
        }
    }
}

#[tokio::test]
async fn test_different_keys_coexist() {
    let (tx, mut rx) = mpsc::channel::<NodeChangedMessage>(32);
    let svc = ThumbnailService::try_spawn(tx).expect("tokio::test provides a runtime");

    svc.request("a".into(), 0, "ca".into(), tiny_image(32, 32));
    svc.request("b".into(), 0, "cb".into(), tiny_image(32, 32));
    svc.request("a".into(), 1, "ca2".into(), tiny_image(32, 32));

    let msgs = drain(&mut rx, 3, 2000).await;
    assert_eq!(msgs.len(), 3, "three distinct keys should each produce a thumbnail");
    let mut seen: Vec<(String, usize)> = msgs
        .iter()
        .filter_map(|m| match m {
            NodeChangedMessage::ThumbnailReady {
                node_id,
                output_index,
                ..
            } => Some((node_id.clone(), *output_index)),
            _ => None,
        })
        .collect();
    seen.sort();
    assert_eq!(
        seen,
        vec![
            ("a".to_string(), 0),
            ("a".to_string(), 1),
            ("b".to_string(), 0),
        ]
    );
}
