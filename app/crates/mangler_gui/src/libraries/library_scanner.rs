//! Background scanner that walks a library's root folder on disk and builds
//! a snapshot of its subfolders and `.mangler.json` graphs. Runs on a
//! dedicated `std::thread` (spawned by `Scanner::spawn`) so the UI thread
//! never blocks on filesystem I/O, which matters on slow/offline network
//! shares. The thread polls every `SCAN_INTERVAL`, publishing an immutable
//! snapshot into `Scanner::results`; `Scanner::request_rescan` lets the UI
//! ask for an immediate rescan after it makes a disk change of its own.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use eframe::egui;

use super::library::LibraryId;

/// How often the background thread rescans every library's root folder.
pub const SCAN_INTERVAL: Duration = Duration::from_secs(2);
/// Maximum folder nesting the scanner will descend into. Deeper folders are
/// still counted (their parent is marked `truncated`) but not walked, so a
/// pathological folder structure can't make the scan thread run forever.
pub const MAX_SCAN_DEPTH: usize = 8;
/// Maximum number of filesystem entries (files + folders, at any depth)
/// read per library per scan. Protects against huge folders (or a folder
/// with a symlink loop) turning every poll into a multi-second stall.
pub const MAX_ENTRIES_PER_LIBRARY: usize = 10_000;
/// The only file extension the scanner lists as a graph. Chosen to match
/// `Graph::save_to_file`'s on-disk naming.
pub const GRAPH_EXTENSION: &str = mangler_core::naming::GRAPH_EXTENSION;

/// One `.mangler.json` graph found while scanning a folder.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphEntry {
    /// Display name: the file name with the `.mangler.json` suffix stripped.
    pub name: String,
    /// Full path to the graph file on disk.
    pub path: PathBuf,
}

/// One image file found while scanning a folder. Unlike graphs, the display
/// name keeps its extension so `foo.png` and `foo.jpg` stay distinguishable.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageEntry {
    /// Display name: the file name *including* its extension.
    pub name: String,
    /// Full path to the image file on disk.
    pub path: PathBuf,
}

/// A snapshot of one folder's contents: its subfolders (recursively scanned)
/// and the graphs directly inside it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FolderScan {
    /// The folder's own name (its last path component).
    pub name: String,
    /// Full path to this folder on disk.
    pub path: PathBuf,
    /// Subfolders, sorted case-insensitively by name.
    pub folders: Vec<FolderScan>,
    /// Graphs directly inside this folder, sorted case-insensitively by name.
    pub graphs: Vec<GraphEntry>,
    /// Image files directly inside this folder, sorted case-insensitively by
    /// name (extension included).
    pub images: Vec<ImageEntry>,
    /// `true` if this folder has content the scanner didn't fully walk
    /// (depth cap reached, or the per-library entry budget ran out). The
    /// panel shows this as a "more items not shown" marker rather than
    /// silently hiding data.
    pub truncated: bool,
}

/// Scan outcome for a single library.
#[derive(Debug, Clone, PartialEq)]
pub enum LibraryScan {
    /// The root folder was read successfully.
    Ok(FolderScan),
    /// The root folder couldn't be read (missing, permission denied, an
    /// offline network share, ...). Carries the OS error message so the
    /// panel can show it; scanning other libraries continues unaffected.
    Unavailable(String),
}

/// Latest scan snapshot for every library, keyed by `LibraryId`.
pub type ScanResults = HashMap<LibraryId, LibraryScan>;

/// Returns `true` if `file_name` should be listed as a graph, i.e. ends in
/// the literal `.mangler.json` suffix (and isn't just the bare suffix with no
/// stem, which would be a strange/unusable name).
pub fn is_graph_file(file_name: &str) -> bool {
    file_name.ends_with(GRAPH_EXTENSION) && file_name.len() > GRAPH_EXTENSION.len()
}

/// Returns `true` if `file_name` has an image extension the app can load,
/// matched case-insensitively. Reuses the exact extension list the
/// "image from file" node advertises (`ValueType::file_extensions` for
/// `Image`), so what the panel lists and what the node can open never drift.
pub fn is_image_file(file_name: &str) -> bool {
    let Some(ext) = Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
    else {
        return false;
    };
    let ext = ext.to_lowercase();
    mangler_core::value::ValueType::file_extensions(&mangler_core::value::ValueType::Image)
        .iter()
        .any(|e| e.eq_ignore_ascii_case(&ext))
}

