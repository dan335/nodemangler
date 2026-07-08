# Video processing with NodeMangler

`mangle-video.sh` runs a NodeMangler node graph over **every frame of a video**.
It uses `ffmpeg` to split the video into frames, `mangler_cli` to run the graph
on each frame, and `ffmpeg` again to stitch the processed frames back into a
video — reusing the source's frame rate and audio.

By default it downloads a small sample clip and applies an edge-detect graph, so
you can see it work with a single command:

```bash
./mangle-video.sh
```

That writes `output.mp4` in the current directory.

<p align="center"><em>testsrc frame → edge-detect graph → processed frame</em></p>

## Requirements

- **ffmpeg** (and `ffprobe`, which ships with it) — `brew install ffmpeg`
- **mangler_cli** — the script finds it automatically:
  1. `$MANGLER_CLI` or `--cli <path>`, else
  2. `mangler_cli` on your `PATH`, else
  3. `app/target/{release,debug}/mangler_cli` in this repo, else
  4. it builds it for you (`cargo build --release -p mangler_cli`).
- **curl** — only when the input is a URL.

## Quick start

```bash
# Default: edge-detect the sample clip → output.mp4
./mangle-video.sh

# Your own video, your own output name
./mangle-video.sh -i my_clip.mov -o stylized.mp4

# A different graph
./mangle-video.sh -g graphs/edge_detect.mangle.json -o edges.mp4

# Fast preview: first 20 frames only, and keep the intermediate frames
./mangle-video.sh --limit 20 --keep
```

## Options

| Option | Default | Description |
| --- | --- | --- |
| `-i, --input <path\|url>` | sample clip online | Source video: a local file or an `http(s)` URL. |
| `-g, --graph <file>` | `graphs/edge_detect.mangle.json` | Graph JSON applied to each frame. |
| `-o, --output <file>` | `output.mp4` | Output video path. |
| `--input-node <id>` | `input` | Node whose path input receives each frame. |
| `--input-index <n>` | `0` | Which input slot on that node is the file path. |
| `--output-node <id>` | `fx` | Node whose image output is saved as the frame. |
| `--output-index <n>` | `0` | Which output slot to save. |
| `--fps <rate>` | detected | Force output frame rate (e.g. `25` or `30000/1001`). |
| `--limit <n>` | all | Process only the first N frames. |
| `--keep` | off | Keep the temporary frame directory. |
| `--workdir <dir>` | temp dir | Use a specific scratch directory. |
| `--cli <path>` | auto | Path to the `mangler_cli` binary. |

Run `./mangle-video.sh --help` for the same list.

## How it works

```
 source video
     │  ffmpeg -i source frame_%06d.png        (1) split into frames
     ▼
 src/frame_000001.png, frame_000002.png, …
     │  for each frame:                         (2) run the graph
     │    mangler_cli graph set-input  --node input --input 0 --value path:<frame>
     │    mangler_cli graph show-output --node fx    --save <out.png>
     ▼
 out/frame_000001.png, frame_000002.png, …
     │  ffmpeg -framerate <fps> -i out/frame_*.png   (3) reassemble
     │         -i source -map 0:v -map 1:a?          (    + copy audio)
     ▼
 output.mp4
```

1. **Split.** `ffmpeg` decodes the source into one PNG per frame under a
   temporary `src/` directory. `ffprobe` reads the source's frame rate so it can
   be preserved on the way out.
2. **Process.** For each frame the script works on a *private copy* of your graph
   (your graph file is never modified). It sets the file node's path input to the
   current frame, runs the graph, and saves the chosen node's image output to
   `out/`.
3. **Reassemble.** `ffmpeg` encodes the processed frames back into a video at the
   source frame rate, and maps the source's audio track across if there is one.

### The graph contract

The script needs to know two things about your graph:

- **which node loads the frame** — an *image from file* node
  (`images/input/from_file`). Its path input is set to each frame in turn.
  Default node ID: `input`.
- **which node's output is the result** — the image saved for each frame.
  Default node ID: `fx`, output `0`.

If your graph uses different IDs, pass `--input-node` / `--output-node` (and
`--input-index` / `--output-index` if the slots differ). Node IDs are shown by
`mangler_cli <graph> info`.

### "Same settings"

The output reuses the source **frame rate** (detected via `ffprobe`, overridable
with `--fps`) and its **audio track** (re-encoded to AAC). The video stream is
normalized to **H.264 / yuv420p** for broad playback compatibility. Resolution
follows the frames — it stays the same unless your graph resizes them.

### Why PNG for the intermediate frames?

NodeMangler images are 32-bit float internally, but `mangler_cli --save` picks an
encoder-compatible 8-bit color format automatically (matching the "to file"
node's defaults), so PNG round-trips 1–4 channel frames losslessly and
`ffmpeg` reads it natively — no separate container needed for input vs.
processed frames.

## Using your own graph

Any graph works as long as it has a file-loader node and a node whose output is
an image. You can build one with the GUI, or from the command line. For example,
a posterize + edge blend, or just swap the effect in the bundled graph.

Build the bundled edge-detect graph from scratch to see the pattern:

```bash
CLI=../../app/target/release/mangler_cli   # or wherever your binary is
G=graphs/my.mangle.json

$CLI $G new
$CLI $G add-node --type images/input/from_file        --id input
$CLI $G add-node --type images/filter/edges/edge_detect --id fx
$CLI $G connect  --from input:0 --to fx:0
$CLI $G set-input --node fx --input 1 --value decimal:1.5   # edge intensity

./mangle-video.sh -g $G -o mine.mp4
```

Swap `edge_detect` for any image operation — browse them with
`mangler_cli show-ops --group images` or `mangler_cli show-ops --search <term>`.
Just keep the file loader wired to the effect, and point `--output-node` at
whichever node produces your final image.

`graphs/edge_detect.mangle.json` is the ready-made default (regenerated
automatically if you delete it).

## Troubleshooting

- **`ffmpeg not found`** — install ffmpeg (`brew install ffmpeg`); `ffprobe`
  comes with it.
- **`set-input failed … Is --input-node correct?`** — your graph's file-loader
  node isn't called `input`. Check IDs with `mangler_cli <graph> info` and pass
  `--input-node`.
- **`graph produced no output for frame …`** — a node in the graph errored on
  that frame. Run the graph on one frame directly to see the message:
  `mangler_cli <graph> set-input --node input --input 0 --value path:<frame>.png`
  then `mangler_cli <graph> show-output --node <id> --save /tmp/test.png`.
- **Slow** — the debug binary is used if that's all that's built. A release build
  is much faster: `(cd ../../app && cargo build --release -p mangler_cli)`.
- **Inspect the frames** — add `--keep` (and optionally `--workdir ./frames`) to
  keep the `src/` and `out/` frame directories after the run.
