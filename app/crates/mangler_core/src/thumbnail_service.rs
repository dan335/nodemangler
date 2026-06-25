//! Async thumbnail service.
//!
//! Image thumbnails (`FloatImage::resize_fit(...).to_rgba8()`) cost 15-50ms
//! on large frames — computing them inline inside the engine's graph-run loop
//! pushes that straight onto the decode pipeline's critical path and makes
//! scrubbing feel stuttery. This service runs the CPU work on a dedicated
//! tokio task + `spawn_blocking` pool so the engine never blocks on it.
//!
//! Correctness properties:
//!
//! - **Coalesced.** Producer `request` bumps a per-key sequence number.
//!   Requests carry that sequence; older requests become stale as soon as a
//!   newer one is issued.
//! - **Stale-reject on both sides.** The worker checks the latest-seq both
//!   before and after the blocking compute, because new requests can arrive
//!   during the 15-50ms compute window. The UI does a final check against the
//!   current output value's `change_id` (see `program.rs`).
//! - **Fire-and-forget on overflow.** If the internal mpsc is full the
//!   request is dropped; by the time it would have been processed, a newer
//!   one has superseded it anyway. This is the correct coalescing behaviour
//!   under a scrub storm.
//!
//! Only `Value::Image` goes through this service. Scalar/enum thumbnails
//! (`Thumbnail::Text`) are cheap enough to compute inline on the engine
//! thread and flow through `NodeChangedMessage::OutputChanged.thumbnail`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::mpsc::{self, Sender};
use tokio::task::JoinHandle;

use crate::float_image::FloatImage;
use crate::thumbnail::Thumbnail;
use crate::value::THUMBNAIL_SIZE;
use crate::NodeChangedMessage;

#[cfg(test)]
#[path = "thumbnail_service_tests.rs"]
mod tests;

/// Capacity of the internal request mpsc. Under scrub storm, excess requests
/// are dropped via `try_send` — correct coalescing, see module doc.
const REQUEST_QUEUE_CAPACITY: usize = 256;

/// Identifier of a node-output slot whose thumbnail is being tracked.
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
struct ThumbnailKey {
    node_id: String,
    output_index: usize,
}

/// What a thumbnail request is built from.
enum ThumbnailSource {
    /// Image outputs — the pixel buffer is already in-memory; just resize.
    Image(Arc<FloatImage>),
    /// Video-handle outputs — the worker must decode frame 0 via
    /// [`crate::video::VideoDecoderCache`] before resizing.
    ///
    /// `path` is read only by the `video`-feature-enabled decode arm in
    /// `decode_source`; the no-video build path ignores it.
    #[cfg_attr(not(feature = "video"), allow(dead_code))]
    Video { path: PathBuf },
}

/// An enqueued thumbnail job.
struct ThumbnailRequest {
    key: ThumbnailKey,
    /// Monotonic sequence for this key. Supersedes older requests for the
    /// same `(node_id, output_index)` via `latest_seq` comparison.
    seq: u64,
    /// Echoed back to the UI so it can stale-reject. For Image values this
    /// is the `change_id` field; for Video values it's the handle's path
    /// (the UI's stale-check has a matching arm).
    change_id: String,
    source: ThumbnailSource,
}

/// Handle on the running thumbnail worker. Owned by `Graph`; clone-able so
/// subgraph nodes can share the parent graph's worker.
///
/// Dropping all clones terminates the worker: the mpsc sender inside the
/// handle gets dropped, the worker receives `None` on `recv()` and exits.
#[derive(Debug)]
pub struct ThumbnailService {
    tx_request: Sender<ThumbnailRequest>,
    /// Latest sequence number issued per key. Producer bumps on `request`;
    /// worker compares against request's `seq` to drop stale work.
    latest_seq: Arc<DashMap<ThumbnailKey, u64>>,
    /// Monotonic counter shared across all keys. Cheaper than per-key
    /// counters and all we need is "newer than the last request for this
    /// key".
    seq_counter: Arc<AtomicU64>,
    _join: Arc<JoinHandle<()>>,
}

impl ThumbnailService {
    /// Spawn a new worker bound to `tx_node_changed`. Thumbnails are
    /// delivered as `NodeChangedMessage::ThumbnailReady`.
    ///
    /// Returns `None` if called outside a tokio runtime — `Graph::new` is
    /// callable from synchronous contexts (construction + tests) where
    /// spawning a task would panic. Callers treat `None` as "no async
    /// service available" and fall back to inline thumbnailing.
    pub fn try_spawn(tx_node_changed: Sender<NodeChangedMessage>) -> Option<Self> {
        let handle = tokio::runtime::Handle::try_current().ok()?;

        let (tx_request, rx_request) = mpsc::channel::<ThumbnailRequest>(REQUEST_QUEUE_CAPACITY);
        let latest_seq: Arc<DashMap<ThumbnailKey, u64>> = Arc::new(DashMap::new());
        let seq_counter = Arc::new(AtomicU64::new(0));

        let latest_seq_worker = Arc::clone(&latest_seq);
        let join = handle.spawn(worker_loop(rx_request, latest_seq_worker, tx_node_changed));

        Some(Self {
            tx_request,
            latest_seq,
            seq_counter,
            _join: Arc::new(join),
        })
    }

