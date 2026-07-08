#!/usr/bin/env bash
#
# mangle-video.sh — run a NodeMangler graph over every frame of a video.
#
# Pipeline:
#   1. Fetch the source video (local path or URL; defaults to a small clip online).
#   2. ffmpeg splits it into individual frames (one PNG per frame).
#   3. mangler_cli runs the chosen node graph on each frame, saving a processed frame.
#   4. ffmpeg reassembles the processed frames into a video, reusing the source's
#      frame rate and audio track.
#
# Run `./mangle-video.sh --help` for usage.

set -euo pipefail

# ── Locate ourselves and the repo ────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# ── Defaults ─────────────────────────────────────────────────────────────────
# A small (≈1 MB, 10 s, 360p) Big Buck Bunny clip that is fine to download.
DEFAULT_INPUT="https://test-videos.co.uk/vids/bigbuckbunny/mp4/h264/360/Big_Buck_Bunny_360_10s_1MB.mp4"
DEFAULT_GRAPH="$SCRIPT_DIR/graphs/edge_detect.mangle.json"

INPUT="$DEFAULT_INPUT"
GRAPH=""
OUTPUT="output.mp4"
INPUT_NODE="input"          # node whose path input receives each frame
INPUT_INDEX="0"             # which input slot on that node is the file path
OUTPUT_NODE="fx"            # node whose image output is saved
OUTPUT_INDEX="0"            # which output slot to save
FPS_OVERRIDE=""             # force a frame rate instead of detecting it
LIMIT="0"                   # process only the first N frames (0 = all)
KEEP_TEMP="0"               # keep the scratch dir with all frames
CLI_BIN="${MANGLER_CLI:-}"  # path to mangler_cli (env override)
WORKDIR=""                  # explicit scratch dir (default: mktemp)

usage() {
    cat <<EOF
mangle-video.sh — run a NodeMangler graph on every frame of a video.

USAGE:
  ./mangle-video.sh [options]

OPTIONS:
  -i, --input <path|url>    Source video. Local file or http(s) URL.
                            (default: a ~1 MB Big Buck Bunny clip online)
  -g, --graph <file>        NodeMangler graph JSON to apply per frame.
                            (default: graphs/edge_detect.mangle.json)
  -o, --output <file>       Output video path. (default: output.mp4)
      --input-node <id>     Node ID whose path input receives each frame.
                            (default: input)
      --input-index <n>     Input slot on that node that is the file path.
                            (default: 0)
      --output-node <id>    Node ID whose image output is saved. (default: fx)
      --output-index <n>    Output slot to save. (default: 0)
      --fps <rate>          Force output frame rate (e.g. 25 or 30000/1001).
                            (default: detected from the source)
      --limit <n>           Process only the first N frames (quick tests).
      --keep                Keep the temporary frame directory.
      --workdir <dir>       Use this scratch dir instead of a temp one.
      --cli <path>          Path to the mangler_cli binary.
  -h, --help                Show this help.

EXAMPLES:
  # Default: edge-detect the sample clip into output.mp4
  ./mangle-video.sh

  # Your own video and graph
  ./mangle-video.sh -i clip.mov -g graphs/edge_detect.mangle.json -o edges.mp4

  # A custom graph with differently-named input/output nodes
  ./mangle-video.sh -g my.mangle.json --input-node loader --output-node result

  # Fast preview of the first 20 frames, keeping the intermediates
  ./mangle-video.sh --limit 20 --keep

The graph must contain an image-from-file node (its path input is set to each
frame) and a node whose image output becomes the processed frame. Use
--input-node / --output-node to name them if they are not "input" / "fx".
EOF
}

# ── Parse arguments ──────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        -i|--input)        INPUT="$2"; shift 2 ;;
        -g|--graph)        GRAPH="$2"; shift 2 ;;
        -o|--output)       OUTPUT="$2"; shift 2 ;;
        --input-node)      INPUT_NODE="$2"; shift 2 ;;
        --input-index)     INPUT_INDEX="$2"; shift 2 ;;
        --output-node)     OUTPUT_NODE="$2"; shift 2 ;;
        --output-index)    OUTPUT_INDEX="$2"; shift 2 ;;
        --fps)             FPS_OVERRIDE="$2"; shift 2 ;;
        --limit)           LIMIT="$2"; shift 2 ;;
        --keep)            KEEP_TEMP="1"; shift ;;
        --workdir)         WORKDIR="$2"; shift 2 ;;
        --cli)             CLI_BIN="$2"; shift 2 ;;
        -h|--help)         usage; exit 0 ;;
        *) echo "error: unknown option '$1' (try --help)" >&2; exit 2 ;;
    esac
