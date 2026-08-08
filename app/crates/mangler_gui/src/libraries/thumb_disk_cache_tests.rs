//! Pure path/key tests for the library thumb disk cache (no global cache dir).

use std::path::Path;

use super::{cache_relative_path, FileFingerprint};

#[test]
fn cache_path_includes_fingerprint_and_shard() {
    let fp = FileFingerprint {
        mtime_ns: 1_700_000_000_000_000_000,
        len: 12_345_678,
    };
    let rel = cache_relative_path(Path::new("/photos/IMG_0001.CR3"), fp, 192);
    let s = rel.to_string_lossy();
    assert!(s.contains("v1_m192"), "version+max segment: {s}");
    assert!(s.contains("1700000000000000000"), "mtime in name: {s}");
    assert!(s.contains("12345678"), "len in name: {s}");
    assert!(s.ends_with(".jpg"));
    // Two path components under the version dir: shard / file
    assert_eq!(rel.components().count(), 3);
}

#[test]
fn different_mtime_different_filename() {
    let a = FileFingerprint {
        mtime_ns: 100,
        len: 50,
    };
    let b = FileFingerprint {
        mtime_ns: 101,
        len: 50,
    };
    let pa = cache_relative_path(Path::new("/a.jpg"), a, 192);
    let pb = cache_relative_path(Path::new("/a.jpg"), b, 192);
    assert_ne!(pa, pb);
}

#[test]
fn different_max_edge_different_dir() {
    let fp = FileFingerprint {
        mtime_ns: 1,
        len: 1,
    };
    let a = cache_relative_path(Path::new("/a.jpg"), fp, 96);
    let b = cache_relative_path(Path::new("/a.jpg"), fp, 192);
    assert_ne!(a.components().next(), b.components().next());
}

#[test]
fn store_and_load_roundtrip_in_temp() {
    // Exercise try_load/store against a real file by pointing cache root via
    // a private path — we call store/load through the public API after writing
    // a source image next to a fingerprinted path.
    use super::{fingerprint, load_or_decode, store, try_load};
    use std::fs;

    let dir = std::env::temp_dir().join(format!(
        "mangler_disk_thumb_{}",
        std::process::id()
    ));
    let _ = fs::create_dir_all(&dir);
    let src = dir.join("src.png");
    let img = image::RgbaImage::from_pixel(32, 24, image::Rgba([10, 20, 30, 255]));
    img.save(&src).unwrap();

    // Force a decode → store → load cycle via load_or_decode.
    let (pixels, size) = load_or_decode(&src, |p| {
        let dyn_img = image::open(p).map_err(|_| ())?;
        let thumb = dyn_img.thumbnail(64, 64);
        let rgba = thumb.to_rgba8();
        Ok((
            rgba.as_raw().clone(),
            [rgba.width() as usize, rgba.height() as usize],
        ))
    })
    .expect("decode+store");

    assert!(!pixels.is_empty());
    assert!(size[0] > 0 && size[1] > 0);

    // Second call must hit disk (we can't observe that directly without a
    // spy, but try_load must succeed and match dimensions).
    let hit = try_load(&src).expect("disk hit after store");
    assert_eq!(hit.1, size);
    assert_eq!(hit.0.len(), pixels.len());

    // Fingerprint must be stable for an untouched file.
    let fp1 = fingerprint(&src).unwrap();
    let fp2 = fingerprint(&src).unwrap();
    assert_eq!(fp1, fp2);

    // store is also callable directly
    store(&src, &pixels, size[0] as u32, size[1] as u32);

    let _ = fs::remove_dir_all(&dir);
}
