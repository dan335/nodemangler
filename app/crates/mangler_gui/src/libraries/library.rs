//! Data model for a "library": a named link to a folder on disk containing
//! saved `.mangler.json` graphs. Only the link (name + source) is persisted
//! in `AppConfig`; the folder tree itself is rescanned from disk each
//! session by `library_scanner`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Identifies a library within a running session. Assigned by
/// `LibrariesState` when libraries are loaded/created; not persisted itself
/// (only the `LibraryConfig` it points at is saved to disk).
pub type LibraryId = u64;

/// Where a library's content lives. Currently only local (or network-share,
/// via a UNC/mapped-drive `path`) folders are supported. This is an enum
/// rather than a bare `PathBuf` so a future remote, read-only "public
/// NodeMangler library" can be added as a new variant without changing the
/// shape of `LibraryConfig`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LibrarySource {
    /// A folder on local disk or a mounted network share.
    Local { path: PathBuf },
}

impl LibrarySource {
    /// Whether this source forbids create/rename/delete operations. Local
    /// sources are always writable; a future remote source would return
    /// `true` here.
    pub fn read_only(&self) -> bool {
        match self {
            LibrarySource::Local { .. } => false,
        }
    }

    /// The on-disk root path for this source, if it has one. `Local` always
    /// has one; a future remote source would return `None`.
    pub fn local_path(&self) -> Option<&Path> {
        match self {
            LibrarySource::Local { path } => Some(path.as_path()),
        }
    }
}

/// One persisted library entry, stored in `AppConfig::libraries`. The
/// display name is independent of the folder name so users can rename a
/// library without touching disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibraryConfig {
    /// User-facing name shown in the Libraries panel.
    pub name: String,
    /// Where the library's graphs live.
    pub source: LibrarySource,
}

#[cfg(test)]
#[path = "library_tests.rs"]
mod tests;