done

die() { echo "error: $*" >&2; exit 1; }

# ── Check external tools ─────────────────────────────────────────────────────
command -v ffmpeg  >/dev/null 2>&1 || die "ffmpeg not found on PATH. Install it (e.g. 'brew install ffmpeg')."
command -v ffprobe >/dev/null 2>&1 || die "ffprobe not found on PATH. It ships with ffmpeg."

# ── Locate the mangler_cli binary ────────────────────────────────────────────
resolve_cli() {
    if [[ -n "$CLI_BIN" ]]; then
        [[ -x "$CLI_BIN" ]] || die "--cli '$CLI_BIN' is not an executable."
        return
    fi
    if command -v mangler_cli >/dev/null 2>&1; then
        CLI_BIN="$(command -v mangler_cli)"; return
    fi
    for candidate in \
        "$REPO_ROOT/app/target/release/mangler_cli" \
        "$REPO_ROOT/app/target/debug/mangler_cli"; do
        if [[ -x "$candidate" ]]; then CLI_BIN="$candidate"; return; fi
    done
    # Nothing prebuilt — build it (release) from the workspace.
    echo "mangler_cli not found; building it (cargo build --release -p mangler_cli)…" >&2
    ( cd "$REPO_ROOT/app" && cargo build --release -p mangler_cli ) \
        || die "failed to build mangler_cli. Build it manually: (cd app && cargo build --release -p mangler_cli)"
    CLI_BIN="$REPO_ROOT/app/target/release/mangler_cli"
    [[ -x "$CLI_BIN" ]] || die "mangler_cli still not found after build."
}
resolve_cli
echo "using mangler_cli: $CLI_BIN"

# ── Resolve the graph (generate the default one if missing) ──────────────────
ensure_default_graph() {
    # Rebuild the bundled edge-detect graph from scratch so it always matches
    # the current graph schema.
    local g="$1"
    mkdir -p "$(dirname "$g")"
    rm -f "$g"
    "$CLI_BIN" "$g" new >/dev/null
    "$CLI_BIN" "$g" add-node --type images/input/from_file --id input >/dev/null
    "$CLI_BIN" "$g" add-node --type images/filter/edges/edge_detect --id fx >/dev/null
    "$CLI_BIN" "$g" connect --from input:0 --to fx:0 >/dev/null
    "$CLI_BIN" "$g" set-input --node fx --input 1 --value decimal:1.5 >/dev/null
}

if [[ -z "$GRAPH" ]]; then
    GRAPH="$DEFAULT_GRAPH"
    if [[ ! -f "$GRAPH" ]]; then
        echo "default graph missing; generating $GRAPH" >&2
        ensure_default_graph "$GRAPH"
    fi
fi
[[ -f "$GRAPH" ]] || die "graph file not found: $GRAPH"
GRAPH="$(cd "$(dirname "$GRAPH")" && pwd)/$(basename "$GRAPH")"   # absolutize

# ── Scratch directory ────────────────────────────────────────────────────────
if [[ -n "$WORKDIR" ]]; then
    mkdir -p "$WORKDIR"
    WORK="$(cd "$WORKDIR" && pwd)"
else
    WORK="$(mktemp -d "${TMPDIR:-/tmp}/mangle-video.XXXXXX")"
fi
SRC_FRAMES="$WORK/src"      # frames extracted from the source
OUT_FRAMES="$WORK/out"      # frames after the graph runs
mkdir -p "$SRC_FRAMES" "$OUT_FRAMES"

cleanup() {
    if [[ "$KEEP_TEMP" == "1" ]]; then
        echo "kept intermediates in: $WORK"
    elif [[ -z "$WORKDIR" ]]; then
        rm -rf "$WORK"
    fi
}
trap cleanup EXIT

