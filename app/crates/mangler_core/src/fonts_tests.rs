//! Tests for installed-font discovery.
//!
//! Which fonts a machine has is not something a test can control, so the
//! assertions split in two: the name-table parsing is exercised against tables
//! built by hand, and the scan itself is only checked for the properties that
//! must hold on *any* machine, including a bare CI container with no fonts at
//! all.

use super::*;

// ---------------------------------------------------------- name-table parsing

/// Build a minimal `name` table holding the given `(name_id, string)` records
/// as Windows-platform UTF-16BE, the encoding essentially every font uses.
fn name_table(records: &[(u16, &str)]) -> Vec<u8> {
    let mut storage: Vec<u8> = Vec::new();
    let mut entries: Vec<u8> = Vec::new();
    for (id, text) in records {
        let utf16: Vec<u8> =
            text.encode_utf16().flat_map(|u| u.to_be_bytes()).collect();
        entries.extend_from_slice(&3u16.to_be_bytes()); // platform: Windows
        entries.extend_from_slice(&1u16.to_be_bytes()); // encoding: UCS-2
        entries.extend_from_slice(&0x0409u16.to_be_bytes()); // language: en-US
        entries.extend_from_slice(&id.to_be_bytes());
        entries.extend_from_slice(&(utf16.len() as u16).to_be_bytes());
        entries.extend_from_slice(&(storage.len() as u16).to_be_bytes());
        storage.extend_from_slice(&utf16);
    }

    let mut table: Vec<u8> = Vec::new();
    table.extend_from_slice(&0u16.to_be_bytes()); // format 0
    table.extend_from_slice(&(records.len() as u16).to_be_bytes());
    table.extend_from_slice(&(6 + entries.len() as u16).to_be_bytes()); // storage offset
    table.extend_from_slice(&entries);
    table.extend_from_slice(&storage);
    table
}

#[test]
fn a_regular_face_is_named_by_its_family_alone() {
    // "Arial Regular" would be noise in a dropdown; the plain weight is the
    // one whose subfamily is dropped.
    let table = name_table(&[(FAMILY, "Arial"), (SUBFAMILY, "Regular")]);
    assert_eq!(display_name(&table).as_deref(), Some("Arial"));
}

#[test]
fn other_weights_keep_their_subfamily() {
    for sub in ["Bold", "Italic", "Bold Italic", "Semilight"] {
        let table = name_table(&[(FAMILY, "Segoe UI"), (SUBFAMILY, sub)]);
        assert_eq!(display_name(&table).as_deref(), Some(&*format!("Segoe UI {sub}")));
    }
}

#[test]
fn regular_is_matched_case_insensitively() {
    // Real fonts ship "regular", "Regular" and "REGULAR".
    for sub in ["regular", "Regular", "REGULAR"] {
        let table = name_table(&[(FAMILY, "Verdana"), (SUBFAMILY, sub)]);
        assert_eq!(display_name(&table).as_deref(), Some("Verdana"));
    }
}

#[test]
fn typographic_names_win_over_the_legacy_pair() {
    // The legacy pair squeezes every face into four styles, so a Semibold ships
    // as family "Open Sans Semibold" / subfamily "Regular". The typographic
    // names are what the designer actually meant.
    let table = name_table(&[
        (FAMILY, "Open Sans Semibold"),
        (SUBFAMILY, "Regular"),
        (TYPOGRAPHIC_FAMILY, "Open Sans"),
        (TYPOGRAPHIC_SUBFAMILY, "Semibold"),
    ]);
    assert_eq!(display_name(&table).as_deref(), Some("Open Sans Semibold"));
}

#[test]
fn the_legacy_pair_is_used_when_there_is_nothing_better() {
    let table = name_table(&[(FAMILY, "Courier New"), (SUBFAMILY, "Bold")]);
    assert_eq!(display_name(&table).as_deref(), Some("Courier New Bold"));
}

#[test]
fn a_missing_subfamily_leaves_the_family_alone() {
    let table = name_table(&[(FAMILY, "Impact")]);
    assert_eq!(display_name(&table).as_deref(), Some("Impact"));
}

#[test]
fn a_table_without_a_family_yields_no_name() {
    // Better than inventing one: an unnamed entry in the dropdown is unusable.
    let table = name_table(&[(SUBFAMILY, "Bold")]);
    assert_eq!(display_name(&table), None);
}

#[test]
fn names_are_trimmed() {
    let table = name_table(&[(FAMILY, "  Tahoma  "), (SUBFAMILY, " Bold ")]);
    assert_eq!(display_name(&table).as_deref(), Some("Tahoma Bold"));
}

#[test]
fn malformed_tables_are_rejected_rather_than_panicking() {
    // These arrive from files on disk, so every length in them is untrusted.
    assert_eq!(display_name(&[]), None);
    assert_eq!(display_name(&[0, 0, 0]), None);
    // Claims one record but supplies no record bytes.
    assert_eq!(display_name(&[0, 0, 0, 1, 0, 6]), None);
    // Record whose string offset points past the end of the table.
    let mut table = name_table(&[(FAMILY, "Arial")]);
    let len = table.len();
    table[6 + 10] = 0xFF;
    table[6 + 11] = 0xFF;
    assert_eq!(display_name(&table), None, "table len {len}");
}

