# Nemotron diffusion porting evidence

Source research checkpoint: 2026-11-08. This model-specific reference is
accompanied by current implementation/evidence in HANDOFF.md. No model Python was imported or executed.

## Public sources and provenance

- [Model repository](https://huggingface.co/nvidia/Nemotron-Labs-Diffusion-3B),
  local source revision `0d51902da1f8869f83413ce642fab402fa5641e0`.
- Model files: `config.json`, `modeling_ministral.py`,
  `modeling_nemotron_labs_diffusion.py`, `tokenizer.json`,
  `tokenizer_config.json`, `chat_template.jinja`.
- [Public YaRN reference](https://github.com/huggingface/transformers/blob/v5.0.0/src/transformers/modeling_rope_utils.py).
  Reading its formula does not introduce a Transformers runtime dependency.
- [Candle safetensors source](https://github.com/huggingface/candle/blob/main/candle-core/src/safetensors.rs)
  and [Mistral source](https://github.com/huggingface/candle/blob/main/candle-transformers/src/models/mistral.rs)
  inspected online on the checkpoint date. These `main` links are unpinned
  research references; no source was copied and no temporary clone was created.
- [Rust tokenizers](https://github.com/huggingface/tokenizers): proposed narrow
  tokenizer dependency, pending initial scope approval.

The listed local model code, configuration and tokenizer/template files match
tracked content at the stated model revision. Do not infer whole-checkout
cleanliness: an ordinary Git status attempted an LFS clean filter in a read-only
sandbox and failed. Use explicit file diffs for source review.

The model's license is separate from this repository's license. Before adopting
upstream implementation text, preserve its actual attribution and notices; no
relicensing of weights or model code is implied here.

## Audited checkpoint

The complete `model.safetensors` file is 7,663,347,000 bytes. Its SHA-256 matches
the Git LFS pointer: `8c4beff3a8a2ec76708f33e67d18c8baecf099ebd5aa5a152ea1cce494b2b51c`.
The 27,952-byte JSON header describes 237 BF16 tensors with 7,663,319,040 payload
bytes and 3,831,659,520 scalar elements. All offsets are contiguous, every shape
matches its byte span, the full expected name set matches, and EOF is exact.
This establishes file integrity and layout, not inference correctness.

| Property | Value |
| --- | --- |
| hidden width | 3072 |
| layers | 26 |
| query heads / KV heads | 32 / 8 |
| head dimension | 128, explicitly configured |
| query projection width | 4096, distinct from hidden width |
| key/value projection width | 1024 |
| MLP intermediate width | 9216 |
| vocabulary | 131072 |
| RMSNorm epsilon | 0.00001 |
| BOS / EOS / mask IDs | 1 / 11 / 100 |
| block size | 32 |
| advertised context | 262144 |
| bias / tied embedding / sliding window | absent / false / none |

For each layer prefix `encoder.layers.{i}.`:

| Suffix | On-disk shape [output, input] |
| --- | --- |
| `self_attn.q_proj.weight` | [4096, 3072] |
| `self_attn.k_proj.weight`, `self_attn.v_proj.weight` | [1024, 3072] |
| `self_attn.o_proj.weight` | [3072, 4096] |
| `mlp.gate_proj.weight`, `mlp.up_proj.weight` | [9216, 3072] |
| `mlp.down_proj.weight` | [3072, 9216] |
| `input_layernorm.weight`, `post_attention_layernorm.weight` | [3072] |

Outside layers: `encoder.embed_tokens.weight` and `diffusion_head.weight` each
have shape [131072, 3072], with separate storage; `encoder.norm.weight` is [3072].
A linear operation is X times the transpose of the stored matrix.

## Explicit forward path

Embed token IDs. Each layer applies input RMSNorm, Q/K/V projections, positional
rotation, query-position scaling, grouped-query attention, output projection and
residual addition. Then apply post-attention RMSNorm, gated SiLU MLP and another
residual addition. Finally apply encoder RMSNorm and the separate diffusion head.
The gated MLP is `down(silu(gate(x)) * up(x))`.

RMSNorm accumulates mean squares in FP32. The upstream BF16 path rounds normalized
activations back to their input dtype before multiplying the norm weight. CPU
FP32 and GPU BF16 comparisons need distinct declared tolerances and rounding
policies. Neither random text plausibility nor top-token agreement alone is a
full numerical parity check.

Do not derive head dimension as hidden width / query heads: that gives 96,
whereas this checkpoint explicitly uses 128. Map each group of four query heads
to one KV head. Native code can index grouped KV directly without replicating it.

### Positional rotation

The model uses YaRN with theta 1,000,000, factor 16, original context 16384,
`beta_fast=32`, `beta_slow=1`, `mscale=1`, `mscale_all_dim=1`.
For pair index j in 0..64, the unscaled inverse frequency is
`f_j = theta^(-2*j/128)`. The correction range, using the public reference's
floor/ceil convention, is low=20 and high=37. Let
`r_j = clamp((j-20)/(37-20), 0, 1)`. The mixed inverse frequency is
`f_j * ((1-r_j) + r_j/16)`.

The magnitude multiplier is **1**: the two configured mscale terms form a ratio
and cancel. Do not apply the often-used `1 + 0.1*ln(16)` multiplier here.

Rotation pairs the first and second halves of each head, not adjacent elements.
After rotation, multiply Q at absolute position p by
`1 + 0.1 * ln(1 + floor(p/16384))`, then scale attention logits by
`1/sqrt(128)`. K does not get this additional query-position multiplier.
Test positions immediately below/at 16384 and multiple later boundaries.

### Attention and cache rules

Use explicit per-call attention mode and cache policy instead of mutating shared
layer flags. Causal query position p attends only keys at absolute positions <=p.
Diffusion queries attend all cached prefix keys and every key in the current
block, including future positions within that block.

A denoising forward concatenates prefix KV with current-block KV for attention
without appending to persistent prefix storage. Only a causal commit forward
extends the cache. Every request owns its cache and scratch buffers.

## Block diffusion behavior to port

1. Causally prefill the prompt and select the first seed from its last logits.
2. Append one block of masks, replacing its first position with that seed.
3. Repeatedly evaluate the current block with bidirectional attention and the
   unchanged prefix cache. Logit at position i predicts position i directly.
4. Select candidate tokens and their softmax confidence. Without a threshold,
   split the initially masked count across block-length steps, front-loading the
   remainder. With a threshold, consider all still-masked positions in confidence
   order: always transfer the best one and transfer others meeting the threshold.
5. Stop refinement when all required positions resolve. An EOS with unresolved
   earlier positions cannot yet terminate the output. A candidate mask ID also
   needs explicit bounded handling so it cannot cause an infinite loop.
6. Causally evaluate the committed block to extend prefix KV and select the next
   block's first token. Return finalized output within the requested token budget.

The upstream implementation requires max_new_tokens divisible by block_length.
The proposed API instead promises arbitrary positive budgets, so document and
independently test the final partial-block adaptation. A whole-block computation
followed by final output truncation is another possible implementation; choose a
policy explicitly because it can change the bidirectional result.

Upstream diffusion's NFE counter excludes initial prefill. Native reporting should
count actual forward passes, documenting this intentional reporting difference.
The approved follow-up requires self-speculation as well as AR and diffusion.
Optional LoRA remains deferred; no adapter is needed for the base hybrid mode.

## Tokenization, chat and tools

`tokenizer.json` defines byte-level BPE with Unicode-aware regex splitting,
no normalizer and 1000 added tokens. Do not substitute SentencePiece, assume
ASCII, or insert BOS just because its ID exists. The configured tokenizer does
not automatically add BOS/EOS. EOS is `<|im_end|>` with ID 11, distinct from `</s>`.

The template emits a system turn even when no system message was supplied.
Default generation suffix is `<|im_start|>assistant` followed by newline and
`<think></think>`. Explicit thinking mode opens a thought instead. Preserve
previous assistant reasoning according to the template's truncation rules.

Tool definitions and calls use XML-shaped `<tool_call>`, `<function=...>` and
`<parameter=...>` blocks. The historical `[TOOL_CALLS]` special token is not the
format this chat template emits. Incoming OpenAI tool arguments are JSON strings;
normalize them to structured values before rendering the template. On output,
map strings, numbers, booleans and structured values using the supplied schema,
preserve multiple calls and associate subsequent tool results with call IDs.

Three standalone Jinja reference checks passed: a Unicode user prompt, thinking
mode suffix, and synthetic tool-call/tool-result history. The reference uses
`ImmutableSandboxedEnvironment(trim_blocks=True, lstrip_blocks=True)` with JSON
serialization configured to preserve Unicode and insertion order. A default
Jinja environment produces an extra leading newline; do not accept it as parity.
These checks do not validate tokenizer IDs, a Rust renderer, model tool generation
or a real harness. No Transformers module was imported.

## Memory and backend constraints

Raw BF16 weights occupy about 7.137 GiB. A full FP32 copy doubles that storage,
and retaining both mapped and converted weights can raise resident memory.
Consider bounded conversion tiles on CPU and direct BF16 GPU storage.

For batch one, BF16 KV costs `26 * 2 * 8 * 128 * 2 = 106496` bytes per token:
8192 tokens need 0.8125 GiB; the full advertised context needs 26 GiB for KV alone.
Thus a configuration's maximum context is not a promise that it fits the selected
device. Bound context against weights, KV, logits, scratch space and admission.
Do not eagerly reserve the full advertised context on a modest GPU.

CPU kernels provide a debugging reference. Direct CUDA GEMM and small specialized
kernels can supply acceleration without a tensor framework; the CUDA C++ backend is now implemented. See HANDOFF.md for its actual
verification state; compilation does not establish Blackwell hardware support. Hardware discovery can require execution outside
an agent sandbox; preserve machine-specific findings in private handoff records.

Candle's separation of mapped assets, dtype handling and model-specific layers is
useful reference material. Its generic Mistral implementation is not a drop-in
for this model's YaRN, query scaling or diffusion cache semantics. If code is
later copied, pin a public revision and include the applicable license notices.

## Numerical evidence still required

Use independent synthetic cases for BF16, matmul layout, RMSNorm, SiLU, YaRN,
grouped attention, causal masks, prefix preservation, threshold fallback, EOS,
stop strings and streamed Unicode. Test a small complete model before expensive
real weights. Compare real embedding output, layer-0 Q/K/V, attention/MLP residuals,
final normalization and selected logits with an independent reference that does
not import Transformers. An independent NumPy reference is used as development-only evidence, never
as the runtime. PyTorch is prohibited by the approved follow-up. See HANDOFF.md
for actual comparison results and outstanding numerical limits.

## Public source hashes

| Asset | SHA-256 |
| --- | --- |
| config.json | fc6ec1011e5f4ff858a104e8e04599e931e45dc874c71b9e73182a7f2dbdec44 |
| configuration_nemotron_labs_diffusion.py | 2b18216e1b4e0d89b728c1c871744088a28004564f99009809294b39ec677b57 |
| modeling_ministral.py | 021afd8a72e23c5bf3a309f9dc5f0a7771db2424c148be5ce2a2737bf4cb7d65 |
| modeling_nemotron_labs_diffusion.py | 29d73c5709e90e3be3c7e537edb61a84fbb3dc1c286b1eaed42d899a3a4e4760 |
| chat_template.jinja | 24901e3846b530e3ed20436b26ea1cd7b3768ab2b2645e31c47df1413ab289dc |
| tokenizer.json | 623c34567aebb18582765289fbe23d901c62704d6518d71866e0e58db892b5b7 |
| tokenizer_config.json | 50d5c40a1e06a86e8d0fc2d4c6a9bf73ed0ffca7bdad3c9b42cedca48baa25dc |

## Delivered native precision policy

The source discussion above records upstream behavior. The delivered native
runtime keeps weights and tensor-core GEMM inputs in BF16 and all activation,
residual, logit and KV state in FP32. An all-BF16 activation prototype accumulated
rounding differences across26 layers; the FP32-state policy passed the unchanged
independent full-forward threshold. CPU and NumPy references use the same declared
policy, with independent implementations. Current results are in VERIFICATION.md.

FP32 KV therefore costs 208 KiB per token, or 3.25 GiB for the recommended 16384-token context; the earlier
104 KiB/26 GiB calculation describes hypothetical BF16 KV, not the current cache.
The memory preflight uses the actual FP32 allocation sizes. Host weight mappings
are released by the server after upload. Diagnostic captures are opt-in `.f32`
files for the first native forward only and must remain outside this repository.
