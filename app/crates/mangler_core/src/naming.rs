//! Centralized graph file naming: the canonical extension, display-name
//! derivation from a file name, and filesystem-safe sanitization of
//! user-entered names.
//!
//! This consolidates logic that used to be duplicated (and drifting) across
//! the GUI's library scanner, libraries panel, and graph-settings save
//! dialog: one sanitizer, one extension constant, one display-name rule.

/// The canonical extension for saved NodeMangler graph files.
pub const GRAPH_EXTENSION: &str = ".mangler.json";

/// Derives a graph's display name from its file name.
///
/// Strips the canonical `.mangler.json` suffix if present. Otherwise strips a
/// plain `.json` suffix, so foreign/legacy `.json` files (from before this
/// extension existed, or dropped in by hand) still get a sensible display
/// name instead of showing their extension. If neither suffix matches, the
/// name is returned unchanged.
pub fn graph_display_name(file_name: &str) -> String {
    if let Some(stripped) = file_name.strip_suffix(GRAPH_EXTENSION) {
        return stripped.to_string();
    }
    if let Some(stripped) = file_name.strip_suffix(".json") {
        return stripped.to_string();
    }
    file_name.to_string()
}

/// Convenience wrapper for callers that only have a [`std::path::Path`]
/// (e.g. a node's `subgraph_path`) rather than a bare file-name string.
/// Falls back to the full path's display form if it has no file-name
/// component or the file name isn't valid UTF-8.
pub fn graph_display_name_from_path(path: &std::path::Path) -> String {
    match path.file_name().and_then(|s| s.to_str()) {
        Some(file_name) => graph_display_name(file_name),
        None => path.display().to_string(),
    }
}

/// Sanitizes a user-entered name into something safe to use as a file or
/// folder name: strips characters illegal on Windows/macOS/Linux filesystems
/// and Windows-reserved device names (`con`, `nul`, ...).
///
/// Spaces are preserved — display names and on-disk file-name stems must
/// agree with each other, and mangling spaces into underscores broke that.
pub fn sanitize_name(name: &str) -> String {
    let options = sanitize_filename::Options {
        truncate: true,  // truncate to 255 bytes
        windows: true,   // strip Windows-reserved names like `con`
        replacement: "", // drop disallowed characters entirely
    };
    sanitize_filename::sanitize_with_options(name, options)
}

/// Sanitizes `name` and appends [`GRAPH_EXTENSION`], producing the on-disk
/// file name for a saved graph.
pub fn graph_file_name(name: &str) -> String {
    format!("{}{}", sanitize_name(name), GRAPH_EXTENSION)
}

/// Picks the first `{base}_{N}` stem (N ≥ 1) that is neither in `taken` nor
/// already a file stem inside `dir`.
///
/// This is the one rule behind every auto-named output file: a fresh output
/// node's `{graph name}_{N}` (see `Graph::add_node`) and the `to file` node
/// created alongside a dropped image (`{image stem}_{N}`). `taken` carries the
/// stems other output nodes in the graph already claim; `dir` (when given and
/// readable) adds what is on disk, extension ignored, so a stem is considered
/// used whatever its extension — `photo_1.png` blocks `photo_1.jpg` too, which
/// is what stops a new node quietly overwriting an unrelated file. Comparison
/// is case-insensitive so it holds on case-insensitive filesystems; an
/// unreadable or nonexistent `dir` simply contributes nothing.
pub fn unique_stem(
    base: &str,
    dir: Option<&std::path::Path>,
    taken: impl IntoIterator<Item = String>,
) -> String {
    let mut used: std::collections::HashSet<String> =
        taken.into_iter().map(|s| s.trim().to_lowercase()).collect();

    // Stems already present on disk in the target folder (extension stripped).
    if let Some(dir) = dir {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Some(stem) = std::path::Path::new(&entry.file_name()).file_stem() {
                    used.insert(stem.to_string_lossy().to_lowercase());
                }
            }
        }
    }

    let mut n: u32 = 1;
    loop {
        let candidate = format!("{}_{}", base, n);
        if !used.contains(&candidate.to_lowercase()) {
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
#[path = "naming_tests.rs"]
mod tests;
