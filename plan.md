# Shoot → develop → website: product plan

NodeMangler already covers a solid core of the photo-shoot workflow (`from
folder` + batch/watch, `from raw` develop, a deep adjustments stack, `to file`
export, Libraries thumbs). This plan captures the remaining gaps that still
make “open a card dump, adjust, ship to a website” harder than it should be.

**Done in this round**

- [x] **Raw-aware folder input** — `from folder` now exposes the same develop
  controls as `from raw` (white balance, output encoding, demosaic, exposure,
  max size). Batch and watch apply one recipe to every raw in the folder;
  non-raw formats ignore those inputs. Selection indices
  (`folder` / `index` / `pinned path`) are unchanged for the engine.

---

## Ideal workflow (target)

```text
Card / Downloads folder
        │
        ▼
  Libraries or from folder + filmstrip
        │  cull: pick/reject
        ▼
  from folder (raw develop knobs)  ──same recipe──►  adjustments graph
        │                              ▲
        │                              │ per-file overrides (optional)
        ▼
  resize / web sizes
        ▼
  to file (Web AVIF/JPG, strip GPS, quality preset)
        ▼
  export/ folder → static site / CMS / rsync
```

Optional: **Watch folder** for tethered “develop as I shoot”; **batch** for
“process the picks tonight.”

**Design principle:** stay NodeMangler-shaped.

- Graph = reusable look / pipeline  
- Folder + selection = shoot unit  
- Overrides = sparse deltas  
- Output folder = website input  

Do not become Lightroom; do not own CMS upload unless delivery becomes a
product goal later.

---

## Remaining work (priority order)

### P1 — Must-have for “easier than today”

#### 1. Cull + batch selection

**Problem:** Real shoots are 200 frames → pick 12. Batch runs the whole
folder. No flags/stars/reject, no “export selection.”

**Proposal:**

- Pick / reject (or star) on library thumbs and/or a filmstrip bound to the
  active `from folder` node.
- Batch / export only selected files.
- Optional “skip already exported” by matching source stem in the output
  folder.

**Touches:** Libraries panel state + persistence, batch driver
(`ChangeGraphMessage::RunBatch` / `start_batch` in engine `app.rs`), maybe a
sidecar `.mangler-picks.json` next to the folder so picks survive sessions.

#### 2. Web export preset

**Problem:** `to file` is flexible but not web-oriented. WebP is lossless
only; no privacy EXIF policy; no one-click “web-ready” settings.

**Proposal:**

- Lossy WebP (quality slider shared with JPG/AVIF).
- Optional EXIF strip (keep orientation/copyright, drop GPS).
- Preset dropdown on `to file` (and/or a thin “web export” recipe): e.g.
  “Web JPG”, “Web AVIF” → sRGB, 8-bit, quality ~80, long-edge max, strip GPS.
- Document that a clean `export/web/` folder is the handoff; no S3/CMS required.

**Touches:** `operations/images/outputs/file.rs` (+ shared encoder helpers),
encoder feature matrix tests, README node help.

#### 3. Filmstrip / next-prev over the folder

**Problem:** Stepping `index` on `from folder` works; reviewing a card dump
does not feel like photo software.

**Proposal:**

- Filmstrip or contact strip tied to the active `from folder` (or library
  folder): click frame → set index, show developed preview.
- Keyboard next/prev while the settings panel is focused on that node.
- Optional before/after on the 2D preview (as-shot / raw default vs graph
  result).

**Touches:** GUI settings panel / program, maybe a thin strip widget reusing
library thumbs + decode pool.

---

### P2 — Strong second wave

#### 4. Per-image overrides

**Problem:** Batch is pure: same graph, every file. Shoots need sparse
exceptions (this one +0.5 EV, that one a different crop).

**Proposal (NodeMangler-shaped):**

- Lightweight sidecars (JSON next to each raw, or one table file keyed by
  stem): overrides for EV / crop rect / WB deltas.
- Graph reads them via a small “override” input or a dedicated node that
  merges defaults with the sidecar for the current stem.
- Avoid a full catalog; overrides are deltas on top of one graph.

**Touches:** new op or engine-side override map; crop/exposure inputs already
exist as graph ports.

#### 5. Preview vs export resolution policy

**Problem:** `max size` defaults to 4096 for interactivity. Users forget to
set `0` (or a web long-edge) before a final batch and ship soft or huge files.

**Proposal:**

- Explicit preview / export quality, or “UI always capped; batch always
  full-res then downscale.”
- Or a single web-export stage: long-edge max + format + quality, independent
  of develop proxy size (pairs well with P1.2).

**Touches:** `from folder` / `from raw` max-size semantics, batch driver
flag, or a resize-before-`to file` convention documented as a template graph.

#### 6. Multi-size web outputs

**Problem:** Sites want thumb / content / hero / 2× from one master.

**Proposal:**

- One node (or multi-output `to file` mode): one image → several files
  (`_sm`, `_md`, `_lg` or fixed widths) with shared quality/format.
- Works under batch (each source stem expands to N files).

**Touches:** new op under `images/outputs/` or extension of `to file`.

#### 7. Proxy / cache for scrubbing large shoots

**Problem:** 26 MP f32 chains make scrubbing a card dump painful; batch of
100 raws with a long graph is memory- and CPU-bound.

**Proposal:**

- Disk proxy cache (developed + resized decode keyed by path + mtime +
  develop settings fingerprint), under the app cache dir (same idea as
  library thumbs).
- Per-file skip-on-error in batch (don’t abort the whole run).
- Careful decode parallelism (memory-bound, not “N full raws at once”).

**Touches:** `raw_decode` / folder load path, batch driver error policy,
cache under `dirs::cache_dir()/NodeMangler/…`.

---

### P3 — Nice later

| Item | Why |
|------|-----|
| Soft-proof / sRGB gamut warnings | Catch neon clips before web |
| Profile lens correction | Wide-angle edges / vignetting |
| High-ISO denoise tuned for raw | Dark shots before downscale |
| Highlight recovery UX | Headroom exists; dedicated recover control |
| Caption / alt / title from EXIF or text sidecar | Gallery sites |
| Gallery HTML or upload nodes | Only if NodeMangler should own delivery |

---

## Suggested implementation order

1. ~~Raw-aware `from folder` develop~~ **done**
2. Web export preset (lossy WebP + EXIF strip + max edge / presets)
3. Filmstrip + next/prev (culling UI foundation)
4. Cull flags + batch selection
5. Preview vs export resolution policy
6. Multi-size web outputs
7. Per-image overrides
8. Proxy cache + robust batch error policy
9. P3 items as demand appears

---

## Out of scope (for now)

- Becoming a DAM / Lightroom catalog replacement
- Built-in S3 / CMS / FTP upload (folder handoff is enough)
- Backwards-compatible migration of old graphs when ports change
  (project policy: no migration paths)

---

## Verification notes (when implementing)

- Headless: `mangler_cli` + a folder of fixtures; batch must write one output
  per selected stem.
- Raw develop: same options on `from folder` and `from raw` must produce
  matching pixels for a given path (unit or fixture test under
  `NODEMANGLER_RAW_FIXTURE`).
- Web export: round-trip size/quality asserts; GPS strip checked with a
  fixture that has location EXIF.
- GUI: filmstrip selection updates `index` and does not fight the watch
  driver’s `pinned path`.