    /// Enqueue an image-thumbnail job. Fire-and-forget. Supersedes older
    /// pending requests for the same `(node_id, output_index)`.
    pub fn request(
        &self,
        node_id: String,
        output_index: usize,
        change_id: String,
        data: Arc<FloatImage>,
    ) {
        self.enqueue(node_id, output_index, change_id, ThumbnailSource::Image(data));
    }

    /// Enqueue a video-thumbnail job (first-frame). Worker decodes frame 0
    /// via the shared [`crate::video::VideoDecoderCache`] before resizing.
    /// Uses the video's path as `change_id` — the UI stale-checks against
    /// the current `Value::Video`'s path so late thumbnails for a file the
    /// user has since swapped don't overwrite the correct preview.
    pub fn request_video(&self, node_id: String, output_index: usize, path: PathBuf) {
        let change_id = path.to_string_lossy().into_owned();
        self.enqueue(
            node_id,
            output_index,
            change_id,
            ThumbnailSource::Video { path },
        );
    }

    fn enqueue(
        &self,
        node_id: String,
        output_index: usize,
        change_id: String,
        source: ThumbnailSource,
    ) {
        let key = ThumbnailKey {
            node_id,
            output_index,
        };
        let seq = self.seq_counter.fetch_add(1, Ordering::Relaxed) + 1;
        self.latest_seq.insert(key.clone(), seq);

        let request = ThumbnailRequest {
            key,
            seq,
            change_id,
            source,
        };

        // Drop on channel-full. See module doc.
        let _ = self.tx_request.try_send(request);
    }

    /// Forget all pending requests for `node_id`. Called when a node is
    /// removed from the graph so late thumbnails for it don't arrive at the
    /// UI after the node is gone.
    pub fn forget_node(&self, node_id: &str) {
        // Removing the latest_seq entries makes any in-flight request for
        // those keys fail the stale check (since the key no longer maps to
        // their seq).
        self.latest_seq.retain(|k, _| k.node_id != node_id);
    }
}

async fn worker_loop(
    mut rx_request: mpsc::Receiver<ThumbnailRequest>,
    latest_seq: Arc<DashMap<ThumbnailKey, u64>>,
    tx_node_changed: Sender<NodeChangedMessage>,
) {
    while let Some(req) = rx_request.recv().await {
        // Stale check #1 — before compute. Weeds out requests superseded
        // while queued.
        let current_seq = latest_seq.get(&req.key).map(|s| *s);
        if current_seq != Some(req.seq) {
            continue;
        }

        let key = req.key.clone();
        let change_id = req.change_id.clone();
        let seq = req.seq;

        // Resolve the source to an Arc<FloatImage>. For Image this is
        // trivial; for Video we ask the decoder cache for frame 0, which
        // may take hundreds of ms but happens off the engine thread.
        let float_image = match resolve_source(req.source).await {
            Some(img) => img,
            None => continue,
        };

        // CPU-bound resize lives on the blocking pool so the worker task
        // itself stays free to drain the next request.
        let compute = tokio::task::spawn_blocking(move || {
            Thumbnail::Image(
                float_image
                    .resize_fit(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1])
                    .to_rgba8(),
            )
        });

        let thumbnail = match compute.await {
            Ok(thumb) => thumb,
            Err(err) => {
                eprintln!(
                    "thumbnail_service: compute task failed for {:?}: {}",
                    key, err
                );
                continue;
            }
        };

        // Stale check #2 — after compute, before send. Covers the window
        // where new requests arrived during the spawn_blocking / decode.
        let current_seq = latest_seq.get(&key).map(|s| *s);
        if current_seq != Some(seq) {
            continue;
        }

        let _ = tx_node_changed.try_send(NodeChangedMessage::ThumbnailReady {
            node_id: key.node_id,
            output_index: key.output_index,
            change_id,
            thumbnail,
        });
    }
}

/// Resolve a request's source into the `Arc<FloatImage>` that gets resized.
/// Image sources are already pixels — cheap. Video sources decode frame 0
/// via [`crate::video::VideoDecoderCache`]; returns `None` on decode error
/// (logged, request dropped).
async fn resolve_source(source: ThumbnailSource) -> Option<Arc<FloatImage>> {
    match source {
        ThumbnailSource::Image(arc) => Some(arc),
        #[cfg(feature = "video")]
        ThumbnailSource::Video { path } => {
            match crate::video::VideoDecoderCache::global().frame(&path, 0).await {
                Ok(frame) => Some(frame),
                Err(err) => {
                    eprintln!(
                        "thumbnail_service: video frame-0 decode failed for {}: {}",
                        path.display(),
                        err.0,
                    );
                    None
                }
            }
        }
        #[cfg(not(feature = "video"))]
        ThumbnailSource::Video { path: _ } => {
            // Without the video feature, Value::Video outputs can exist (to
            // support loading saved graphs) but there's no way to decode.
            // Drop silently — the run() stub has already errored on the node.
            None
        }
    }
}
