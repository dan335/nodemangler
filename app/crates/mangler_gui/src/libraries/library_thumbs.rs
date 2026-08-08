//! Background thumbnail cache for the Libraries panel's thumbnails view.
//!
//! A small persistent worker pool decodes images off the UI thread, resizes
//! them to a fixed max edge, and uploads `egui` textures on the next
//! [`LibraryThumbCache::poll`]. Jobs are a priority queue: **visible** cells
//! go to the front, **prefetch** (near-viewport) cells to the back, so what
//! the user is looking at always wins. The cache is bounded (LRU for Ready
//! textures; age-sweep only for abandoned Loading/Failed) and shared by every
//! Libraries panel leaf via [`super::libraries_state::LibrariesState`].
//!
//! The panel enqueues for a band around the clip rect (not only strictly
//! visible cells) so scrolling lands on already-decoded thumbs. Already-Ready
//! textures stay cached while scrolled off so scrolling back does not
//! re-decode. Workers consult [`super::thumb_disk_cache`] first so a second
//! visit (or cold process) reloads ~ms JPEGs instead of re-decoding sources.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use eframe::egui;
use egui::ColorImage;

/// Default on-screen cell image size in points.
pub const LIBRARY_THUMB_UI: f32 = 80.0;
/// Below this available width, the panel falls back to list rows for images.
pub const LIBRARY_THUMB_MIN_UI: f32 = 56.0;
/// How many viewport heights above **and** below the clip rect to prefetch.
/// 1.5 ≈ a screen and a half of runway in each scroll direction.
pub const LIBRARY_THUMB_PREFETCH_VIEWPORTS: f32 = 1.5;
/// Persistent worker threads (I/O may block a worker for seconds on shares).
const LIBRARY_THUMB_WORKERS: usize = 4;
/// Hard cap on cached texture entries. Ready textures survive scroll-off;
/// only this cap (and [`LibraryThumbCache::invalidate_all`]) drops them.
const LIBRARY_THUMB_LRU_CAP: usize = 512;
/// Frames without a touch before an abandoned **Loading**/**Failed** slot is
/// dropped. Ready textures are **not** age-swept — scrolling away must not
/// force a re-decode when the user scrolls back.
const LIBRARY_THUMB_SWEEP_AGE: u64 = 120;

/// One finished (or failed) decode delivered by a worker.
struct ThumbResult {
    path: PathBuf,
    /// RGBA8 pixels + size on success.
    pixels: Result<(Vec<u8>, [usize; 2]), ()>,
}

/// Per-path cache slot.
enum ThumbSlot {
    /// Enqueued or in-flight; show placeholder.
    Loading { last_seen_frame: u64 },
    Ready {
        texture: egui::TextureHandle,
        last_seen_frame: u64,
    },
    /// Decode failed; keep as placeholder so we don't re-spam the pool.
    Failed { last_seen_frame: u64 },
}

impl ThumbSlot {
    fn last_seen(&self) -> u64 {
        match self {
            ThumbSlot::Loading { last_seen_frame }
            | ThumbSlot::Ready { last_seen_frame, .. }
            | ThumbSlot::Failed { last_seen_frame } => *last_seen_frame,
        }
    }

    fn set_last_seen(&mut self, frame: u64) {
        match self {
            ThumbSlot::Loading { last_seen_frame }
            | ThumbSlot::Ready { last_seen_frame, .. }
            | ThumbSlot::Failed { last_seen_frame } => *last_seen_frame = frame,
        }
    }
}

/// Shared thumbnail cache for the Libraries panel.
pub struct LibraryThumbCache {
    entries: HashMap<PathBuf, ThumbSlot>,
    /// Least-recently-touched at the front; most recent at the back.
    lru: VecDeque<PathBuf>,
    /// Paths sitting on the job queue or currently held by a worker.
    queued: HashSet<PathBuf>,
    /// Priority job queue: front = high (visible), back = low (prefetch).
    jobs: Arc<Mutex<VecDeque<PathBuf>>>,
    results_rx: Receiver<ThumbResult>,
    /// Kept so Drop can close the channel if needed; workers hold clones.
    _results_tx: Sender<ThumbResult>,
    /// Frame counter for mark-and-sweep (bumped each [`Self::poll`]).
    frame: u64,
}

