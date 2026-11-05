# Adding a Model

> **Audience:** AI agents and human operators adding a new GGUF model to
> the native macOS llama.cpp server in this directory.
>
> Follow this end to end. It is written so an agent with only a HuggingFace URL
> can produce a working, well-reasoned profile without guessing.

---

## The shape of the thing

Two files per model, and one of them is not committed:

| File | Committed? | Purpose |
|---|---|---|
| `profiles/<name>.yml.example` | yes | The template. Portable, annotated, no machine-specific paths. |
| `profiles/<name>.yml` | **no** (`.gitignore`) | What `launch.sh` actually reads. Local tweaks: host, thread count, context. |

Weights never live in the repo. They go to the standard HuggingFace hub cache
(`~/.cache/huggingface/hub`), the same cache `mlx_lm` and `transformers`
populate, so one download serves every runtime on the machine.

Profiles reference a model by **repo + filename**, not by path:

```yaml
hf_repo: empero-ai/Qwen3.8-9B-Distill-GGUF
hf_file: Qwen3.8-9B-Q8_0.gguf
```

`launch.sh` resolves that through `refs/main` at startup. Do not paste a
`snapshots/<sha>/` path into a profile — it breaks the next time the repo is
re-fetched. (The older `model_path:` key still works for models outside the
cache.)

---

## Recipe

### 1. Read the model card, then the base model's card

The quantizer's card gives you filenames, sizes, and a sampling recipe. It
almost never gives you the architecture. **Follow its "Base model" link** — the
context length, layer layout, and per-mode sampling parameters live upstream.

Collect, and write into the profile header:

- Native context length, and whether YaRN/RoPE scaling is offered beyond it
- Layer count, and how many are **full attention** vs. linear/recurrent/SSM
- KV heads and head dimension for the attention layers
- Recommended temperature / top_p / top_k, and any warnings
- Whether it is a reasoning model (`<think>` blocks) and how to handle them
- Any minimum llama.cpp build requirement

### 2. Download

```bash
./fetch-model.sh list empero-ai/Qwen3.8-9B-Distill-GGUF
./fetch-model.sh download empero-ai/Qwen3.8-9B-Distill-GGUF Qwen3.8-9B-Q8_0.gguf
```

`list` shows exact byte sizes — use them for the sizing math below rather than
the card's rounded decimal GB. `download` prints the `hf_repo`/`hf_file` keys to
paste into the profile.

### 3. Size the context

Two numbers decide everything. Get them right and the rest is boilerplate.

**Usable context per request is `ctx_size / parallel`.** `-c` is the *total*
across server slots, not per slot. This is the single most common mistake:
`ctx_size: 262144, parallel: 4` gives each request 65536 tokens, not 262144.

**KV cache size** — for a dense transformer:

```
bytes_per_token = n_layers x n_kv_heads x head_dim x 2 x bytes_per_value
```

where `bytes_per_value` is 2.0 for `f16`, 1.0625 for `q8_0`, 0.5625 for `q4_0`.
Multiply by `ctx_size` (the total, not per slot).

For **hybrid** models — Qwen3.5, Jamba, Falcon-H1 and friends — count only the
full-attention layers. The linear-attention/recurrent layers hold a fixed-size
state per slot, not a per-token cache, and they are usually the majority. Worked
example, Qwen3.5-9B: 32 blocks arranged as 8 x (3 x Gated DeltaNet -> 1 x Gated
Attention), so 8 attention layers with 4 KV heads x 256 dim:

```
8 x 4 x 256 x 2          = 16,384 values/token
  x 2 bytes (f16)        = 32 KiB/token
  x 262,144 tokens       = 8.0 GiB
```

Treating it as a dense 32-layer model would predict 32 GiB and push you into
quantizing the cache for no reason. **Always check which layers actually cache.**

Then budget: `weights + KV + ~1-2 GiB compute buffers` against unified memory.
Stay under ~60% of installed RAM and macOS will not fight you. Above roughly
75% you need to raise the Metal working-set limit
(`sudo sysctl iogpu.wired_limit_mb=...`), which is worth avoiding.

### 4. Pick the trade

Context and throughput compete for the same memory. Commit to a position per
profile and say so in the header:

