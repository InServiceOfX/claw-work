# VoxCPM TTS for macOS

A local voice-cloning CLI and native Mac app for
[`mlx-community/VoxCPM2-bf16`](https://huggingface.co/mlx-community/VoxCPM2-bf16).
Rust handles reference audio, text chunking, progress, retries, WAV stitching and
cleanup. The native Rust engine uses candle/Metal. An optional Python MLX engine
provides a fallback and numerical reference. See [current verification and remaining
work](docs/HANDOFF.md) before treating a feature as verified.

Requires an Apple Silicon Mac running macOS 15 or later, Rust/Cargo, and enough
unified memory for approximately 5 GB of weights plus inference scratch space.
Rust 1.88 or newer is required; the verified development stack uses Rust 1.91.1. The native float32 path expands
the weights to roughly 10 GB; 24–32 GB or more unified memory is recommended.
The Mac mini M4 has not yet been tested. Weights run locally; model downloads
come from Hugging Face on first use. Provide a clean recording of one speaker
whose voice you are authorized to use.

## Install

From this directory:

```sh
./scripts/install.sh
```

This builds release binaries, installs `~/.local/bin/voxcpm-tts`, and creates an
ad-hoc signed `~/Applications/VoxCPM TTS.app`. Add `~/.local/bin` to your PATH if
needed. Existing selected installation files are protected; use `--force` when
you intend to replace them. No model, virtual environment, or voice sample is
included in the app bundle. It uses the shared Hugging Face cache.

To install just the CLI, or also install the optional Python fallback:

```sh
./scripts/install.sh --cli-only
./scripts/install.sh --with-python-engine --force
```

The fallback requires [`uv`](https://docs.astral.sh/uv/) and uses Python 3.13 with
pinned dependencies from `reference/requirements.txt`. It installs independently
under `~/Library/Application Support/voxcpm-tts/reference`. It is used only when
you select `--engine mlx-python` or choose it in the app. Installation can fetch
Python and packages. `--fetch` optionally downloads model weights immediately;
otherwise synthesis loads or downloads them when needed.

For a reviewable installation entirely within this checkout:

```sh
./scripts/install.sh --prefix "$PWD/dist/local" --app-dir "$PWD/dist/Applications"
./dist/local/bin/voxcpm-tts doctor
open 'dist/Applications/VoxCPM TTS.app'
```

A custom fallback location can use `--support-dir "$PWD/dist/support"`; set
`VOXCPM_TTS_HOME` to that directory when running the CLI or app. The default
support location needs no override. `--offline` prevents Cargo/uv downloads
while staging cached dependencies; it cannot be combined with `--fetch`.
For a disconnected synthesis run, cache the complete model first and set
`HF_HUB_OFFLINE=1`; both engines reject missing model files in offline mode.
`--skip-build --debug` packages existing debug binaries for a quick packaging
check. Release binaries are the default for normal installation.

To build only an app bundle without installing the CLI:

```sh
./scripts/bundle-app.sh
open 'dist/VoxCPM TTS.app'
```

Signing is local and ad hoc, not Apple Developer ID signing or notarization.
Build or run the installer on each Mac rather than copying a Python venv between
machines. The Rust app bundle does not depend on the source checkout for native
inference.

## Read a script in a cloned voice

```sh
voxcpm-tts say /path/to/script.txt --voice /path/to/speaker.wav --output narration.wav
voxcpm-tts say --text 'Welcome to the harbor.' --voice /path/to/speaker.mp3 -o welcome.wav
voxcpm-tts say --edit --voice /path/to/speaker.wav -o edited-script.wav
```

Text files must be UTF-8. `--edit` opens `$VISUAL`, `$EDITOR`, or `nano`.
Reference audio accepts WAV, MP3, FLAC and M4A. Use at least two seconds of one
speaker; five to twenty seconds of clean speech is a good starting point.
References longer than thirty seconds are trimmed. Optionally add an exact
`--transcript 'Words spoken in the sample.'` or `--transcript-file /path/to/transcript.txt`.

Long text splits automatically into small chunks and produces one 48 kHz WAV.
Intermediate parts are removed automatically; `--keep-parts /path/to/parts`
explicitly retains a copy. Existing output needs `--force` to overwrite it.
The default output is 16-bit PCM; `--float` selects 32-bit float WAV.
`--report /path/to/report.json` records the job result. Reports may include
local paths or script details: keep your own outputs outside the public source
tree. Generated WAVs, weights, caches and `dist/` are ignored by Git.

Plan chunks and inspect an approximate duration without loading the model:

```sh
voxcpm-tts plan /path/to/script.txt --verbose
voxcpm-tts doctor
voxcpm-tts fetch
```

Progress reports chunk count, generated audio and remaining time. Estimates
improve after completed runs calibrate the engine. A first native run may spend
time compiling Metal kernels; allow extra time for that cold start. The measured
Python fallback on the development M5 took about 3.7 seconds per second of speech
at seven steps; performance depends on load and engine settings. See
[reference evidence](docs/REFERENCE_ENGINE.md) for the measured conditions.

Choose an engine or quality settings explicitly:

```sh
voxcpm-tts say script.txt --voice speaker.wav --engine native -o native.wav
voxcpm-tts say script.txt --voice speaker.wav --engine mlx-python --timesteps 7 -o mlx.wav
```

The defaults are ten diffusion steps and guidance `--cfg 2.0`. `--max-chars 300`
controls the application chunk size; shorter chunks can help unstable passages.
This character budget is not an exact tokenizer budget for every language.
`--seed 42` makes the initial random seed repeatable; retries use new seeds.

## Keep weights loaded for several scripts

```sh
voxcpm-tts session --voice /path/to/speaker.wav --out-dir /path/to/narrations
```

Use the session's help for file input and commands. `:edit` opens an editor,
`:unload` releases the model, and `:quit` or Ctrl-D unloads and exits. Single
`say` commands release weights when they finish. Ctrl-C cancels a job and runs
cleanup. The Python fallback child is terminated on cancellation or close so
its model cannot remain resident.

The Mac app uses the same library for generation. Open or paste a script,
select a reference recording and destination, then generate. Its memory controls
and engine options are described by the current UI and handoff. Closing the app
ends its inference process; `Free memory` unloads between jobs.

## Development and evidence

```sh
cargo test
cargo clippy --all-targets --features gui -- -D warnings
reference/.venv/bin/python -m unittest discover -s reference -p test_worker.py -v
python3 scripts/test_packaging.py -v
```

Packaging tests use existing debug CLI/app binaries and temporary destinations.
They verify overwrite refusal, explicit replacement, paths containing spaces,
metadata, signature and a bundle containing only the executable/metadata/signature.
They never write to the user's installation directories.

Native numerical tests use cached weights and local golden tensors and may be
ignored by default. [Porting specification](docs/PORTING_SPEC.md) documents the
comparison points. [Handoff](docs/HANDOFF.md), [reference engine](docs/REFERENCE_ENGINE.md),
and [packaging evidence](docs/PACKAGING.md) record completed checks and outstanding
limits. Product intent, prompts and independent user stories preserve intended
behavior; numerical evidence does not replace an audible voice-quality review.