/// Derives a graph's display name from its file name by stripping the
/// `.mangler.json` suffix. Never parses the file itself — scans must stay
/// fast on network shares, so the on-disk name is all we show.
///
/// Thin wrapper over `mangler_core::naming::graph_display_name` (the single
/// source of truth for this rule).
pub fn graph_display_name(file_name: &str) -> String {
    mangler_core::naming::graph_display_name(file_name)
}

/// Recursively scans `path`, listing its immediate subfolders and graphs and
/// (depth/budget permitting) recursing into each subfolder.
///
/// `depth` is the nesting level of `path` relative to the library root (the
/// root itself is depth 0). `budget` is a shared entry counter decremented
/// once per filesystem entry visited across the *whole* library scan (not
/// just this folder) — passing `&mut` down the recursion lets every level
/// see the same remaining budget.
///
/// Returns `Err` only if `path` itself can't be read (used by the root call
/// to build `LibraryScan::Unavailable`); unreadable *entries* or *subfolders*
/// found while walking are skipped individually so one bad item doesn't
/// blank out an entire library.
pub fn scan_folder(path: &Path, depth: usize, budget: &mut usize) -> std::io::Result<FolderScan> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut scan = FolderScan {
        name,
        path: path.to_path_buf(),
        folders: Vec::new(),
        graphs: Vec::new(),
        images: Vec::new(),
        truncated: false,
    };

    // Propagate: an unreadable root is how the caller detects a
    // missing/offline library and turns it into `LibraryScan::Unavailable`.
    let read_dir = std::fs::read_dir(path)?;

    // First pass: classify this folder's own entries into subfolder paths
    // and graph files. We always read this folder's own contents, even at
    // the depth cap -- only *recursing further* is skipped below.
    let mut subfolder_paths: Vec<PathBuf> = Vec::new();
    for entry in read_dir {
        // Skip individual entries that fail to read (permission error,
        // deleted mid-scan, ...) rather than failing the whole folder.
        let Ok(entry) = entry else { continue };

        if *budget == 0 {
            scan.truncated = true;
            break;
        }
        *budget -= 1;

        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let entry_path = entry.path();

        if file_type.is_dir() {
            subfolder_paths.push(entry_path);
        } else if file_type.is_file() && is_graph_file(&file_name) {
            scan.graphs.push(GraphEntry {
                name: graph_display_name(&file_name),
                path: entry_path,
            });
        } else if file_type.is_file() && is_image_file(&file_name) {
            scan.images.push(ImageEntry {
                // Keep the extension visible so foo.png / foo.jpg are distinct.
                name: file_name,
                path: entry_path,
            });
        }
    }

    if depth >= MAX_SCAN_DEPTH {
        // At the depth cap: this folder's own graphs are already listed
        // above, but we don't walk any deeper. Flag it as truncated so the
        // UI can show "more items not shown" instead of silently pretending
        // this folder has no subfolders.
        if !subfolder_paths.is_empty() {
            scan.truncated = true;
        }
    } else {
        for subfolder_path in subfolder_paths {
            if *budget == 0 {
                scan.truncated = true;
                break;
            }
            match scan_folder(&subfolder_path, depth + 1, budget) {
                Ok(sub) => scan.folders.push(sub),
                // Unreadable subfolder: skip it, don't fail the parent scan.
                Err(_) => continue,
            }
        }
    }

    // Case-insensitive sort keeps the panel's tree stable and predictable
    // regardless of filesystem enumeration order.
    scan.folders
        .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    scan.graphs
        .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    scan.images
        .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(scan)
}

/// Runs `scan_folder` against a library's root and turns an I/O error into
/// `LibraryScan::Unavailable` (a missing/offline root should never abort the
/// whole scan pass, just that one library's entry).
fn scan_library_root(root: &Path) -> LibraryScan {
    let mut budget = MAX_ENTRIES_PER_LIBRARY;
    match scan_folder(root, 0, &mut budget) {
        Ok(folder_scan) => LibraryScan::Ok(folder_scan),
        Err(err) => LibraryScan::Unavailable(err.to_string()),
    }
}