- **Max context** — `parallel: 1`, `ctx_size` = native window, `f16` KV if it
  fits. One request gets the model's full window.
- **Concurrency at full context** — `parallel: 4`, `ctx_size` = 4 x native,
  `q8_0` KV. Costs ~2x the memory of the above; use a smaller quant to pay for it.
- **Constrained machine** — `parallel: 1`, halve `ctx_size`, `q8_0` KV, smaller
  `batch_size`/`ubatch_size`, `cache_ram: 2048`.

Speed levers, roughly in order of effect:

| Lever | Effect |
|---|---|
| `flash_attn: "on"` | Large constant-factor win on attention, and required for long context to be practical at all. Always on. It does **not** make decode speed independent of context depth — see below. |
| `batch_size` / `ubatch_size` | Prompt-processing throughput. 2048 on 64 GB, 1024 on 32 GB. Spikes memory. |
| `cache_type_k` / `_v: q8_0` | Halves KV. At long context the win is bandwidth; at short context f16 is usually slightly faster. Measure. |
| `mlock: true` | Stops macOS compressing weights out from under you. |
| `threads` | P-core count (10 on M4 Pro). Minor with all layers on Metal. |
| `priority: 1` | Lower scheduling latency. |
| `spec_default: true` | Speculative decoding. Opt-in, benchmark before trusting. |

### Decode speed falls as context fills — measure it

Flash attention is often described as making generation speed independent of
context length. It is not. Every decoded token still streams the whole KV cache
for the attention layers, so throughput decays as the cache fills. Measured on
an M4 Pro / 64 GB, Qwen3.8-9B Q8_0, f16 KV, `flash_attn: "on"`:

| Context | Prefill tok/s | Prefill time | Decode tok/s |
|---:|---:|---:|---:|
| 575 | 402 | 1s | 24.0 |
| 30,800 | 300 | 103s | 19.7 |
| 60,482 | 248 | 244s | 16.1 |
| 91,253 | 196 | 466s | 13.0 |
| 130,868 | 158 | **827s** | 11.0 |

Roughly **half the generation speed by 90K tokens**, and the decay is steeper
than a pure weights-plus-KV bandwidth model predicts — which is exactly why
this belongs in a measurement rather than an estimate.

And note which column actually hurts: at 128K, **prefill alone is ~14 minutes**
before the first token. When you promise a 256K window, that is the real cost,
not the decode rate.

### Decode is bandwidth-bound — know the ceiling before tuning

Every decoded token streams the whole weight file, so decode has a hard roof:

```
peak_tok_per_s ≈ memory_bandwidth / weight_file_bytes
```

M4 Pro is ~273 GB/s. For Qwen3.8-9B Q8_0 (9.79 GB) that is 27.9 tok/s; measured
24.0, i.e. **86% of theoretical peak**. There is essentially no tuning left at
that point — the only lever is fewer bytes per token (smaller quant, smaller
model). Compute this ratio before spending time on flags; if you are already
near the roof, you are optimizing the wrong thing.

### The quant's benefit shrinks as context deepens

A smaller quant speeds decode only while the weights dominate the bytes moved
per token. Once the KV cache is large it dominates instead, and it is the same
size regardless of quant. Measured, Qwen3.8-9B, Q4_K_M against Q8_0:

| Context | Decode speedup | Prefill speedup |
|---:|---:|---:|
| 571 | 1.52× | 0.91× |
| 130,868 | 1.16× | 0.98× |
| 249,639 | 1.11× | 1.00× |

Prefill barely moves at all — it is compute-bound, so dequantization is added
work, not saved work. Do not assume "smaller quant → bigger usable context";
measure both ends of your intended range before choosing.

### For reasoning models, tok/s is not what the user experiences

An answer with ~1500 thinking tokens and ~300 visible ones spends ~80% of its
wall clock before anything appears. At 24 tok/s decode, visible throughput is
~4 tok/s. Report **time-to-first-visible-token** and **seconds per turn**
alongside tok/s, or the numbers will mislead.

The largest lever is usually skipping the thinking, not the quant. Check the
key against the GGUF's own template rather than copying it from another
profile — `curl -s localhost:8080/props | jq -r .chat_template | grep -o
'enable_thinking'`. Qwen3.5/3.8 use `enable_thinking`; a wrong key such as
`thinking` is silently ignored, leaving reasoning on while you believe it is off.

