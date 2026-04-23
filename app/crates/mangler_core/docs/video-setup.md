# Video feature setup

The `video` cargo feature on `mangler_core` enables the video input operation
(and, in later phases, video rendering). It pulls in
[`video-rs`](https://crates.io/crates/video-rs), which wraps the FFmpeg
libraries via
[`ffmpeg-next`](https://github.com/zmwangx/rust-ffmpeg). Building against those
crates needs **FFmpeg development libraries** on the system — not just the
`ffmpeg.exe` CLI.

## Quick check

```bash
cargo build -p mangler_core --features video
```

If this fails with "Could not find ffmpeg with vcpkg" / "pkg-config command
could not be found", follow one of the options below.

## Windows (recommended): vcpkg

```powershell
git clone https://github.com/Microsoft/vcpkg.git C:\vcpkg
C:\vcpkg\bootstrap-vcpkg.bat
C:\vcpkg\vcpkg.exe install ffmpeg:x64-windows
setx VCPKG_ROOT "C:\vcpkg"
setx PATH "%PATH%;C:\vcpkg\installed\x64-windows\bin"
```

Both env vars are persistent; restart your shell for them to take effect.
`VCPKG_ROOT` is how `ffmpeg-sys-next`'s build script locates the dev libs at
compile time. The `PATH` entry is how the avcodec-*/avformat-*/avutil-* DLLs
load at runtime.

## Windows (alternative): prebuilt shared-library pack

1. Download a `*-win64-gpl-shared-*.zip` from
   <https://github.com/BtbN/FFmpeg-Builds/releases>.
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