/// Owns the background scan thread and the latest published snapshot.
///
/// `results` is the only field the UI reads from every frame; `roots` and
/// `rescan_now` are write-only from the UI's perspective (set via
/// `set_roots`/`request_rescan`) and read-only from the scan thread's
/// perspective. Splitting them into separate `Arc<Mutex<_>>`s means the UI
/// updating `roots` never contends with the scan thread reading `results`.
pub struct Scanner {
    /// Latest snapshot of every library's folder tree. Cloned/locked briefly
    /// by the UI each frame it needs to draw the panel.
    pub results: Arc<Mutex<ScanResults>>,
    /// The set of libraries to scan, as `(id, root path)` pairs. Replaced
    /// wholesale by `set_roots` whenever the library list changes.
    roots: Arc<Mutex<Vec<(LibraryId, PathBuf)>>>,
    /// Set by `request_rescan` to wake the scan thread from its sleep early;
    /// the thread swaps it back to `false` once it acts on it.
    rescan_now: Arc<AtomicBool>,
}

impl Scanner {
    /// Spawns the background scan thread and returns a handle to it. The
    /// thread is detached (not joined) -- it lives for the lifetime of the
    /// process, same as the rest of the app's background workers.
    ///
    /// `ctx` is used only to call `request_repaint()` after a snapshot
    /// actually changes, so an idle app doesn't busy-repaint every poll.
    pub fn spawn(ctx: egui::Context) -> Self {
        let results: Arc<Mutex<ScanResults>> = Arc::new(Mutex::new(ScanResults::new()));
        let roots: Arc<Mutex<Vec<(LibraryId, PathBuf)>>> = Arc::new(Mutex::new(Vec::new()));
        let rescan_now = Arc::new(AtomicBool::new(false));

        let thread_results = Arc::clone(&results);
        let thread_roots = Arc::clone(&roots);
        let thread_rescan_now = Arc::clone(&rescan_now);

        thread::spawn(move || {
            loop {
                // Snapshot the roots under a short lock, then release it
                // before doing any filesystem I/O so `set_roots` never
                // blocks on a slow/offline scan in progress.
                let roots_snapshot: Vec<(LibraryId, PathBuf)> =
                    thread_roots.lock().unwrap().clone();

                let mut new_results: ScanResults = HashMap::with_capacity(roots_snapshot.len());
                for (id, root_path) in &roots_snapshot {
                    new_results.insert(*id, scan_library_root(root_path));
                }

                // Only take the results lock (and only repaint) if the
                // snapshot actually differs from what's published -- avoids
                // spamming repaints when nothing on disk changed.
                let changed = {
                    let mut guard = thread_results.lock().unwrap();
                    if *guard != new_results {
                        *guard = new_results;
                        true
                    } else {
                        false
                    }
                };
                if changed {
                    ctx.request_repaint();
                }

                // Sleep up to SCAN_INTERVAL in short slices so a
                // UI-requested rescan (e.g. right after the user creates a
                // graph) doesn't have to wait out the full interval.
                let mut slept = Duration::ZERO;
                while slept < SCAN_INTERVAL {
                    if thread_rescan_now.swap(false, Ordering::SeqCst) {
                        break;
                    }
                    let slice = Duration::from_millis(100).min(SCAN_INTERVAL - slept);
                    thread::sleep(slice);
                    slept += slice;
                }
            }
        });

        Scanner {
            results,
            roots,
            rescan_now,
        }
    }

    /// Replaces the set of libraries to scan and immediately requests a
    /// rescan, so a newly added library shows up within one poll slice
    /// instead of waiting up to `SCAN_INTERVAL`.
    pub fn set_roots(&self, roots: Vec<(LibraryId, PathBuf)>) {
        *self.roots.lock().unwrap() = roots;
        self.request_rescan();
    }

    /// Asks the scan thread to rescan as soon as possible instead of waiting
    /// out its current sleep. Used after UI-initiated disk operations
    /// (create/rename/delete) so the panel updates without a visible delay.
    pub fn request_rescan(&self) {
        self.rescan_now.store(true, Ordering::SeqCst);
    }
}

#[cfg(test)]
#[path = "library_scanner_tests.rs"]
mod tests;
