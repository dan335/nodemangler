//! On-disk thumbnail cache for the Libraries panel.
//!
//! Cache entries live under `dirs::cache_dir()/NodeMangler/thumbs/…`. The
//! validation key is **in the filename** (`hash_mtime_len`), so a hit is a
//! single `File::open` of an exactly-computed path — no index, no header
//! parse. Changing the source file's mtime or size misses naturally; bumping
//! [`super::thumb_decode::LIBRARY_THUMB_MAX`] changes the directory segment
//! and invalidates the whole set.
//!
//! Metadata is read lazily on the worker (`stat` at decode time), not on the
//! scanner poll, so network shares aren't taxed every 2s for every image.

use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::thumb_decode::LIBRARY_THUMB_MAX;

/// Directory layout version. Bump when the on-disk format changes incompatibly.
const CACHE_FORMAT_VERSION: u32 = 1;

/// JPEG quality for opaque thumbs (~few KB at 192 px).
const CACHE_JPEG_QUALITY: u8 = 85;

/// Source-file fingerprint used as part of the cache filename.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileFingerprint {
    pub mtime_ns: u64,
    pub len: u64,
}

/// Stats `path` for the cache key. `None` if the file is missing/unreadable.
pub fn fingerprint(path: &Path) -> Option<FileFingerprint> {
    let meta = fs::metadata(path).ok()?;
    let mtime_ns = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as u64)?;
    Some(FileFingerprint {
        mtime_ns,
        len: meta.len(),
    })
}

/// Pure path construction for tests (no filesystem).
pub fn cache_relative_path(path: &Path, fp: FileFingerprint, max_edge: u32) -> PathBuf {
    let hash = path_hash(path, max_edge);
    let shard = format!("{:02x}", (hash >> 56) as u8);
    let name = format!("{:016x}_{}_{}.jpg", hash, fp.mtime_ns, fp.len);
    PathBuf::from(format!("v{CACHE_FORMAT_VERSION}_m{max_edge}"))
        .join(shard)
        .join(name)
}

/// Absolute cache file path for `path` at its current fingerprint, if known.
pub fn cache_path_for(path: &Path) -> Option<PathBuf> {
    let fp = fingerprint(path)?;
    let root = cache_root()?;
    Some(root.join(cache_relative_path(path, fp, LIBRARY_THUMB_MAX)))
}

/// Root of the thumb cache, or `None` if the platform has no cache dir.
pub fn cache_root() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join("NodeMangler").join("thumbs"))
}

/// Loads a cached RGBA8 thumb for `path` if a valid file exists.
pub fn try_load(path: &Path) -> Option<(Vec<u8>, [usize; 2])> {
    let cache_path = cache_path_for(path)?;
    // Opaque thumbs are `.jpg`; alpha sources write `.png` instead.
    load_cache_file(&cache_path).or_else(|| load_cache_file(&cache_path.with_extension("png")))
}

fn load_cache_file(cache_path: &Path) -> Option<(Vec<u8>, [usize; 2])> {
    let file = File::open(cache_path).ok()?;
    let reader = BufReader::new(file);
    let dyn_img = image::ImageReader::new(reader)
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?;
    let rgba = dyn_img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    // Guard against a corrupt/truncated cache file.
    if size[0] == 0 || size[1] == 0 || size[0].max(size[1]) > LIBRARY_THUMB_MAX as usize * 2 {
        return None;
    }
    Some((rgba.into_raw(), size))
}

/// Writes `rgba` (w×h) into the cache for `path`. Best-effort — failures are
/// silent (cache is an optimisation, not a correctness path).
pub fn store(path: &Path, rgba: &[u8], width: u32, height: u32) {
    if width == 0 || height == 0 {
        return;
    }
    let Some(cache_path) = cache_path_for(path) else {
        return;
    };
    if let Some(parent) = cache_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let Some(img) = image::RgbaImage::from_raw(width, height, rgba.to_vec()) else {
        return;
    };

    // Prefer JPEG for opaque thumbs; PNG if any pixel has alpha < 255.
    let has_alpha = rgba.chunks_exact(4).any(|p| p[3] < 255);
    if has_alpha {
        let png_path = cache_path.with_extension("png");
        let _ = image::DynamicImage::ImageRgba8(img).save_with_format(&png_path, image::ImageFormat::Png);
        // Don't leave a stale .jpg next to a newer .png for the same key.
        let _ = fs::remove_file(&cache_path);
    } else {
        // Encode RGB JPEG (drop alpha).
        let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
        if let Ok(file) = File::create(&cache_path) {
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, CACHE_JPEG_QUALITY);
            let _ = encoder.encode(
                rgb.as_raw(),
                width,
                height,
                image::ExtendedColorType::Rgb8,
            );
        }
        let _ = fs::remove_file(cache_path.with_extension("png"));
    }
}

/// Decode path used by workers: disk hit first, else `decode`, then store.
pub fn load_or_decode(
    path: &Path,
    decode: impl FnOnce(&Path) -> Result<(Vec<u8>, [usize; 2]), ()>,
) -> Result<(Vec<u8>, [usize; 2]), ()> {
    if let Some(hit) = try_load(path) {
        return Ok(hit);
    }
    let decoded = decode(path)?;
    store(
        path,
        &decoded.0,
        decoded.1[0] as u32,
        decoded.1[1] as u32,
    );
    Ok(decoded)
}

fn path_hash(path: &Path, max_edge: u32) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    max_edge.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
#[path = "thumb_disk_cache_tests.rs"]
mod tests;
