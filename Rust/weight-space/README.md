# Rust weight-space experiments

Rust library and CLI for hash-checked, copy-only BF16 weight editing and paired
local inference experiments. The first experiment reconstructs the published
Qwen3.5-0.8B `p50c` loop-suppression edit. CUDA inference uses an existing
llama.cpp Docker image; this is not a Rust-native inference implementation.

The editor does not use Python, PyTorch, Transformers, gradients or an optimizer.
Replaying this known patch is a reproducibility experiment, not a demonstration
that this tool independently discovers improvements. A separate reusable
projection routine implements the relevant linear algebra for future work.

See the [research audit](../../research/weight-space-programming/rtx3060-research.md),
[experiment protocol](docs/EXPERIMENT_PLAN.md), and
[verification record](docs/VERIFICATION.md).

Measured on the RTX 3060, with frozen synthetic math and exact-reply tasks:

| Model/settings | Correct at 8,192 tokens (24 cases) | Correct at 32,768 tokens (fixed 6 cases) |
|---|---:|---:|
| Base | 11/24 | 4/6 |
| Published weight edit | 22/24 | 6/6 |
| Base + repetition penalty 1.1 | 22/24 | 6/6 |

The weight edit recovered 11 primary answers without a baseline regression;
the decoding control recovered 12 and regressed one. Both helped on this small
workload; it does not establish an overall advantage for editing or reproduce
MATH-500/GSM8K. All 90 frozen outputs passed independent arithmetic, grading and
request checks. The source model remained unchanged.

## Build and test

On Linux with Rust/Cargo:

```sh
cargo build --release --locked
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --check
```

Use the release binary for whole-model hashes and byte audits; debug builds are
much slower on multi-gigabyte files. No CUDA toolkit is needed to build the Rust
tools. GPU evaluation requires NVIDIA Container Toolkit, Docker and a compatible
existing llama.cpp CUDA server image. Downloading/building that backend is a
separate setup step; the tool never pulls an image automatically.

## Fetch and reconstruct the published edit

Choose a private data directory **outside every Git checkout**. Output paths are
explicit, existing outputs are rejected, and model/evidence writes inside Git
checkouts are rejected, including through symlink aliases.

```sh
WSP_BIN="$PWD/target/release/claw-wsp"
WSP_DATA=/path/to/private/wsp-experiment
mkdir -p "$WSP_DATA/assets"
"$WSP_BIN" fetch --asset model --output "$WSP_DATA/assets/base.gguf"
"$WSP_BIN" fetch --asset manifest --output "$WSP_DATA/assets/manifest.json"
"$WSP_BIN" fetch --asset patch --output "$WSP_DATA/assets/edit.npz"
"$WSP_BIN" inspect --model "$WSP_DATA/assets/base.gguf"
"$WSP_BIN" patch \
  --model "$WSP_DATA/assets/base.gguf" \
  --manifest "$WSP_DATA/assets/manifest.json" \
  --patch "$WSP_DATA/assets/edit.npz" \
  --output "$WSP_DATA/edited"
"$WSP_BIN" audit \
  --model "$WSP_DATA/assets/base.gguf" \
  --edited "$WSP_DATA/edited/model.gguf" \
  --manifest "$WSP_DATA/assets/manifest.json" \
  --patch "$WSP_DATA/assets/edit.npz"
```

The pinned BF16 GGUF is 1,557,662,496 bytes. Allow at least 4 GB of disk space
for source, edited copy and evidence. `fetch` pins revision and SHA-256 for each
asset, permits HTTPS redirects for public model storage, and does not use ambient
credentials or proxies. Model fetch was bootstrapped with curl in the recorded
experiment, then the Rust fetcher independently retrieved and verified all three
assets, including the full model through its public storage redirects.

`patch` requires a fresh output directory whose parent exists and is outside the
source checkpoint directory. It validates every sparse replacement before
creating an output, copies the source, verifies both tensor hashes, independently
compares every byte and publishes `model.gguf` with `receipt.json` only after the
audit passes. A failed temporary checkpoint is not published as `model.gguf`.
The original file is only opened for reading. Rollback means selecting it again.

Supported edit inputs are sparse NPZ/NPY patches with little-endian signed 32-bit
indices and unsigned 16-bit BF16 replacement bits, described by the released
manifest schema. Only MLP `down_proj` tensors can be targeted. Quantized target
tensors, unsupported forms, mismatched hashes, no-op replacements, nonfinite
values, invalid dimensions and duplicate/out-of-range indices are rejected.
This is deliberately not a general GGUF converter or an arbitrary-model editor.

## Run a controlled comparison