impl LibraryThumbCache {
    /// Spawns the worker pool and returns an empty cache.
    ///
    /// `ctx` is cloned into each worker so they can `request_repaint` when a
    /// decode finishes (same wake-up pattern as the library scanner).
    pub fn spawn(ctx: egui::Context) -> Self {
        let jobs: Arc<Mutex<VecDeque<PathBuf>>> = Arc::new(Mutex::new(VecDeque::new()));
        let (results_tx, results_rx) = mpsc::channel();

        for _ in 0..LIBRARY_THUMB_WORKERS {
            let jobs = Arc::clone(&jobs);
            let results_tx = results_tx.clone();
            let ctx = ctx.clone();
            thread::spawn(move || worker_loop(jobs, results_tx, ctx));
        }

        Self {
            entries: HashMap::new(),
            lru: VecDeque::new(),
            queued: HashSet::new(),
            jobs,
            results_rx,
            _results_tx: results_tx,
            frame: 0,
        }
    }

    /// Drains finished worker results into texture slots, then sweeps stale
    /// entries and enforces the LRU cap. Call once per frame **before** the
    /// panel tree requests thumbs (and before locking the scan snapshot).
    pub fn poll(&mut self, ctx: &egui::Context) {
        self.frame = self.frame.wrapping_add(1);

        while let Ok(result) = self.results_rx.try_recv() {
            self.queued.remove(&result.path);

            // Only apply if still expected (Loading). Late results after
            // invalidate_all, LRU eviction, or sweep are dropped.
            let still_loading = matches!(
                self.entries.get(&result.path),
                Some(ThumbSlot::Loading { .. })
            );
            if !still_loading {
                continue;
            }

            match result.pixels {
                Ok((rgba, size)) => {
                    let color_image = ColorImage::from_rgba_unmultiplied(size, &rgba);
                    let texture = ctx.load_texture(
                        format!("lib_thumb:{}", result.path.display()),
                        color_image,
                        Default::default(),
                    );
                    self.entries.insert(
                        result.path.clone(),
                        ThumbSlot::Ready {
                            texture,
                            last_seen_frame: self.frame,
                        },
                    );
                    touch_lru(&mut self.lru, &result.path);
                }
                Err(()) => {
                    self.entries.insert(
                        result.path.clone(),
                        ThumbSlot::Failed {
                            last_seen_frame: self.frame,
                        },
                    );
                    touch_lru(&mut self.lru, &result.path);
                }
            }
        }

        self.sweep_stale();
        self.evict_over_cap();
    }

    /// High-priority request for a **visible** cell. Enqueues at the front of
    /// the queue (or promotes an existing prefetch entry) and returns a ready
    /// texture if one is available.
    pub fn get(&mut self, path: &Path) -> Option<&egui::TextureHandle> {
        self.request(path, true);
        match self.entries.get(path) {
            Some(ThumbSlot::Ready { texture, .. }) => Some(texture),
            _ => None,
        }
    }

    /// Low-priority request for a **near-viewport** cell (prefetch band).
    /// Does not promote already-queued work to the front; just ensures a
    /// decode is scheduled so scrolling lands on ready thumbs.
    pub fn prefetch(&mut self, path: &Path) {
        self.request(path, false);
    }

    fn request(&mut self, path: &Path, high_priority: bool) {
        let path_buf = path.to_path_buf();

        if let Some(slot) = self.entries.get_mut(&path_buf) {
            slot.set_last_seen(self.frame);
            touch_lru(&mut self.lru, &path_buf);
            // Promote a waiting low-priority job when the cell becomes visible.
            if high_priority && matches!(slot, ThumbSlot::Loading { .. }) {
                promote_job(&self.jobs, &path_buf);
            }
            return;
        }

        // Miss: reserve Loading and enqueue once.
        self.entries.insert(
            path_buf.clone(),
            ThumbSlot::Loading {
                last_seen_frame: self.frame,
            },
        );
        touch_lru(&mut self.lru, &path_buf);
        if self.queued.insert(path_buf.clone()) {
            let mut jobs = self.jobs.lock().unwrap();
            enqueue_job(&mut jobs, path_buf, high_priority);
        }
    }

    /// Drops every in-memory texture and pending job. Not used on routine
    /// rescans (disk keys include mtime/size); kept for a future "clear
    /// thumbnail cache" action or tests.
    #[allow(dead_code)]
    pub fn invalidate_all(&mut self) {
        self.entries.clear();
        self.lru.clear();
        self.queued.clear();
        self.jobs.lock().unwrap().clear();
        // Drain any results already in the channel so they don't re-populate
        // entries for paths we just cleared (workers may still finish late;
        // poll only applies when slot is Loading).
        while self.results_rx.try_recv().is_ok() {}
    }

