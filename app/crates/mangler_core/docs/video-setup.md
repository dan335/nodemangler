# Video feature setup

The `video` cargo feature on `mangler_core` enables the video input operation
(and, in later phases, video rendering). It pulls in
[`video-rs`](https://crates.io/crates/video-rs), which wraps the FFmpeg
libraries via
[`ffmpeg-next`](https://github.com/zmwangx/rust-ffmpeg). Building against those
crates needs **FFmpeg development libraries** on the system — not just the
`ffmpeg.exe` CLI.

## Licensing (read before distributing builds)

FFmpeg core is LGPL, but the encoders you need for real output are not:

- **`libx264` (H.264) and `libx265` (H.265) are GPL.** Linking them requires
  building FFmpeg with `--enable-gpl`, which makes that FFmpeg GPL-licensed as a
  whole. These are exactly the encoders the setup steps above enable.
- **`libvpx` (VP8/VP9) and `libaom` (AV1)** are permissively licensed (BSD /
  Alliance for Open Media), so on their own they impose no copyleft.

What this means in practice:

- **Building locally for yourself** — no obligations; do whatever you like.
- **Distributing a binary** of `mangler_gui` / `mangler_cli` linked against a GPL
  FFmpeg (one built with x264/x265 + `--enable-gpl`) makes the *combined
  distributed work* subject to the GPL: you must license the whole under
  GPL-compatible terms and offer the corresponding source. A permissive project
  license (MIT/Apache) does **not** override this for that binary.
- **To distribute without copyleft**, build with the `video` feature disabled, or
  link an LGPL-only FFmpeg (no `libx264`/`libx265`, no `--enable-gpl`) — you keep
  decode and VP8/VP9/AV1 encode but lose x264/x265 encode.

The `video` feature is **off by default** in `mangler_core` (`default = []`);
`mangler_gui` and `mangler_cli` opt in via their own `Cargo.toml`, so a default
`cargo build` of the core has no FFmpeg/GPL footprint at all.

### Attribution

- FFmpeg — <https://ffmpeg.org> (LGPL-2.1+, or GPL-2.0+ with `--enable-gpl`)
- x264 — <https://www.videolan.org/developers/x264.html> (GPL-2.0+)
- x265 — <https://www.x265.org> (GPL-2.0+)
- libvpx — <https://www.webmproject.org/code> (BSD-3-Clause)
- libaom — <https://aomedia.googlesource.com/aom> (BSD-2-Clause + AOM Patent License)

## Quick check

```bash
cargo build -p mangler_core --features video
```

If this fails with "Could not find ffmpeg with vcpkg" / "pkg-config command
could not be found", follow one of the options below.

## Encode vs. decode

FFmpeg's built-in H.264 support is **decode-only**. To encode H.264 (i.e.
render video files out of mangler) you need one of:

- `libx264` — the software H.264 encoder. GPL-licensed; **not** in vcpkg's
  default `ffmpeg` port.
- A hardware encoder: `h264_nvenc` (NVIDIA), `h264_amf` (AMD), `h264_qsv`
  (Intel QuickSync), `h264_mf` (Windows Media Foundation). Each requires its
  own `--enable-*` flag when FFmpeg is built.

Without one of those, `VideoEncoder::open` fails with
`"video-rs: Invalid argument"`. This is not a mangler bug — the ffmpeg build
you linked against has no encoder registered for H.264.

## Windows (recommended): vcpkg

**Important:** vcpkg's default `ffmpeg` port is built with `--disable-libx264`
and no hardware H.264 encoders, so you get decode but no encode. Install with
the `x264` feature (which pulls in GPL code) so an encoder is actually
registered:

```powershell
git clone https://github.com/Microsoft/vcpkg.git C:\vcpkg
C:\vcpkg\bootstrap-vcpkg.bat
C:\vcpkg\vcpkg.exe install "ffmpeg[x264,gpl]:x64-windows" --recurse
setx VCPKG_ROOT "C:\vcpkg"
setx PATH "%PATH%;C:\vcpkg\installed\x64-windows\bin"
```

`--recurse` is required because `x264` / `gpl` are not in the default feature
set and vcpkg refuses to quietly re-plan without it. Both env vars are
persistent; restart your shell for them to take effect. `VCPKG_ROOT` is how
`ffmpeg-sys-next`'s build script locates the dev libs at compile time. The
`PATH` entry is how the avcodec-*/avformat-*/avutil-* DLLs load at runtime.

If you already installed `ffmpeg:x64-windows` without x264 and want to
switch:

```powershell
C:\vcpkg\vcpkg.exe remove ffmpeg:x64-windows
C:\vcpkg\vcpkg.exe install "ffmpeg[x264,gpl]:x64-windows" --recurse
```

Then rebuild the ffmpeg-sys-next bindings:

```powershell
cargo clean -p ffmpeg-sys-next
cargo build -p mangler_gui
```

## Windows (alternative): prebuilt shared-library pack

1. Download a `*-win64-gpl-shared-*.zip` from
   <https://github.com/BtbN/FFmpeg-Builds/releases> — the `-gpl-` variants
   include `libx264`, `libx265`, and hardware encoders.
2. Extract to e.g. `C:\ffmpeg`.
3. Set `FFMPEG_DIR=C:\ffmpeg` (the folder that contains `bin`, `lib`, `include`).
4. Add `C:\ffmpeg\bin` to `PATH` so the DLLs load at runtime.

## macOS

```bash
brew install ffmpeg pkg-config
```

## Linux (Debian/Ubuntu)

```bash
sudo apt install libavcodec-dev libavformat-dev libavutil-dev \
                 libavfilter-dev libavdevice-dev libswscale-dev \
                 libswresample-dev pkg-config
```

## Running with the feature

```bash
cargo run -p mangler_gui --features mangler_core/video
```

Or enable by default for development by adding to `~/.cargo/config.toml`:

```toml
[build]
rustflags = []

# (there is no direct "default features" knob; re-run cargo with
# --features each time, or set it via a workspace default if desired)
```

## Why the gate?

FFmpeg isn't a Rust dependency — it's a C/C++ library, and its build/link is
platform-specific. Gating `video` behind an opt-in feature keeps the default
workspace build frictionless for contributors who don't need video decoding.