Select the RTX 3060 explicitly using its index or UUID from `nvidia-smi`. Pin the
**full image ID** of a locally installed backend. The recorded backend was
llama.cpp build 10603, commit `c060ca974`, image ID below.

```sh
WSP_GPU=1
WSP_IMAGE=sha256:68347eb344cc9a0c40267f1d07f7ad4313fba66af16df01d11a84a85b0e94df4
"$WSP_BIN" cases --seed 78323 --count 24 --output "$WSP_DATA/cases.json"
"$WSP_BIN" evaluate \
  --model "$WSP_DATA/assets/base.gguf" --cases "$WSP_DATA/cases.json" \
  --output "$WSP_DATA/base" --arm base --max-tokens 8192 \
  --gpu "$WSP_GPU" --image "$WSP_IMAGE"
"$WSP_BIN" evaluate \
  --model "$WSP_DATA/edited/model.gguf" --cases "$WSP_DATA/cases.json" \
  --output "$WSP_DATA/edit" --arm edit --max-tokens 8192 \
  --gpu "$WSP_GPU" --image "$WSP_IMAGE"
"$WSP_BIN" evaluate \
  --model "$WSP_DATA/assets/base.gguf" --cases "$WSP_DATA/cases.json" \
  --output "$WSP_DATA/decoding" --arm decoding --max-tokens 8192 \
  --gpu "$WSP_GPU" --image "$WSP_IMAGE"
"$WSP_BIN" compare --left "$WSP_DATA/base" --right "$WSP_DATA/edit"
"$WSP_BIN" compare --left "$WSP_DATA/base" --right "$WSP_DATA/decoding"
```

Each `evaluate` owns one temporary container, publishes only loopback port 8097,
mounts the selected model directory read-only, waits for health, runs serially
and stops that container when it completes or returns an ordinary error. It
fails if its port is occupied and never stops another server. Options `--port`
and `--context` select alternatives; context defaults to 40,960 tokens. Keep at
least 512 tokens above the generation cap, with additional room for longer
prompts. An external kill or host failure can bypass cleanup; identify only
the experiment container named `claw-wsp-<process-id>-<port>` before stopping it.

The runtime renders its real chat template with thinking enabled. Requests use
temperature 0.6, top-p 0.95, top-k 20, min-p 0, presence penalty 1.5, frequency
penalty 0 and an explicit context-sized repetition window. The decoding arm
adds repetition penalty 1.1. Returned parameters and seeds are verified. Backend
penalty semantics need not exactly match vLLM; this is a local comparison.

The frozen cases, tool/model/image hashes, settings, rendered requests, raw
responses, token IDs and errors are retained in the selected output directory.
Every completed attempt is flushed to disk. `summary.json` separates correctness,
thinking completion, token caps and repetition. API/protocol errors produce
nonzero CLI exit status; a model that simply answers incorrectly does not.

Math grading accepts the last complete boxed integer in the final answer after
`</think>`. Numbers occurring only in reasoning do not count. Exact-string
probes require precisely the requested final text. The repetition diagnostic is
an identical contiguous 32-token n-gram appearing at least four times in an
output of at least 128 tokens; overlaps count. It is not a semantic loop judge.

`compare` checks identical case identity, settings, tool hash, runtime and seeds.
It retains errors in the denominator and reports paired wins/losses, a fixed-seed
10,000-resample percentile bootstrap interval, and exact two-sided McNemar p.
These small synthetic cases cannot establish general model quality or reproduce
MATH-500/GSM8K scores. Inspect raw outputs as well as summary percentages.

## Projection API

`projection::project` takes a row-major matrix `W[out,in]`, a nonzero output-space
direction, selected input columns and a finite alpha. It returns
`W - alpha * u * (u^T W M)` using normalized `u`, FP32 inputs/outputs and wider
accumulation. It rejects invalid/nonfinite inputs and preserves unselected
columns. This algebra does not determine which direction has a useful meaning.
The [geometry note](docs/EDIT_GEOMETRY.md) derives the operator and its limits.

The `project` CLI reads JSON fields `weights`, `rows`, `cols`, `direction`,
`selected`, `alpha` and writes a new JSON array. It does not modify a checkpoint.

## Provenance and PDD

Own Rust code: MIT. Model: Qwen Apache-2.0. Released patch/data: Takehi et al.,
CC BY 4.0; fetched rather than bundled. See [NOTICE.md](NOTICE.md).

The approved intent is preserved under `docs/intents/`. PDD recorded that event,
then its local architecture stage failed because the configured chat endpoint
could not provide the required tool-capable coding-agent workflow. The modular
prompts, architecture map, human stories and executable tests were maintained
directly. No completed automated PDD generation/synchronization or generated
story-contract validation is claimed.
