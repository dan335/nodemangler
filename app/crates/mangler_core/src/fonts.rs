//! Installed-font discovery for the `from text` node.
//!
//! The node used to rasterise with one font compiled into the binary. This
//! module finds the faces installed on the machine so the node can offer them
//! as a dropdown, and hands back an `ab_glyph` font for whichever one is
//! selected.
//!
//! ## Why the scan reads table directories instead of whole files
//! A Windows install has several hundred faces and a CJK font is tens of
//! megabytes, so slurping every file to read one string would put seconds into
//! [`options`] — which `create_inputs()` calls, meaning every node creation and
//! every graph load. Instead [`read_name_table`] parses the sfnt header and
//! table directory (a few hundred bytes) and then reads *only* the `name`
//! table. That is two short seeks per file rather than one full read.
//!
//! ## Caching
//! Two caches, for two different costs. [`installed`] holds the scan behind a
//! `OnceLock`: it happens once per process, and every later call is a slice
//! read. [`load`] memoises the parsed fonts themselves, because a font is only
//! read when it is actually used to render — but that cache is *bounded*, since
//! clicking down a 400-entry dropdown would otherwise load every font on the
//! system into memory.
//!
//! ## Missing fonts are not an error
//! A graph carries a family *name*, so opening it on a machine without that
//! font falls back to the built-in rather than failing — the same thing every
//! design tool does. The name stays in the input, so the graph renders
//! correctly again on a machine that has it (the settings panel's dropdown
//! displays its raw value, so the missing family is still visible).

use ab_glyph::{FontArc, FontVec};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Dropdown entry for the font compiled into the binary. Also the default.
pub const BUILT_IN: &str = "Manrope (built-in)";

/// Bytes of the embedded font, used for [`BUILT_IN`] and as the fallback
/// whenever a selected family cannot be found or parsed.
static BUILT_IN_BYTES: &[u8] = include_bytes!("../assets/Manrope-Regular.ttf");

/// File extensions worth opening. Bitmap and Type-1 formats are skipped —
/// `ab_glyph` cannot rasterise them, so listing them would offer choices that
/// silently fall back.
const FONT_EXTENSIONS: [&str; 4] = ["ttf", "otf", "ttc", "otc"];

/// How deep to walk a font directory. Linux nests by foundry and family
/// (`/usr/share/fonts/truetype/dejavu/`); nothing sane goes deeper.
const MAX_SCAN_DEPTH: usize = 4;

/// Parsed fonts kept in memory. Cleared wholesale once it exceeds this, which
/// bounds the damage from a user arrowing down the whole dropdown; re-loading
/// a font is one file read.
const MAX_CACHED_FONTS: usize = 16;

/// One selectable face.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Face {
    /// Name shown in the dropdown, e.g. `"Segoe UI Semibold"`.
    pub name: String,
    /// File the face lives in.
    pub path: PathBuf,
    /// Index within a `.ttc`/`.otc` collection; 0 for a single-font file.
    pub index: u32,
}

/// Every installed face, sorted by name and de-duplicated. Scanned once.
pub fn installed() -> &'static [Face] {
    static FACES: OnceLock<Vec<Face>> = OnceLock::new();
    FACES.get_or_init(|| collect(&font_directories()))
}

/// Dropdown options: the built-in font first, then everything installed.
///
/// The built-in leads because it is the default and the one entry guaranteed to
/// exist on every machine.
pub fn options() -> Vec<String> {
    let mut out = Vec::with_capacity(installed().len() + 1);
    out.push(BUILT_IN.to_string());
    out.extend(installed().iter().map(|f| f.name.clone()));
    out
}

/// The font to rasterise with for a dropdown value.
///
/// Never fails: an empty, unknown or unparsable selection yields the built-in
/// font. Returns whether the request was honoured so a caller can say which
/// font it actually used.
pub fn load(name: &str) -> (FontArc, bool) {
    if name.is_empty() || name == BUILT_IN {
        return (built_in(), true);
    }
    let Some(face) = installed().iter().find(|f| f.name == name) else {
        return (built_in(), false);
    };

    let cache = cache();
    if let Ok(map) = cache.lock() {
        if let Some(font) = map.get(name) {
            return (font.clone(), true);
        }
    }

    let Some(font) = read_face(face) else { return (built_in(), false) };
    if let Ok(mut map) = cache.lock() {
        if map.len() >= MAX_CACHED_FONTS {
            map.clear();
        }
        map.insert(name.to_string(), font.clone());
    }
    (font, true)
}

