# llama.cpp Native Server (macOS / Metal)

Run llama.cpp HTTP server natively on Apple Silicon with Metal acceleration. No Docker.
It serves an OpenAI-compatible API that any agent harness (pi, OpenClaw, Hermes,
Codex-style clients) can point at, using GGUF weights from the standard
HuggingFace hub cache (`~/.cache/huggingface/hub`).

## Quick Start

```bash
# 1. Copy and edit config (port, binary)
cp config.yml.example config.yml

# 2. Copy a profile
cp profiles/qwen38-9b-distill-q8.yml.example profiles/qwen38-9b-distill-q8.yml

# 3. Download the weights into the HuggingFace hub cache
./fetch-model.sh download empero-ai/Qwen3.8-9B-Distill-GGUF Qwen3.8-9B-Q8_0.gguf

# 4. Launch (foreground — Ctrl-C to stop)
./launch.sh qwen38-9b-distill-q8

# 5. Or launch in background
./launch.sh qwen38-9b-distill-q8 --background
./launch.sh --status
./launch.sh --stop
```

In background mode the server log goes to `.llama-server.log` in this
directory. Startup is fast — a 9B with `mlock` and a 262144-token KV cache is
listening in ~3.4s, warmup included.

API endpoint: `http://127.0.0.1:<port>/v1/chat/completions` (OpenAI-compatible),
where `<port>` is `server.port` in `config.yml` (8080 unless you change it).

**Adding a new model?** Read [ADDING-A-MODEL.md](ADDING-A-MODEL.md) — the full
runbook, including how to size context against unified memory.

