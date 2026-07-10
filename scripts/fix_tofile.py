#!/usr/bin/env python3
"""
Repair NodeMangler "to file" (OpImageOutputFile) nodes broken by the v1.0.4
naming/saving overhaul (commit fd3ceaa).

Old schema inputs:  [image, file name, folder, image format, quality,
                     color format, png compression]
New schema inputs:  [image, file path, quality, color format, png compression]

The overhaul replaced folder+name+format with a single "file path" input whose
extension picks the format. Old graphs load with the old inputs verbatim, so the
new run() reads a *folder* (no extension) as the file path and refuses to save.

This script rewrites each old "to file" node in place: it joins folder + file
name + (extension derived from the old image-format enum) into one path and
rebuilds the input list in the new order. image / quality / color format /
png compression carry over unchanged (their names + settings already match).

Usage:
    python fix_tofile.py <folder-or-file> [--dry-run]

Recurses into subfolders for *.mangler.json. Writes a .bak beside each file it
changes (skips if a .bak already exists). Idempotent: already-migrated nodes
are left alone.
"""
import json
import sys
import os
import uuid
import glob

# image::ImageFormat serde variant name -> canonical extension used by
# image_format_from_path (must be one the loader recognizes).
EXT_MAP = {
    "Png": "png", "Jpeg": "jpg", "Gif": "gif", "WebP": "webp",
    "Pnm": "pbm", "Tiff": "tiff", "Tga": "tga", "Bmp": "bmp",
    "Ico": "ico", "Hdr": "hdr", "OpenExr": "exr", "Farbfeld": "ff",
    "Avif": "avif", "Qoi": "qoi",
}


def new_input(name, description, value, settings):
    return {
        "id": uuid.uuid4().hex[:21],
        "name": name,
        "description": description,
        "value": value,
        "settings": settings,
        "connection": None,
        "is_exposed": False,
        "accepts_any_type": False,
    }


FILE_PATH_SETTINGS = {
    "Path": {
        "extension_filter": ["png", "jpg", "gif", "webp", "pbm", "tiff", "tga",
                             "bmp", "ico", "hdr", "exr", "ff", "qoi", "jxl", "psd"],
        "set_directory": None,
        "set_file_name": None,
        "set_title": "save image",
        "file_dialog_type": "SaveFile",
    }
}


def val_of(inp):
    """Return the single-key value dict's inner value, e.g. {'Path': 'x'} -> 'x'."""
    v = inp.get("value")
    if isinstance(v, dict) and len(v) == 1:
        return next(iter(v.values()))
    return None


def join_path(folder, filename, ext):
    folder = (folder or "").rstrip("/\\")
    base = f"{filename}.{ext}"
    if not folder:
        return base
    return f"{folder}/{base}"


def migrate_node(node):
    """Return True if the node was changed."""
    op = node.get("node_type", {})
    if not isinstance(op, dict):
        return False
    op = op.get("Operation", {})
    if not isinstance(op, dict) or op.get("operation") != "OpImageOutputFile":
        return False

    inputs = node.get("inputs", [])
    by_name = {i.get("name"): i for i in inputs}

    # Already new schema? Nothing to do.
    if "file path" in by_name and "folder" not in by_name and "file name" not in by_name:
        return False

    # Old schema markers.
    if "folder" not in by_name and "file name" not in by_name and "image format" not in by_name:
        return False  # unrecognized shape; leave it alone

    folder = val_of(by_name.get("folder", {})) or ""
    filename = val_of(by_name.get("file name", {})) or "image"
    imgfmt = val_of(by_name.get("image format", {})) or "Png"
    ext = EXT_MAP.get(imgfmt, "png")
    full = join_path(folder, filename, ext)

    image_in = by_name.get("image") or new_input(
        "image", "Image to encode and save to disk.",
        {"Image": None}, None)

    quality_in = by_name.get("quality") or new_input(
        "quality",
        "Lossy compression quality from 1 (smallest) to 100 (best); applies to JPEG and AVIF.",
        {"Integer": 85},
        {"Slider": {"range": [1.0, 100.0], "step_by": 1.0, "clamp_to_range": True}})

    color_in = by_name.get("color format") or new_input(
        "color format",
        "Pixel encoding (bit depth and channel layout) used to write the file.",
        {"ColorFormat": "Rgb8"}, None)

    png_in = by_name.get("png compression") or new_input(
        "png compression",
        "PNG compression effort (lossless; affects file size and encode time only). Ignored for other formats.",
        {"Text": "fast"},
        {"Dropdown": {"options": ["fast", "default", "best", "uncompressed"]}})

    file_path_in = new_input(
        "file path",
        "Full destination path for the saved file; its extension selects the image format.",
        {"Path": full},
        FILE_PATH_SETTINGS)

    node["inputs"] = [image_in, file_path_in, quality_in, color_in, png_in]
    return True


def process_file(path, dry_run):
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
    nodes = data.get("nodes", {})
    node_iter = nodes.values() if isinstance(nodes, dict) else nodes
    changed = []
    for node in node_iter:
        if migrate_node(node):
            changed.append(node.get("id", "?"))
    if not changed:
        return None
    if not dry_run:
        bak = path + ".bak"
        if not os.path.exists(bak):
            os.replace(path, bak)  # move original aside
            with open(bak, "r", encoding="utf-8") as f:
                orig = f.read()
            with open(path, "w", encoding="utf-8") as f:
                json.dump(data, f, indent=1)
        else:
            with open(path, "w", encoding="utf-8") as f:
                json.dump(data, f, indent=1)
    return changed


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    dry_run = "--dry-run" in sys.argv
    if not args:
        print(__doc__)
        sys.exit(1)
    target = args[0]
    if os.path.isdir(target):
        files = glob.glob(os.path.join(target, "**", "*.mangler.json"), recursive=True)
    else:
        files = [target]
    total = 0
    for path in sorted(files):
        try:
            changed = process_file(path, dry_run)
        except Exception as e:
            print(f"!! {path}: {e}")
            continue
        if changed:
            total += len(changed)
            tag = "[dry-run] would fix" if dry_run else "fixed"
            print(f"{tag} {len(changed)} node(s) in {path}: {', '.join(changed)}")
    print(f"\n{'Would migrate' if dry_run else 'Migrated'} {total} 'to file' node(s) across {len(files)} file(s).")


if __name__ == "__main__":
    main()