/// The font compiled into the binary.
pub fn built_in() -> FontArc {
    static FONT: OnceLock<FontArc> = OnceLock::new();
    FONT.get_or_init(|| {
        FontArc::try_from_slice(BUILT_IN_BYTES).expect("embedded font must parse")
    })
    .clone()
}

fn cache() -> &'static Mutex<HashMap<String, FontArc>> {
    static CACHE: OnceLock<Mutex<HashMap<String, FontArc>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn read_face(face: &Face) -> Option<FontArc> {
    let bytes = std::fs::read(&face.path).ok()?;
    FontVec::try_from_vec_and_index(bytes, face.index).ok().map(FontArc::from)
}

// ------------------------------------------------------------------ scanning

/// Where the platform keeps fonts. Missing directories are harmless — the walk
/// simply finds nothing, which is also what happens on a headless CI box.
fn font_directories() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    let mut dirs: Vec<PathBuf> = Vec::new();

    if cfg!(windows) {
        let windir =
            std::env::var_os("WINDIR").unwrap_or_else(|| std::ffi::OsString::from("C:\\Windows"));
        dirs.push(PathBuf::from(windir).join("Fonts"));
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            // Per-user installs (Windows 10+ "Install for me only").
            dirs.push(PathBuf::from(local).join("Microsoft/Windows/Fonts"));
        }
    } else if cfg!(target_os = "macos") {
        dirs.push(PathBuf::from("/System/Library/Fonts"));
        dirs.push(PathBuf::from("/Library/Fonts"));
        if let Some(home) = &home {
            dirs.push(PathBuf::from(home).join("Library/Fonts"));
        }
    } else {
        dirs.push(PathBuf::from("/usr/share/fonts"));
        dirs.push(PathBuf::from("/usr/local/share/fonts"));
        if let Some(home) = &home {
            dirs.push(PathBuf::from(home).join(".local/share/fonts"));
            dirs.push(PathBuf::from(home).join(".fonts"));
        }
    }
    dirs
}

/// Walk `roots`, naming every face found. Sorted and de-duplicated by name:
/// the same family is often installed in more than one place, and a dropdown
/// with "Arial" three times is worse than one that quietly keeps the first.
fn collect(roots: &[PathBuf]) -> Vec<Face> {
    let mut faces: Vec<Face> = Vec::new();
    for root in roots {
        walk(root, 0, &mut faces);
    }
    faces.sort_by(|a, b| a.name.cmp(&b.name));
    faces.dedup_by(|a, b| a.name == b.name);
    faces
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<Face>) {
    if depth > MAX_SCAN_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else { continue };
        if kind.is_dir() {
            walk(&path, depth + 1, out);
        } else if has_font_extension(&path) {
            out.extend(faces_in_file(&path));
        }
    }
}

fn has_font_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .is_some_and(|e| FONT_EXTENSIONS.contains(&e.as_str()))
}

/// Name every face in one file. A `.ttc` collection holds several.
fn faces_in_file(path: &Path) -> Vec<Face> {
    let Ok(mut file) = std::fs::File::open(path) else { return Vec::new() };
    let count = collection_count(&mut file).unwrap_or(1).min(64);
    (0..count)
        .filter_map(|index| {
            let name = read_name_table(&mut file, index)
                .and_then(|table| display_name(&table))?;
            Some(Face { name, path: path.to_path_buf(), index })
        })
        .collect()
}

/// Number of fonts in a `ttcf` collection, or `None` for a single-font file.
fn collection_count(file: &mut std::fs::File) -> Option<u32> {
    let mut header = [0u8; 12];
    file.seek(SeekFrom::Start(0)).ok()?;
    file.read_exact(&mut header).ok()?;
    (&header[..4] == b"ttcf").then(|| be32(&header[8..12]))
}

/// Byte offset of font `index`'s table directory. Zero unless this is a
/// collection, whose header is a list of offsets to the real directories.
fn directory_offset(file: &mut std::fs::File, index: u32) -> Option<u64> {
    let Some(count) = collection_count(file) else {
        return (index == 0).then_some(0);
    };
    if index >= count {
        return None;
    }
    let mut offset = [0u8; 4];
    file.seek(SeekFrom::Start(12 + index as u64 * 4)).ok()?;
    file.read_exact(&mut offset).ok()?;
    Some(be32(&offset) as u64)
}