Benchmark a new model the same way before writing throughput claims into its
profile header. Use `cache_prompt: false` so prompt-cache reuse does not
flatter the numbers, and read `.timings` off the response:

```bash
curl -s localhost:8080/v1/chat/completions -H 'Content-Type: application/json' \
  -d '{"messages":[{"role":"user","content":"..."}],"max_tokens":600,"cache_prompt":false}' \
  | jq '.timings | {prompt_n, prompt_per_second, predicted_n, predicted_per_second}'
```

### 5. Reasoning models

If responses open with `<think>`:

```yaml
jinja: true
reasoning_format: deepseek   # -> message.reasoning_content, not message.content
# reasoning_budget: 8192     # cap runaway deliberation
n_predict: -1                # never cap thinking server-side
```

`deepseek` keeps the thinking out of `message.content` so OpenAI-compatible
clients get clean text without each one stripping tags itself. Use `none` only
if a client needs the raw tags.

### 6. Write both files

Write `.yml.example` first — portable, fully annotated, `127.0.0.1`. Then copy
to `.yml` and apply local changes (LAN host, thread count, tighter context).
State the sizing arithmetic in the header comment; the next agent should not
have to redo it.

### 7. Verify

```bash
./launch.sh                        # profile appears in the list
./launch.sh <name> --background
./launch.sh --status
curl -s localhost:8080/v1/models | jq .
curl -s localhost:8080/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{"messages":[{"role":"user","content":"2+2? Answer with the number only."}],"max_tokens":2048}' | jq .
./launch.sh --stop
```

In the startup log, confirm three lines against your arithmetic:

- `new slot, n_ctx = N` — equals `ctx_size / parallel`, and is not below `n_ctx_train`
- `MTL0 : … (NNNNN MiB)` — the Metal working-set ceiling your total must fit under
- No `model architecture not supported` error — a too-old llama.cpp build fails
  here, not later

Build 9310 does **not** print a `KV self size` line. To check your KV
arithmetic, send one short prompt and read the log:

```
prompt_save: saving prompt with length 770, total state size = 74.328 MiB
create_check: created context checkpoint 1 of 32 ... size = 50.251 MiB
```

Subtract the checkpoint (the fixed recurrent state, constant in token count)
from the total and divide by the token count to get bytes/token. For Qwen3.8-9B
that gives 32.02 KiB/token against 32 KiB derived — if yours is off by ~4x, you
counted every layer instead of only the attention layers.

---

## Beyond the native window: YaRN

Only when a request genuinely needs more than the native context. Static YaRN
degrades short-context quality, so it is a per-profile decision, not a default.
Qwen3.5 documents `factor=4.0` over a 262144 original context for ~1M tokens:

```yaml
ctx_size: 1048576
parallel: 1
rope_scaling: yarn
rope_scale: 4.0
yarn_orig_ctx: 262144
cache_type_k: q8_0
cache_type_v: q8_0
```

Prompt processing at that length is slow enough to be impractical for
interactive use. Keep it in a separate profile.

---

## Adding a flag `launch.sh` does not map

Use `extra_args` first — it needs no code change:

```yaml
extra_args:
  - --slots
```

If it turns out to be broadly useful, add a mapping in the Python block of
`launch.sh` (`add()` for a value flag, `add_bool()` for an on/off pair) and
document it in `README.md`. Check `llama-server --help` for the exact spelling;
flags move between releases.

---

## Checklist

- [ ] Read the quantizer's card **and** the base model's card
- [ ] Downloaded via `./fetch-model.sh` — nothing copied into the repo
- [ ] Profile uses `hf_repo`/`hf_file`, no snapshot hash
- [ ] `ctx_size / parallel` is the context you actually intended
- [ ] KV math counts only the caching layers, and is written in the header
- [ ] `weights + KV + ~2 GiB` fits comfortably in unified memory
- [ ] Sampling defaults match the card; `temp: 0` avoided on reasoning models
- [ ] Both `.yml.example` (committed) and `.yml` (local) exist
- [ ] Server starts, `/v1/models` responds, a real completion comes back
- [ ] `KV self size` and `n_ctx_per_seq` in the log match the header comment