# ── Fetch the source video ───────────────────────────────────────────────────
SOURCE="$WORK/source"
if [[ "$INPUT" =~ ^https?:// ]]; then
    ext="${INPUT##*.}"; [[ "$ext" =~ ^[A-Za-z0-9]{2,4}$ ]] || ext="mp4"
    SOURCE="$WORK/source.$ext"
    echo "downloading $INPUT"
    command -v curl >/dev/null 2>&1 || die "curl not found; needed to download a URL."
    curl -fL --retry 3 --output "$SOURCE" "$INPUT" || die "download failed: $INPUT"
else
    [[ -f "$INPUT" ]] || die "input video not found: $INPUT"
    SOURCE="$(cd "$(dirname "$INPUT")" && pwd)/$(basename "$INPUT")"
fi

# ── Probe frame rate ─────────────────────────────────────────────────────────
if [[ -n "$FPS_OVERRIDE" ]]; then
    FPS="$FPS_OVERRIDE"
else
    FPS="$(ffprobe -v error -select_streams v:0 \
        -show_entries stream=r_frame_rate -of default=nw=1:nk=1 "$SOURCE" | head -1)"
    [[ -n "$FPS" && "$FPS" != "0/0" ]] || FPS="25"
fi
echo "frame rate: $FPS"

# ── Extract frames ───────────────────────────────────────────────────────────
echo "extracting frames → $SRC_FRAMES"
ffmpeg -y -loglevel error -i "$SOURCE" -fps_mode passthrough \
    "$SRC_FRAMES/frame_%06d.png"

shopt -s nullglob
frames=( "$SRC_FRAMES"/frame_*.png )
shopt -u nullglob
total="${#frames[@]}"
[[ "$total" -gt 0 ]] || die "ffmpeg produced no frames from $SOURCE"
if [[ "$LIMIT" -gt 0 && "$LIMIT" -lt "$total" ]]; then
    frames=( "${frames[@]:0:$LIMIT}" )
    total="$LIMIT"
fi
echo "processing $total frame(s) through graph: $GRAPH"

# ── Run the graph on each frame ──────────────────────────────────────────────
# Work on a private copy of the graph so we never mutate the user's file.
WORK_GRAPH="$WORK/graph.json"
cp "$GRAPH" "$WORK_GRAPH"

process_frame() {
    # $1 = absolute input frame, $2 = absolute output frame
    "$CLI_BIN" "$WORK_GRAPH" set-input \
        --node "$INPUT_NODE" --input "$INPUT_INDEX" --value "path:$1" >/dev/null 2>"$WORK/err.log" \
        || die "set-input failed on node '$INPUT_NODE' input $INPUT_INDEX. $(cat "$WORK/err.log"). Is --input-node correct?"
    "$CLI_BIN" "$WORK_GRAPH" show-output \
        --node "$OUTPUT_NODE" --output "$OUTPUT_INDEX" --save "$2" >/dev/null 2>"$WORK/err.log" \
        || die "show-output failed on node '$OUTPUT_NODE'. $(cat "$WORK/err.log"). Is --output-node correct?"
    # show-output returns success even when a node errored; verify a frame landed.
    [[ -s "$2" ]] || die "graph produced no output for frame $1 (a node likely errored). $(cat "$WORK/err.log")"
}

i=0
for f in "${frames[@]}"; do
    base="$(basename "${f%.png}")"
    out="$OUT_FRAMES/$base.png"        # lossless, compact, ffmpeg-native
    process_frame "$f" "$out"
    i=$((i + 1))
    printf "\r  frame %d/%d" "$i" "$total"
done
printf "\n"

# ── Reassemble ───────────────────────────────────────────────────────────────
echo "encoding → $OUTPUT"
# Glob the processed frames (numbering is preserved from extraction) and pull
# the audio, if any, straight from the source. Video is normalized to H.264 /
# yuv420p for broad playback; frame rate and audio come from the source.
ffmpeg -y -loglevel error \
    -framerate "$FPS" -pattern_type glob -i "$OUT_FRAMES/frame_*.png" \
    -i "$SOURCE" \
    -map 0:v:0 -map "1:a:0?" \
    -c:v libx264 -pix_fmt yuv420p -r "$FPS" \
    -c:a aac -b:a 192k \
    -shortest \
    "$OUTPUT"

echo "done: $OUTPUT"