/// The raw `name` table of one face.
///
/// Reads the table directory and then just that table, rather than the whole
/// file — see the module docs for why that matters at several hundred fonts.
fn read_name_table(file: &mut std::fs::File, index: u32) -> Option<Vec<u8>> {
    let base = directory_offset(file, index)?;

    let mut header = [0u8; 12];
    file.seek(SeekFrom::Start(base)).ok()?;
    file.read_exact(&mut header).ok()?;
    let num_tables = be16(&header[4..6]) as usize;
    // A plausibility guard: a corrupt file could claim 65535 tables and send us
    // reading a megabyte of garbage.
    if num_tables == 0 || num_tables > 512 {
        return None;
    }

    let mut directory = vec![0u8; num_tables * 16];
    file.read_exact(&mut directory).ok()?;
    let entry = directory.chunks_exact(16).find(|e| &e[..4] == b"name")?;
    let offset = be32(&entry[8..12]) as u64;
    let length = be32(&entry[12..16]) as usize;
    // Name tables are a few KB; anything claiming more is not one.
    if length == 0 || length > 1 << 20 {
        return None;
    }

    let mut table = vec![0u8; length];
    file.seek(SeekFrom::Start(offset)).ok()?;
    file.read_exact(&mut table).ok()?;
    Some(table)
}

/// The dropdown label for a face: family, plus the subfamily when it is not the
/// plain regular weight — `"Arial"`, `"Arial Bold"`, `"Segoe UI Semilight"`.
///
/// Preferred (typographic) names win where a font supplies them: those are the
/// names a font's designer intends, and they are what avoids the legacy
/// four-style grouping that turns "Semibold" into "Bold".
pub(crate) fn display_name(name_table: &[u8]) -> Option<String> {
    let family = pick_name(name_table, &[TYPOGRAPHIC_FAMILY, FAMILY])?;
    let subfamily = pick_name(name_table, &[TYPOGRAPHIC_SUBFAMILY, SUBFAMILY]);
    Some(match subfamily {
        Some(sub) if !sub.is_empty() && !sub.eq_ignore_ascii_case("regular") => {
            format!("{family} {sub}")
        }
        _ => family,
    })
}

const FAMILY: u16 = 1;
const SUBFAMILY: u16 = 2;
const TYPOGRAPHIC_FAMILY: u16 = 16;
const TYPOGRAPHIC_SUBFAMILY: u16 = 17;

/// First readable string for any of `wanted`, in preference order.
///
/// Hand-rolled rather than routed through a font crate because we hold the
/// `name` table alone, not a whole parsable font file — the point of the
/// targeted read above.
fn pick_name(table: &[u8], wanted: &[u16]) -> Option<String> {
    if table.len() < 6 {
        return None;
    }
    let count = be16(&table[2..4]) as usize;
    let storage = be16(&table[4..6]) as usize;
    let records = table.get(6..6 + count * 12)?;

    for id in wanted {
        for record in records.chunks_exact(12) {
            if be16(&record[6..8]) != *id {
                continue;
            }
            let platform = be16(&record[0..2]);
            let encoding = be16(&record[2..4]);
            let length = be16(&record[8..10]) as usize;
            let offset = be16(&record[10..12]) as usize;
            let Some(bytes) = table.get(storage + offset..storage + offset + length) else {
                continue;
            };
            // Windows (3) and Unicode (0) records are UTF-16BE; Macintosh (1)
            // Roman is close enough to ASCII for a font name.
            let text = match platform {
                3 | 0 => utf16_be(bytes),
                1 if encoding == 0 => Some(bytes.iter().map(|&b| b as char).collect()),
                _ => None,
            };
            if let Some(text) = text {
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    return Some(trimmed);
                }
            }
        }
    }
    None
}

fn utf16_be(bytes: &[u8]) -> Option<String> {
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    let units: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
    String::from_utf16(&units).ok()
}

fn be16(bytes: &[u8]) -> u16 {
    u16::from_be_bytes([bytes[0], bytes[1]])
}

fn be32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

#[cfg(test)]
#[path = "fonts_tests.rs"]
mod tests;