**Connecting an AI harness?** See [Client Integration](#client-integration). The
whole workflow is also packaged as the `llama-cpp-macos` agent skill:

```bash
./install-skill.sh    # into ~/.claude/skills, ~/.agents/skills, ~/.hermes/skills
```

Then ask any Claude Code, Codex, Grok Build, pi or Hermes session to set up or
connect a local model (or invoke `/llama-cpp-macos`, `$llama-cpp-macos`,
`/skill:llama-cpp-macos`).

## Structure

```
llama-cpp-server-macos/
├── launch.sh                           # Launcher (runs binary directly)
├── fetch-model.sh                      # Download weights into the HF hub cache
├── hf_fetch.py                         # Python helper behind fetch-model.sh
├── patch-chat-template.py              # Accept mid-conversation system messages
├── install-skill.sh                    # Install the agent skill per harness
├── skills/llama-cpp-macos/             # Portable agent skill + harness reference
├── config.yml.example                  # Global config template
├── .gitignore                          # Keeps machine-local config and state out of Git
├── profiles/
│   ├── qwen35-9b-distilled-{q4,q5,q8}.yml.example
│   └── qwen38-9b-distill-{q4,q4-fast,q4-max,q5,q6,q8}.yml.example
├── ADDING-A-MODEL.md                   # Runbook for adding a model
└── README.md
```

`*.yml` files are local and gitignored; `*.yml.example` files are committed.
So is `templates/`, generated per machine by `patch-chat-template.py`.

## Requirements

- `llama-server` binary in PATH (via `brew install llama.cpp` or build from source).
  Qwen3.5-class models need a build with Gated DeltaNet support — older builds
  fail to load the architecture.
  Homebrew ships `ggml` as a separate formula: if it gets upgraded without
  `llama.cpp` (e.g. as another formula's dependency), the old binary aborts at
  load with `GGML_ASSERT(...) failed` in `ggml_gated_delta_net`. Fix with
  `brew upgrade llama.cpp`; `otool -L` on its `libllama` shows the ggml it expects.
- PyYAML, from either a venv in this directory
  (`uv venv --python 3.13 && uv pip install pyyaml`) or, if there is none,
  an ephemeral `uv run --with pyyaml` environment. Installing
  [uv](https://docs.astral.sh/uv/) is enough.
- Model GGUF files downloaded locally — see `./fetch-model.sh`. It uses
  `huggingface_hub` from the local venv if present, otherwise an ephemeral
  `uv run --with` environment, so no extra setup is required.

## Referencing models

Preferred: name the repo and file, and let `launch.sh` resolve them through the
HuggingFace hub cache at startup (following `refs/main`).

```yaml
hf_repo: empero-ai/Qwen3.8-9B-Distill-GGUF
hf_file: Qwen3.8-9B-Q8_0.gguf
# hf_revision: <sha>   # optional, pins a specific upload
```

This survives re-fetches and revision bumps, unlike a hard-coded
`snapshots/<hash>/` path. Use `./fetch-model.sh` to populate the cache — it
writes to `~/.cache/huggingface/hub` (honouring `HF_HUB_CACHE` / `HF_HOME`),
the same cache `mlx_lm` and `transformers` use, so one download serves every
runtime on the machine.

```bash
./fetch-model.sh list <repo_id>                    # files and exact sizes
./fetch-model.sh download <repo_id> <file>...      # fetch into the cache
./fetch-model.sh path <repo_id> [file]             # resolve, no network
```

Still supported: `model_path` with an absolute path, or a path relative to
`models_dir` in `config.yml`. Use it for GGUFs that live outside the cache.
If a profile sets both, `model_path` wins.

## Supported Profile Options

The launcher maps common YAML keys to `llama-server` flags:

```yaml
n_gpu_layers: 99        # -ngl 99
ctx_size: 262144        # -c 262144, 256 * 1024
batch_size: 2048        # -b 2048
ubatch_size: 2048       # -ub 2048
parallel: 4             # -np 4
flash_attn: "on"        # -fa on
cache_type_k: q8_0      # --cache-type-k q8_0
cache_type_v: q8_0      # --cache-type-v q8_0
cont_batching: true     # --cont-batching
cache_ram: 8192         # --cache-ram 8192, prompt cache limit in MiB
jinja: true             # --jinja
priority: 1             # --prio 1
```

Sampling defaults, applied to requests that don't set their own — this is how a
profile carries a model card's recommended recipe:

```yaml
temp: 0.6               # --temp 0.6
top_p: 0.95             # --top-p 0.95
top_k: 20               # --top-k 20
min_p: 0.0              # --min-p 0.0
presence_penalty: 1.5   # --presence-penalty 1.5
frequency_penalty: 0.0  # --frequency-penalty 0.0
repeat_penalty: 1.0     # --repeat-penalty 1.0
repeat_last_n: 64       # --repeat-last-n 64
```

Reasoning models and long-context extension:

```yaml
reasoning: auto             # --reasoning auto
reasoning_format: deepseek  # --reasoning-format deepseek
reasoning_budget: 8192      # --reasoning-budget 8192
context_shift: false        # --no-context-shift
rope_scaling: yarn          # --rope-scaling yarn
rope_scale: 4.0             # --rope-scale 4.0
yarn_orig_ctx: 262144       # --yarn-orig-ctx 262144
yarn_ext_factor: -1.0       # --yarn-ext-factor -1.0
yarn_attn_factor: -1.0      # --yarn-attn-factor -1.0
```

## Memory Notes

Two things to internalize before setting `ctx_size`.

**`-c` is the total across slots.** Usable context per request is
`ctx_size / parallel`. `ctx_size: 262144, parallel: 4` gives each request
65536 tokens, not 262144.

**Count only the layers that actually cache.** Qwen3.5-class models are hybrids:
32 blocks arranged as 8 × (3 × Gated DeltaNet → 1 × Gated Attention). Only the
8 Gated Attention layers hold a growing KV cache (4 KV heads × 256 dim); the 24
Gated DeltaNet layers keep a fixed-size recurrent state per slot instead. So:

```
8 × 4 × 256 × 2 = 16,384 values/token   →  32 KiB/token at f16, 17 KiB at q8_0
× 262,144 tokens                        →  8.0 GiB at f16, 4.25 GiB at q8_0
```

Sizing these models as if all 32 layers cached would predict ~32 GiB and push
you into quantizing the cache for no reason.

Confirmed empirically on build 9310. Note it does **not** print a `KV self size`
line; instead, send a short prompt and read `.llama-server.log`:

```
prompt_save: saving prompt with length 770, total state size = 74.328 MiB
create_check: created context checkpoint 1 of 32 ... size = 50.251 MiB
```

The checkpoint is the fixed-size Gated DeltaNet recurrent state (~50 MiB,
independent of token count). The remainder is KV:
`(74.328 − 50.251) MiB ÷ 770 tokens = 32.02 KiB/token` — matching the 32 KiB
derived above to three significant figures.

Budget `weights + KV + ~1–2 GiB` of compute buffers against unified memory.
Suggested starting points:

- Mac mini M4 Pro / 64GB: Q8 at `ctx_size: 262144`, `parallel: 1`, f16 KV
  (~18.6 GiB) for the largest single-request window; or Q4 at
  `ctx_size: 1048576`, `parallel: 4`, q8_0 KV (~24.4 GiB) for four slots at
  full context. `cache_ram: 8192` for both.
- MacBook Pro M5 / 32GB, Q5: `ctx_size: 262144`, `parallel: 1`, q8_0 KV,
  `batch_size`/`ubatch_size` 1024, `cache_ram: 2048` (~12 GiB). Halve
  `ctx_size` if memory pressure climbs with apps open.
- MacBook Pro M5 / 32GB, Q8: prefer Q5 or Q6 for long context. If you need Q8,
  start at `ctx_size: 131072`, `parallel: 1`, q8_0 KV, `cache_ram: 1024`.

The Metal working-set limit is the real ceiling, and llama-server prints it at
startup — on a 64 GiB Mac mini it reports `MTL0 : Apple M4 Pro (53084 MiB)`,
i.e. ~81% of installed RAM. Check that line rather than guessing; raise it with
`sudo sysctl iogpu.wired_limit_mb=…` only if you must.

## Client Integration

Anything that speaks the OpenAI Chat Completions API can use the server:
base URL `http://127.0.0.1:<port>/v1`, any non-empty API key unless the profile
sets `api_key`, and any model id (see below).

Quick checks from the shell:

```bash
curl -s http://127.0.0.1:8080/v1/models
curl -s http://127.0.0.1:8080/v1/chat/completions -H 'Content-Type: application/json' \
  -d '{"messages":[{"role":"user","content":"Say hi in five words."}],"max_tokens":512}'
```

The server also hosts a browser chat UI at `http://127.0.0.1:<port>`.

### Agent harnesses

Verified end to end with pi, Claude Code, Codex, Grok Build and Hermes Agent.
Exact configuration for each, with measured first-turn times, is in
[skills/llama-cpp-macos/references/harnesses.md](skills/llama-cpp-macos/references/harnesses.md):

| Harness | API it uses | Configured in |
|---|---|---|
| pi | Chat Completions | `~/.pi/agent/models.json` |
| Claude Code | Anthropic Messages (`/v1/messages`) | env vars + `--bare --strict-mcp-config` |
| Codex | Responses (`/v1/responses`) | `~/.codex/<profile>.config.toml` |
| Grok Build | Chat Completions | `[model.<name>]` in `~/.grok/config.toml` |
| Hermes Agent | Chat Completions | `model_aliases` in `~/.hermes/config.yaml` |

Harness system prompts and tool lists are 10–20K tokens, so the first turn
spends 30–60s in prefill on a 9B; every configured MCP tool adds to that. Raise
client request timeouts well beyond their defaults for deep-context work.

### Mid-conversation system messages (Claude Code)

Qwen3-family chat templates raise `System message must be at the beginning`
when a system message follows a user turn, and Claude Code sends exactly that.
Generate a tolerant copy of the profile's embedded template once:

```bash
uv run --no-project --with gguf --with pyyaml --with jinja2 \
  python patch-chat-template.py qwen38-9b-distill-q4-max
```

It changes that one check to render the message as a system turn, writes
`templates/<profile>.jinja`, and prints the line to add to the profile:

```yaml
chat_template_file: templates/qwen38-9b-distill-q4-max.jinja   # --chat-template-file
```

### Only one profile is loaded at a time

llama-server serves a single model per process, but a client can list every
profile as selectable. **llama-server ignores the requested model id entirely** —
verified: a request for `bogus-model-name` returns 200 and is served by whatever
is loaded. There is no error to catch.

So `contextWindow` in the client config is a promise about the *profile*, not
about the running server. Ask before assuming:

```bash
./launch.sh --status     # prints the active profile and the loaded GGUF
```

Switching models means re-launching:

```bash
./launch.sh qwen38-9b-distill-q4-fast --background
```

## 32 GB Apple Silicon (e.g. MacBook Pro M5)

The `.example` profiles are sized for a 64 GB Mac mini M4 Pro. On a 32 GB
machine llama-server reports roughly half the Metal working set
(`MTL0 : Apple M5 (25559 MiB)` against the mini's 53084), so resize the local
`profiles/*.yml` rather than copying them verbatim. Sizes that work on a
32 GB MacBook Pro M5:

| profile | ctx x slots | KV | total |
|---|---|---|---|
| `qwen38-9b-distill-q4-fast` | 65536 x 1 | f16, 2.00 GiB | ~8.9 GiB |
| `qwen38-9b-distill-q4` | 131072 x 2 | q8_0, 4.25 GiB | ~11.1 GiB |
| `qwen38-9b-distill-q4-max` | 262144 x 1 | q8_0, 4.25 GiB | ~11.1 GiB |
| `qwen38-9b-distill-q8` | 262144 x 1 | q8_0, 4.25 GiB | ~14.9 GiB |

Two further departures from the 64 GB profiles: turn `mlock` off, because
wiring the weights on a 32 GB laptop that also runs a browser and an editor
pushes the rest of the system into swap; and set `threads` to the
performance-core count (4 on the M5) — counting efficiency cores slows decode.

## Measured Throughput

Mac mini M4 Pro / 64 GB, `qwen38-9b-distill-q8` (Qwen3.8-9B Q8_0, 262144 ctx,
1 slot, f16 KV, `flash_attn: "on"`), llama.cpp build 9310. Measured with
`cache_prompt: false`, reading `.timings` off the response:

| Context | Prefill tok/s | Prefill time | Decode tok/s |
|---:|---:|---:|---:|
| 575 | 402 | 1s | 24.0 |
| 6,897 | 332 | 21s | 22.7 |
| 30,800 | 300 | 103s | 19.7 |
| 60,482 | 248 | 244s | 16.1 |
| 91,253 | 196 | 466s | 13.0 |
| 130,868 | 158 | **827s** | 11.0 |

**Prefill time is the number that bites.** At 250K you wait ~44 minutes before
the first token, and no decode-rate improvement touches that. Prompt-cache hits
(`cache_ram`) skip prefill entirely and are worth far more than any sampling or
quant tuning on repeated turns.

### Q4_K_M vs Q8_0 — the quant only helps at short context

Same prompts, same machine, `qwen38-9b-distill-q4-max` vs `qwen38-9b-distill-q8`:

| Context | Prefill Q8 → Q4 | Decode Q8 → Q4 | Decode speedup |
|---:|---:|---:|---:|
| 571 | 402 → 367 tok/s | 24.00 → 36.48 | **1.52×** |
| 30,796 | 300 → 295 tok/s | 19.71 → 28.25 | 1.43× |
| 130,868 | 158 → 155 tok/s | 11.00 → 12.72 | 1.16× |
| 249,639 | 95 → 95 tok/s | 6.48 → 7.21 | **1.11×** |

Two results worth internalizing:

- **Prefill is unchanged by the quant** — 2623s vs 2629s at 250K. Prefill is
  compute-bound, so smaller weights don't help; at short context Q4 is actually
  *slower* (0.91×) because dequantization is pure added work.
- **The decode advantage decays to nothing with depth**, 1.52× → 1.11×, because
  the f16 KV cache is identical on both quants and comes to dominate per-token
  traffic once it is large.

So the intuition "use a smaller quant to afford a bigger context" is backwards
here. Q4_K_M is worth choosing for short-context interactive work; for deep
context it trades reasoning quality for ~11% decode. Use Q8_0 there.

Resident set was 17.3 GiB at load, against the 18.6 GiB the profile header
predicts — the hybrid KV math above holds. It grew to ~19.8 GiB after a series
of long-context requests, as the prompt cache (`cache_ram: 8192`) and the
compute buffers filled. Budget for the steady-state figure, not the load-time one.

Note that decode speed is **not** flat in context depth, despite flash
attention: it roughly halves by 90K tokens, because every generated token
streams the whole KV cache for the attention layers. Plan long-context work
around that; the 256K window is real, but its tail is slow.

`spec_default: true` maps to `--spec-default`; keep it opt-in and benchmark it
for the model/workload before leaving it on. For newly-added llama.cpp flags,
use `extra_args`:

```yaml
extra_args:
  - --slots
```