    fn sweep_stale(&mut self) {
        // Only purge abandoned Loading/Failed slots. Ready textures stay until
        // LRU eviction or invalidate_all — otherwise scrolling off a row for a
        // couple of seconds drops the GPU upload and forces a full re-decode
        // when the user scrolls back up.
        let frame = self.frame;
        let stale: Vec<PathBuf> = self
            .entries
            .iter()
            .filter(|(_, slot)| {
                matches!(slot, ThumbSlot::Loading { .. } | ThumbSlot::Failed { .. })
                    && is_stale(slot.last_seen(), frame, LIBRARY_THUMB_SWEEP_AGE)
            })
            .map(|(path, _)| path.clone())
            .collect();
        for path in stale {
            self.entries.remove(&path);
            self.queued.remove(&path);
            self.lru.retain(|p| p != &path);
        }
    }

    fn evict_over_cap(&mut self) {
        while should_evict_lru(self.entries.len(), LIBRARY_THUMB_LRU_CAP) {
            let Some(old) = self.lru.pop_front() else {
                break;
            };
            self.entries.remove(&old);
            self.queued.remove(&old);
        }
    }
}

fn worker_loop(
    jobs: Arc<Mutex<VecDeque<PathBuf>>>,
    results_tx: Sender<ThumbResult>,
    ctx: egui::Context,
) {
    loop {
        let path = {
            let mut guard = jobs.lock().unwrap();
            next_job(&mut guard)
        };

        let Some(path) = path else {
            // Idle: short sleep so we don't spin; UI enqueue will be picked
            // up within this slice (same spirit as the library scanner).
            thread::sleep(Duration::from_millis(50));
            continue;
        };

        // Disk hit first (path+mtime+size key); miss → decode → write cache.
        let pixels = super::thumb_disk_cache::load_or_decode(&path, |p| {
            super::thumb_decode::decode_thumb(p)
        });
        // If the UI dropped the receiver (cache destroyed), exit.
        if results_tx
            .send(ThumbResult { path, pixels })
            .is_err()
        {
            break;
        }
        ctx.request_repaint();
    }
}

/// Pops the next job (front of the priority queue). Pure helper for tests.
pub fn next_job(jobs: &mut VecDeque<PathBuf>) -> Option<PathBuf> {
    jobs.pop_front()
}

/// Enqueues `path` at the front (high) or back (low). Pure helper for tests.
pub fn enqueue_job(jobs: &mut VecDeque<PathBuf>, path: PathBuf, high_priority: bool) {
    if high_priority {
        jobs.push_front(path);
    } else {
        jobs.push_back(path);
    }
}

/// Moves an already-queued path to the front (visible cell). No-op if absent.
fn promote_job(jobs: &Mutex<VecDeque<PathBuf>>, path: &Path) {
    let mut guard = jobs.lock().unwrap();
    if let Some(pos) = guard.iter().position(|p| p == path) {
        if pos > 0 {
            if let Some(p) = guard.remove(pos) {
                guard.push_front(p);
            }
        }
    }
}

/// Whether the LRU should drop an entry given current length and cap.
pub fn should_evict_lru(len: usize, cap: usize) -> bool {
    len > cap
}

/// True when `last_seen` is more than `age` frames behind `frame`.
pub fn is_stale(last_seen: u64, frame: u64, age: u64) -> bool {
    frame.saturating_sub(last_seen) > age
}

/// Expanded clip band used to decide whether a cell should prefetch.
/// Pure helper so the margin math is unit-testable.
pub fn prefetch_band(clip: egui::Rect, viewports: f32) -> egui::Rect {
    let margin_y = (clip.height() * viewports).max(LIBRARY_THUMB_UI * 4.0);
    egui::Rect::from_min_max(
        egui::pos2(clip.min.x, clip.min.y - margin_y),
        egui::pos2(clip.max.x, clip.max.y + margin_y),
    )
}

fn touch_lru(lru: &mut VecDeque<PathBuf>, path: &Path) {
    lru.retain(|p| p != path);
    lru.push_back(path.to_path_buf());
}

#[cfg(test)]
#[path = "library_thumbs_tests.rs"]
mod tests;
