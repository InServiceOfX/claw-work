# MLX Python fallback and numerical reference

The Rust CLI and app supervise `reference/voxcpm_worker.py` as a child process.
The application owns chunking, retries, stitching and cancellation. Python is an
optional fallback for the MLX implementation and a numerical reference for the
Rust port. Normal inference does not send the script or voice to a service.
The first model fetch needs network access; cached runs can set
`HF_HUB_OFFLINE=1`.

## Reproducible environment

Verified on Apple M5, macOS 26.2, Python 3.13.9, `mlx-audio==0.6.4`,
`mlx==0.32.4`, `mlx-metal==0.32.4`. `reference/requirements.txt` records the
installed dependency closure. Model snapshot:
`mlx-community/VoxCPM2-bf16@a7fb80d9056983135a54ca3f71fcc5a637ccddf5`.
Weights are 4.955 GB in memory before inference scratch allocations.

```sh
uv venv --python 3.13 reference/.venv
uv pip install --python reference/.venv/bin/python -r reference/requirements.txt
reference/.venv/bin/python -m unittest discover -s reference -p test_worker.py -v
```

macOS sandbox sessions can hide the Metal GPU; an import error saying
`No Metal device available` is an environment access failure. It does not show
that the cached dependencies or model are broken.

The [model card](https://huggingface.co/mlx-community/VoxCPM2-bf16) describes the
48 kHz model. The installed implementation is the reference for the protocol
adapter. See `PORTING_SPEC.md` for discrepancies with OpenBMB/PyTorch semantics.
MLX defaults to reduced precision for some float32 matrix operations. Set
`MLX_ENABLE_TF32=0` **before Python starts** for strict float32 golden comparisons;
see [MLX precision documentation](https://ml-explore.github.io/mlx/build/html/usage/precision.html).
The benchmark below used the default (environment variable unset).

## Protocol

Each stdin line is one JSON object. stdout contains only JSON protocol lines;
Python and native-library logging is redirected to stderr. IDs are echoed.
Startup emits `{"event":"ready","pid":123}` without loading weights.

| Operation | Inputs | Terminal reply |
| --- | --- | --- |
| `load` | `model` (slug or local snapshot), optional `cache_limit_gb` | `ok`, `sample_rate`, `load_seconds`, memory metrics |
| `synthesize` | `text`, `out_wav`, optional `ref_audio`, `ref_text`, `seed`, `cfg_value`, `inference_timesteps`, `max_audio_seconds`, `max_tokens` | `ok`, duration, elapsed time, patch count, attempts, memory metrics |
| `unload` | none | `was_loaded`, memory metrics |
| `ping` | none | `loaded`, memory metrics |
| `shutdown` | none | `ok`, then clean process exit |

EOF exits cleanly too. Invalid requests emit `ok:false`, `code:"request_error"`
and a descriptive `error`; the worker remains available. Generation that reaches
its cap or contains empty/non-finite samples emits `code:"bad_generation"` and
writes no new output. Rust can retry this failure with a new seed. Cancellation
is implemented by the Rust supervisor terminating the worker, which releases all
weights even when synthesis is busy.

During synthesis, every 25 patches a progress event reports `audio_seconds`,
`patches`, `max_patches`, and `elapsed_seconds`. One patch is 0.16 seconds.
`max_audio_seconds` bounds generation by whole patches; `max_tokens` is a patch
limit, not a text-token limit. The PyTorch `6 * text_tokens + 10` guard also
bounds each request. Rust disables the worker's internal retry loop and owns
chunk retries. Direct protocol clients can opt into `retry_badcase:true` and
set `retry_badcase_max_times`; exhausted retries fail explicitly. Seeded retries
use successive seeds.

Reference-only cloning uses `ref_audio`. With an exact `ref_text`, the worker
uses combined reference and continuation inputs; the installed model's plain
`ref_text` argument is ignored, so the adapter supplies `prompt_audio` and
`prompt_text` explicitly. A separator is inserted between Latin-script runs.
The worker removes the eight surplus tail samples introduced by the installed
MLX VAE decoder, producing exactly 7680 samples per generated patch. It writes
mono IEEE float32 WAV atomically. The raw golden-dump tool remains separate.

The MLX freed-buffer cache is bounded at 2 GB and cleared after each request.
`unload` drops the model, collects cycles and clears the cache. The supervisor
then shuts down the child, providing the strongest memory-release boundary.

## Verification, 2026-11-07

Twelve offline tests pass. Initial tests reproduced ignored `max_audio_seconds`,
surplus output samples, and successful replies after exhausted runaway attempts.
The suite now checks those cases, invalid caps, invalid JSON recovery, EOF,
float WAV output, restoration of the sampler after exceptions, non-finite audio
rejection, progress counting only completed patches, and the typed generation-failure protocol. These tests use a mock
model and run without GPU access.

A separate real-model benchmark used the cached public `test_en.wav` voice,
synthetic lighthouse prose from `bench_worker.py`, seed 42, guidance 2.0,
seven diffusion steps, and a warm-up call. No private voice was used.

| Text | Tokens | Output | Wall time | Wall/audio ratio | Patches |
| --- | --- | --- | --- | --- | --- |
| 101 characters / 21 words | 24 | 5.12 s | 19.005 s | 3.712 | 32 |
| 293 characters / 56 words | 67 | 14.24 s | 52.180 s | 3.664 | 89 |

Both outputs were valid mono 48 kHz float32 WAVs, stopped before the cap and
required no retry. Four progress events were observed. The 293-character result
had a longest internal quiet gap of 0.14 s, with 0.02 s trailing silence.
These metrics establish plausible audio output, not perceived voice similarity
or pronunciation quality. Human listening and ASR were not part of this run.
Around 300 English characters fit the recommended 80-token budget in this
sample; that is not a bound for every language or input.

Load took 2.669 s. Peak MLX allocation was 8.988 GB. Process footprint peaked
at 9302.6 MiB; immediately after unload it was 1713.4 MiB, then 155.4 MiB two
seconds later. MLX active and cache allocations were both zero after unload.
Shutdown exited with status zero. Whole-system free-memory percentages are
noisy when other applications run and are not used as release evidence.

To reproduce (from the crate root, supply the cached public sample path):

```sh
HF_HUB_OFFLINE=1 reference/.venv/bin/python reference/bench_worker.py \
  --ref /path/to/public/test_en.wav --modes controllable \
  --timesteps 7 --lengths 100 300 --out-dir /tmp/voxcpm-reference-check
```

The benchmark supervisor terminates its worker in a `finally` block on errors.
Use `--modes ultimate --ref-text 'exact sample transcript'` to measure combined
cloning; a transcript is required so the benchmark cannot mislabel reference-only
cloning as combined mode. `--asr` optionally runs local Whisper after TTS exits;
it may fetch separate weights, so use offline mode when those are already cached.
No M4 device run has been performed in this session.