// -------------------------------------------------------------- the built-in

#[test]
fn the_built_in_font_always_loads() {
    // It is the fallback for every failure path, so it must never be one.
    let (_, honoured) = load(BUILT_IN);
    assert!(honoured);
    let (_, honoured) = load("");
    assert!(honoured, "an empty selection means the default, not a failure");
}

#[test]
fn an_unknown_family_falls_back_without_failing() {
    // A graph moved to a machine lacking its font must still render.
    let (_, honoured) = load("Definitely Not An Installed Font 12345");
    assert!(!honoured, "the caller should be able to tell it did not get what it asked for");
}

#[test]
fn the_built_in_is_the_first_option() {
    // It leads because it is the default and the one entry that exists
    // everywhere — a machine with no fonts installed still gets a usable list.
    let options = options();
    assert_eq!(options.first().map(String::as_str), Some(BUILT_IN));
    assert!(!options.is_empty());
}

// ------------------------------------------------------------------ the scan

#[test]
fn scanning_produces_sorted_unique_names() {
    // Duplicates are the norm — the same family is often installed system-wide
    // and per-user — and a dropdown listing "Arial" twice is a bug the user
    // sees. Holds trivially on a machine with no fonts.
    let faces = installed();
    let names: Vec<&str> = faces.iter().map(|f| f.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted, "faces should come back sorted");
    let mut unique = sorted.clone();
    unique.dedup();
    assert_eq!(unique.len(), names.len(), "names should be unique");
    assert!(faces.iter().all(|f| !f.name.is_empty()), "no unnamed entries");
}

#[test]
fn the_scan_is_cached_after_the_first_call() {
    // `create_inputs()` calls `options()`, so this runs on every node creation
    // and every node of every loaded graph. Re-walking the filesystem there
    // would be a visible hitch each time.
    let first = std::time::Instant::now();
    let _ = installed();
    let cold = first.elapsed();

    let second = std::time::Instant::now();
    for _ in 0..100 {
        let _ = installed();
    }
    let hot = second.elapsed();
    assert!(
        hot < std::time::Duration::from_millis(10),
        "100 cached scans took {hot:?} (first was {cold:?})"
    );
}

#[test]
fn a_scan_of_nothing_is_empty_rather_than_an_error() {
    // Headless CI, a stripped container, a Windows box with a redirected
    // WINDIR: all must degrade to "only the built-in", never a panic.
    assert!(collect(&[]).is_empty());
    assert!(collect(&[std::path::PathBuf::from("/definitely/not/a/font/dir")]).is_empty());
}

#[test]
fn only_rasterisable_extensions_are_offered() {
    // Listing a .pfb would put an entry in the dropdown that silently falls
    // back to the built-in, which is worse than not listing it.
    assert!(has_font_extension(std::path::Path::new("x/Arial.ttf")));
    assert!(has_font_extension(std::path::Path::new("x/Arial.TTF")), "case-insensitive");
    assert!(has_font_extension(std::path::Path::new("x/Cambria.ttc")));
    assert!(has_font_extension(std::path::Path::new("x/Font.otf")));
    assert!(!has_font_extension(std::path::Path::new("x/Old.pfb")));
    assert!(!has_font_extension(std::path::Path::new("x/Bitmap.fon")));
    assert!(!has_font_extension(std::path::Path::new("x/readme.txt")));
    assert!(!has_font_extension(std::path::Path::new("x/noextension")));
}

#[test]
fn every_discovered_face_actually_loads() {
    // The scan reads only the name table, so a file could name itself and then
    // fail to parse as a font. Checking a handful keeps the test quick while
    // still catching a scan that offers junk.
    for face in installed().iter().take(5) {
        let (_, honoured) = load(&face.name);
        assert!(honoured, "{} ({}) was listed but would not load", face.name, face.path.display());
    }
}

#[test]
fn the_cold_scan_does_not_read_whole_font_files() {
    // A smoke test for the one design decision that makes calling this from
    // `create_inputs()` acceptable at all. Reading each file in full to get one
    // string costs hundreds of milliseconds across a few hundred faces (a CJK
    // font alone is tens of megabytes); reading just the table directory and
    // the `name` table measured 232 faces in 7ms on the development machine.
    //
    // The bound is deliberately loose — this is disk I/O on unknown hardware,
    // so it is here to catch a regression to full-file reads, not to police
    // milliseconds.
    let started = std::time::Instant::now();
    let faces = collect(&font_directories());
    let elapsed = started.elapsed();
    assert!(
        elapsed < std::time::Duration::from_millis(1500),
        "scanning {} faces took {elapsed:?} — has the targeted name-table read been lost?",
        faces.len()
    );
}
