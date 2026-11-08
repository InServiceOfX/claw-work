# VoxCPM2-bf16 weights inventory

Checkpoint: `mlx-community/VoxCPM2-bf16` (MLX conversion of `openbmb/VoxCPM2`).
Files: `model-00001.safetensors` (~4.29 GB), `model-00002.safetensors` (~0.67 GB),
`model.safetensors.index.json` (`total_size` = 4,955,158,672 bytes). **820 tensors**:
577 BF16 (whole LM/DiT/LocEnc stack), 242 F32 (AudioVAE, plus 9 RoPE buffers), 1 I32.

The tables below were generated from the safetensors headers (fetched with HTTP range
requests: the first 8 bytes are a little-endian u64 header length, then a JSON header).
Repeated transformer layers are collapsed to `layers.{i}`; repeated VAE residual units to
`res{r}` (r = 1, 2, 3 → dilation 1, 3, 9). The `count` column expands the pattern; group
counts add up to 820. "shard" is the shard file (or files, for a collapsed row) that hold the key.

## Conventions that matter for candle

1. **Linear weights are PyTorch layout `[out, in]`.** MLX `nn.Linear` stores `[out, in]` and
   computes `x @ W.T + b`, the same as `candle_nn::Linear`. No transpose is needed. Biases:
   the LM-family linears (q/k/v/o, gate/up/down) have **no bias**. All top-level projections,
   `in_proj`/`cond_proj`/`out_proj`, and the timestep MLPs have biases. `stop_head` has none.
2. **Conv1d weights are MLX channels-last `[out, kernel, in/groups]`.** candle's
   `conv1d` expects PyTorch `[out, in/groups, kernel]`, so apply `permute(0, 2, 1)`
   (`.transpose(1, 2)`) followed by `.contiguous()`. Activations in MLX are `[B, T, C]`; candle
   conv expects `[B, C, T]`.
3. **ConvTranspose1d weights (`*.conv_t.weight`) are MLX `[out, kernel, in]`**. Example:
   `decoder.blocks.layers.0.conv_t.weight` is `1024×16×2048` = (out=1024, K=16, in=2048).
   candle `conv_transpose1d` expects PyTorch `[in, out/groups, kernel]`, so use
   `permute(2, 0, 1)` → `2048×1024×16`.
4. **Weight norm is already folded.** No `weight_g`/`weight_v` keys exist, and the unused
   `fc_logvar` head was dropped. The stored `weight` is the effective conv weight
   `g * v / ||v||` (the norm is taken per output channel over (in, k)).
5. **Depthwise convs**: `res*.conv1.weight` has shape `C×7×1` with `groups = C`, and
   `decoder.conv_in.layers.0.weight` has shape `64×7×1` with `groups = 64`. Every
   `res*.conv2` is pointwise (`C×1×C`, k=1). The encoder residual units are depthwise too
   (`depthwise=true` in the config).
6. **Snake `alpha` is stored `[1, 1, C]`** for channels-last broadcasting. For `[B, C, T]`
   activations, reshape it to `[1, C, 1]`.
7. **No tied embeddings and no LM head.** `base_lm.embed_tokens.weight` (73448×2048) is used
   only for input embedding. VoxCPM2 never predicts text tokens. The only output heads are
   `stop_head` (2-way) and the diffusion `out_proj`.
8. **`residual_lm`, `feat_encoder.encoder`, and `feat_decoder.estimator.decoder` have no
   `embed_tokens`** (vocab_size = 0). `residual_lm` has **no RoPE** (`residual_lm_no_rope = true`),
   so no `residual_lm.rope.*` keys exist.
9. **RoPE buffers are stored as F32 "weights"** (`*.rope.inv_freq`, `*.rope.short_factor`,
   `*.rope.long_factor`, each length 64) for `base_lm`, `feat_encoder.encoder`, and
   `feat_decoder.estimator.decoder`. They are deterministic: `inv_freq[j] = 10000^(-j/64)`
   (verified, max err 3e-8), and short_factor == long_factor (identical lists in config.json).
   Rust may recompute them or load them. Ignore them when you build a strict key→module mapping.
10. **`audio_vae.decoder._sr_boundaries`** (I32 `[20000, 30000, 40000]`) is a buffer, not a
    parameter. The bucket for 48 kHz output is **3**.
11. **Attention shapes (GQA, head_dim 128 everywhere).** `q_proj` maps to 16×128 = 2048 and `k_proj`/`v_proj` map to
    2×128 = 256 (2 KV heads, inherited from `lm_config` by LocEnc and LocDiT). For LocEnc and
    LocDiT (hidden 1024), q_proj is `2048×1024` and o_proj is `1024×2048`, so **the attention
    inner width (2048) is not the hidden size (1024)**.
12. **`feat_encoder.special_token`** is a learned `[1,1,1,1024]` BF16 CLS vector (not random at
    inference, despite the `randn` init in the Python constructors).
13. Key renames versus the OpenBMB PyTorch checkpoint: this MLX checkpoint already has
    readable VAE names (`encoder.conv_in`, `encoder.blocks.layers.N.{res1,res2,res3,snake,conv}`,
    `decoder.conv_in.layers.{0,1}`, `decoder.blocks.layers.N.{snake,conv_t,res1..3}`,
    `decoder.snake_out`, `decoder.conv_out`, `decoder.sr_cond_layers.N`). The PyTorch names are
    `encoder.block.{0..}`, `decoder.model.{0..}`, and `decoder.sr_cond_model.{2..7}`, with
    `block.{0..3}` inside residual units. The mapping is in mlx-audio `AudioVAE.sanitize`.
    Load *this* checkpoint and you can ignore the mapping.

## Parameter counts

| module | params |
|---|---|
| base_lm (TSLM, 28 layers + 73448×2048 embedding) | 1.472 B (embedding 150 M) |
| residual_lm (RALM, 8 layers) | 0.378 B |
| feat_encoder (LocEnc, 12 layers @1024) | 0.208 B |
| feat_decoder (LocDiT, 12 layers @1024 + time MLPs) | 0.212 B |
| projections + FSQ + stop | 0.021 B |
| audio_vae (f32) | 0.094 B |
| **total** | **2.384 B** |

## Tensor tables

### `base_lm`

| key | dtype | shape | count | shard(s) |
|---|---|---|---|---|
| `base_lm.embed_tokens.weight`  | BF16 | 73448×2048 | 1 | 1 |
| `base_lm.layers.{i}.input_layernorm.weight` i∈[0,27] | BF16 | 2048 | 28 | 1 |
| `base_lm.layers.{i}.mlp.down_proj.weight` i∈[0,27] | BF16 | 2048×6144 | 28 | 1 |
| `base_lm.layers.{i}.mlp.gate_proj.weight` i∈[0,27] | BF16 | 6144×2048 | 28 | 1 |
| `base_lm.layers.{i}.mlp.up_proj.weight` i∈[0,27] | BF16 | 6144×2048 | 28 | 1 |
| `base_lm.layers.{i}.post_attention_layernorm.weight` i∈[0,27] | BF16 | 2048 | 28 | 1 |
| `base_lm.layers.{i}.self_attn.k_proj.weight` i∈[0,27] | BF16 | 256×2048 | 28 | 1 |
| `base_lm.layers.{i}.self_attn.o_proj.weight` i∈[0,27] | BF16 | 2048×2048 | 28 | 1 |
| `base_lm.layers.{i}.self_attn.q_proj.weight` i∈[0,27] | BF16 | 2048×2048 | 28 | 1 |
| `base_lm.layers.{i}.self_attn.v_proj.weight` i∈[0,27] | BF16 | 256×2048 | 28 | 1 |
| `base_lm.norm.weight`  | BF16 | 2048 | 1 | 1 |
| `base_lm.rope.inv_freq`  | F32 | 64 | 1 | 1 |
| `base_lm.rope.long_factor`  | F32 | 64 | 1 | 1 |
| `base_lm.rope.short_factor`  | F32 | 64 | 1 | 1 |

Tensors in group: **257**


### `residual_lm`

| key | dtype | shape | count | shard(s) |
|---|---|---|---|---|
| `residual_lm.layers.{i}.input_layernorm.weight` i∈[0,7] | BF16 | 2048 | 8 | 1,2 |
| `residual_lm.layers.{i}.mlp.down_proj.weight` i∈[0,7] | BF16 | 2048×6144 | 8 | 1,2 |
| `residual_lm.layers.{i}.mlp.gate_proj.weight` i∈[0,7] | BF16 | 6144×2048 | 8 | 1,2 |
| `residual_lm.layers.{i}.mlp.up_proj.weight` i∈[0,7] | BF16 | 6144×2048 | 8 | 1,2 |
| `residual_lm.layers.{i}.post_attention_layernorm.weight` i∈[0,7] | BF16 | 2048 | 8 | 1,2 |
| `residual_lm.layers.{i}.self_attn.k_proj.weight` i∈[0,7] | BF16 | 256×2048 | 8 | 1,2 |
| `residual_lm.layers.{i}.self_attn.o_proj.weight` i∈[0,7] | BF16 | 2048×2048 | 8 | 1,2 |
| `residual_lm.layers.{i}.self_attn.q_proj.weight` i∈[0,7] | BF16 | 2048×2048 | 8 | 1,2 |
| `residual_lm.layers.{i}.self_attn.v_proj.weight` i∈[0,7] | BF16 | 256×2048 | 8 | 1,2 |
| `residual_lm.norm.weight`  | BF16 | 2048 | 1 | 2 |

Tensors in group: **73**


### `feat_encoder`

| key | dtype | shape | count | shard(s) |
|---|---|---|---|---|
| `feat_encoder.encoder.layers.{i}.input_layernorm.weight` i∈[0,11] | BF16 | 1024 | 12 | 1 |
| `feat_encoder.encoder.layers.{i}.mlp.down_proj.weight` i∈[0,11] | BF16 | 1024×4096 | 12 | 1 |
| `feat_encoder.encoder.layers.{i}.mlp.gate_proj.weight` i∈[0,11] | BF16 | 4096×1024 | 12 | 1 |
| `feat_encoder.encoder.layers.{i}.mlp.up_proj.weight` i∈[0,11] | BF16 | 4096×1024 | 12 | 1 |
| `feat_encoder.encoder.layers.{i}.post_attention_layernorm.weight` i∈[0,11] | BF16 | 1024 | 12 | 1 |
| `feat_encoder.encoder.layers.{i}.self_attn.k_proj.weight` i∈[0,11] | BF16 | 256×1024 | 12 | 1 |
| `feat_encoder.encoder.layers.{i}.self_attn.o_proj.weight` i∈[0,11] | BF16 | 1024×2048 | 12 | 1 |
| `feat_encoder.encoder.layers.{i}.self_attn.q_proj.weight` i∈[0,11] | BF16 | 2048×1024 | 12 | 1 |
| `feat_encoder.encoder.layers.{i}.self_attn.v_proj.weight` i∈[0,11] | BF16 | 256×1024 | 12 | 1 |
| `feat_encoder.encoder.norm.weight`  | BF16 | 1024 | 1 | 1 |
| `feat_encoder.encoder.rope.inv_freq`  | F32 | 64 | 1 | 1 |
| `feat_encoder.encoder.rope.long_factor`  | F32 | 64 | 1 | 1 |
| `feat_encoder.encoder.rope.short_factor`  | F32 | 64 | 1 | 1 |
| `feat_encoder.in_proj.bias`  | BF16 | 1024 | 1 | 1 |
| `feat_encoder.in_proj.weight`  | BF16 | 1024×64 | 1 | 1 |
| `feat_encoder.special_token`  | BF16 | 1×1×1×1024 | 1 | 1 |

Tensors in group: **115**


### `feat_decoder`

| key | dtype | shape | count | shard(s) |
|---|---|---|---|---|
| `feat_decoder.estimator.cond_proj.bias`  | BF16 | 1024 | 1 | 1 |
| `feat_decoder.estimator.cond_proj.weight`  | BF16 | 1024×64 | 1 | 1 |
| `feat_decoder.estimator.decoder.layers.{i}.input_layernorm.weight` i∈[0,11] | BF16 | 1024 | 12 | 1 |
| `feat_decoder.estimator.decoder.layers.{i}.mlp.down_proj.weight` i∈[0,11] | BF16 | 1024×4096 | 12 | 1 |
| `feat_decoder.estimator.decoder.layers.{i}.mlp.gate_proj.weight` i∈[0,11] | BF16 | 4096×1024 | 12 | 1 |
| `feat_decoder.estimator.decoder.layers.{i}.mlp.up_proj.weight` i∈[0,11] | BF16 | 4096×1024 | 12 | 1 |
| `feat_decoder.estimator.decoder.layers.{i}.post_attention_layernorm.weight` i∈[0,11] | BF16 | 1024 | 12 | 1 |
| `feat_decoder.estimator.decoder.layers.{i}.self_attn.k_proj.weight` i∈[0,11] | BF16 | 256×1024 | 12 | 1 |
| `feat_decoder.estimator.decoder.layers.{i}.self_attn.o_proj.weight` i∈[0,11] | BF16 | 1024×2048 | 12 | 1 |
| `feat_decoder.estimator.decoder.layers.{i}.self_attn.q_proj.weight` i∈[0,11] | BF16 | 2048×1024 | 12 | 1 |
| `feat_decoder.estimator.decoder.layers.{i}.self_attn.v_proj.weight` i∈[0,11] | BF16 | 256×1024 | 12 | 1 |
| `feat_decoder.estimator.decoder.norm.weight`  | BF16 | 1024 | 1 | 1 |
| `feat_decoder.estimator.decoder.rope.inv_freq`  | F32 | 64 | 1 | 1 |
| `feat_decoder.estimator.decoder.rope.long_factor`  | F32 | 64 | 1 | 1 |
| `feat_decoder.estimator.decoder.rope.short_factor`  | F32 | 64 | 1 | 1 |
| `feat_decoder.estimator.delta_time_mlp.linear_1.bias`  | BF16 | 1024 | 1 | 1 |
| `feat_decoder.estimator.delta_time_mlp.linear_1.weight`  | BF16 | 1024×1024 | 1 | 1 |
| `feat_decoder.estimator.delta_time_mlp.linear_2.bias`  | BF16 | 1024 | 1 | 1 |
| `feat_decoder.estimator.delta_time_mlp.linear_2.weight`  | BF16 | 1024×1024 | 1 | 1 |
| `feat_decoder.estimator.in_proj.bias`  | BF16 | 1024 | 1 | 1 |
| `feat_decoder.estimator.in_proj.weight`  | BF16 | 1024×64 | 1 | 1 |
| `feat_decoder.estimator.out_proj.bias`  | BF16 | 64 | 1 | 1 |
| `feat_decoder.estimator.out_proj.weight`  | BF16 | 64×1024 | 1 | 1 |
| `feat_decoder.estimator.time_mlp.linear_1.bias`  | BF16 | 1024 | 1 | 1 |
| `feat_decoder.estimator.time_mlp.linear_1.weight`  | BF16 | 1024×1024 | 1 | 1 |
| `feat_decoder.estimator.time_mlp.linear_2.bias`  | BF16 | 1024 | 1 | 1 |
| `feat_decoder.estimator.time_mlp.linear_2.weight`  | BF16 | 1024×1024 | 1 | 1 |

Tensors in group: **126**


### `projections / heads (top level)`

| key | dtype | shape | count | shard(s) |
|---|---|---|---|---|
| `enc_to_lm_proj.bias`  | BF16 | 2048 | 1 | 1 |
| `enc_to_lm_proj.weight`  | BF16 | 2048×1024 | 1 | 1 |
| `fsq_layer.in_proj.bias`  | BF16 | 512 | 1 | 1 |
| `fsq_layer.in_proj.weight`  | BF16 | 512×2048 | 1 | 1 |
| `fsq_layer.out_proj.bias`  | BF16 | 2048 | 1 | 1 |
| `fsq_layer.out_proj.weight`  | BF16 | 2048×512 | 1 | 1 |
| `fusion_concat_proj.bias`  | BF16 | 2048 | 1 | 1 |
| `fusion_concat_proj.weight`  | BF16 | 2048×4096 | 1 | 1 |
| `lm_to_dit_proj.bias`  | BF16 | 1024 | 1 | 1 |
| `lm_to_dit_proj.weight`  | BF16 | 1024×2048 | 1 | 1 |
| `res_to_dit_proj.bias`  | BF16 | 1024 | 1 | 1 |
| `res_to_dit_proj.weight`  | BF16 | 1024×2048 | 1 | 1 |
| `stop_head.weight`  | BF16 | 2×2048 | 1 | 2 |
| `stop_proj.bias`  | BF16 | 2048 | 1 | 2 |
| `stop_proj.weight`  | BF16 | 2048×2048 | 1 | 2 |

Tensors in group: **15**


### `audio_vae.encoder`

| key | dtype | shape | count | shard(s) |
|---|---|---|---|---|
| `audio_vae.encoder.blocks.layers.0.conv.bias`  | F32 | 256 | 1 | 1 |
| `audio_vae.encoder.blocks.layers.0.conv.weight`  | F32 | 256×4×128 | 1 | 1 |
| `audio_vae.encoder.blocks.layers.0.res{r}.conv1.bias` r∈{1,2,3} | F32 | 128 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.0.res{r}.conv1.weight` r∈{1,2,3} | F32 | 128×7×1 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.0.res{r}.conv2.bias` r∈{1,2,3} | F32 | 128 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.0.res{r}.conv2.weight` r∈{1,2,3} | F32 | 128×1×128 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.0.res{r}.snake1.alpha` r∈{1,2,3} | F32 | 1×1×128 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.0.res{r}.snake2.alpha` r∈{1,2,3} | F32 | 1×1×128 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.0.snake.alpha`  | F32 | 1×1×128 | 1 | 1 |
| `audio_vae.encoder.blocks.layers.1.conv.bias`  | F32 | 512 | 1 | 1 |
| `audio_vae.encoder.blocks.layers.1.conv.weight`  | F32 | 512×10×256 | 1 | 1 |
| `audio_vae.encoder.blocks.layers.1.res{r}.conv1.bias` r∈{1,2,3} | F32 | 256 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.1.res{r}.conv1.weight` r∈{1,2,3} | F32 | 256×7×1 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.1.res{r}.conv2.bias` r∈{1,2,3} | F32 | 256 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.1.res{r}.conv2.weight` r∈{1,2,3} | F32 | 256×1×256 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.1.res{r}.snake1.alpha` r∈{1,2,3} | F32 | 1×1×256 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.1.res{r}.snake2.alpha` r∈{1,2,3} | F32 | 1×1×256 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.1.snake.alpha`  | F32 | 1×1×256 | 1 | 1 |
| `audio_vae.encoder.blocks.layers.2.conv.bias`  | F32 | 1024 | 1 | 1 |
| `audio_vae.encoder.blocks.layers.2.conv.weight`  | F32 | 1024×16×512 | 1 | 1 |
| `audio_vae.encoder.blocks.layers.2.res{r}.conv1.bias` r∈{1,2,3} | F32 | 512 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.2.res{r}.conv1.weight` r∈{1,2,3} | F32 | 512×7×1 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.2.res{r}.conv2.bias` r∈{1,2,3} | F32 | 512 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.2.res{r}.conv2.weight` r∈{1,2,3} | F32 | 512×1×512 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.2.res{r}.snake1.alpha` r∈{1,2,3} | F32 | 1×1×512 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.2.res{r}.snake2.alpha` r∈{1,2,3} | F32 | 1×1×512 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.2.snake.alpha`  | F32 | 1×1×512 | 1 | 1 |
| `audio_vae.encoder.blocks.layers.3.conv.bias`  | F32 | 2048 | 1 | 1 |
| `audio_vae.encoder.blocks.layers.3.conv.weight`  | F32 | 2048×16×1024 | 1 | 1 |
| `audio_vae.encoder.blocks.layers.3.res{r}.conv1.bias` r∈{1,2,3} | F32 | 1024 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.3.res{r}.conv1.weight` r∈{1,2,3} | F32 | 1024×7×1 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.3.res{r}.conv2.bias` r∈{1,2,3} | F32 | 1024 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.3.res{r}.conv2.weight` r∈{1,2,3} | F32 | 1024×1×1024 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.3.res{r}.snake1.alpha` r∈{1,2,3} | F32 | 1×1×1024 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.3.res{r}.snake2.alpha` r∈{1,2,3} | F32 | 1×1×1024 | 3 | 1 |
| `audio_vae.encoder.blocks.layers.3.snake.alpha`  | F32 | 1×1×1024 | 1 | 1 |
| `audio_vae.encoder.conv_in.bias`  | F32 | 128 | 1 | 1 |
| `audio_vae.encoder.conv_in.weight`  | F32 | 128×7×1 | 1 | 1 |
| `audio_vae.encoder.fc_mu.bias`  | F32 | 64 | 1 | 1 |
| `audio_vae.encoder.fc_mu.weight`  | F32 | 64×3×2048 | 1 | 1 |

Tensors in group: **88**


### `audio_vae.decoder`

| key | dtype | shape | count | shard(s) |
|---|---|---|---|---|
| `audio_vae.decoder._sr_boundaries`  | I32 | 3 | 1 | 1 |
| `audio_vae.decoder.blocks.layers.0.conv_t.bias`  | F32 | 1024 | 1 | 1 |
| `audio_vae.decoder.blocks.layers.0.conv_t.weight`  | F32 | 1024×16×2048 | 1 | 1 |
| `audio_vae.decoder.blocks.layers.0.res{r}.conv1.bias` r∈{1,2,3} | F32 | 1024 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.0.res{r}.conv1.weight` r∈{1,2,3} | F32 | 1024×7×1 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.0.res{r}.conv2.bias` r∈{1,2,3} | F32 | 1024 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.0.res{r}.conv2.weight` r∈{1,2,3} | F32 | 1024×1×1024 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.0.res{r}.snake1.alpha` r∈{1,2,3} | F32 | 1×1×1024 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.0.res{r}.snake2.alpha` r∈{1,2,3} | F32 | 1×1×1024 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.0.snake.alpha`  | F32 | 1×1×2048 | 1 | 1 |
| `audio_vae.decoder.blocks.layers.1.conv_t.bias`  | F32 | 512 | 1 | 1 |
| `audio_vae.decoder.blocks.layers.1.conv_t.weight`  | F32 | 512×12×1024 | 1 | 1 |
| `audio_vae.decoder.blocks.layers.1.res{r}.conv1.bias` r∈{1,2,3} | F32 | 512 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.1.res{r}.conv1.weight` r∈{1,2,3} | F32 | 512×7×1 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.1.res{r}.conv2.bias` r∈{1,2,3} | F32 | 512 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.1.res{r}.conv2.weight` r∈{1,2,3} | F32 | 512×1×512 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.1.res{r}.snake1.alpha` r∈{1,2,3} | F32 | 1×1×512 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.1.res{r}.snake2.alpha` r∈{1,2,3} | F32 | 1×1×512 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.1.snake.alpha`  | F32 | 1×1×1024 | 1 | 1 |
| `audio_vae.decoder.blocks.layers.2.conv_t.bias`  | F32 | 256 | 1 | 1 |
| `audio_vae.decoder.blocks.layers.2.conv_t.weight`  | F32 | 256×10×512 | 1 | 1 |
| `audio_vae.decoder.blocks.layers.2.res{r}.conv1.bias` r∈{1,2,3} | F32 | 256 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.2.res{r}.conv1.weight` r∈{1,2,3} | F32 | 256×7×1 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.2.res{r}.conv2.bias` r∈{1,2,3} | F32 | 256 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.2.res{r}.conv2.weight` r∈{1,2,3} | F32 | 256×1×256 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.2.res{r}.snake1.alpha` r∈{1,2,3} | F32 | 1×1×256 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.2.res{r}.snake2.alpha` r∈{1,2,3} | F32 | 1×1×256 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.2.snake.alpha`  | F32 | 1×1×512 | 1 | 1 |
| `audio_vae.decoder.blocks.layers.3.conv_t.bias`  | F32 | 128 | 1 | 1 |
| `audio_vae.decoder.blocks.layers.3.conv_t.weight`  | F32 | 128×4×256 | 1 | 1 |
| `audio_vae.decoder.blocks.layers.3.res{r}.conv1.bias` r∈{1,2,3} | F32 | 128 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.3.res{r}.conv1.weight` r∈{1,2,3} | F32 | 128×7×1 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.3.res{r}.conv2.bias` r∈{1,2,3} | F32 | 128 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.3.res{r}.conv2.weight` r∈{1,2,3} | F32 | 128×1×128 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.3.res{r}.snake1.alpha` r∈{1,2,3} | F32 | 1×1×128 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.3.res{r}.snake2.alpha` r∈{1,2,3} | F32 | 1×1×128 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.3.snake.alpha`  | F32 | 1×1×256 | 1 | 1 |
| `audio_vae.decoder.blocks.layers.4.conv_t.bias`  | F32 | 64 | 1 | 1 |
| `audio_vae.decoder.blocks.layers.4.conv_t.weight`  | F32 | 64×4×128 | 1 | 1 |
| `audio_vae.decoder.blocks.layers.4.res{r}.conv1.bias` r∈{1,2,3} | F32 | 64 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.4.res{r}.conv1.weight` r∈{1,2,3} | F32 | 64×7×1 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.4.res{r}.conv2.bias` r∈{1,2,3} | F32 | 64 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.4.res{r}.conv2.weight` r∈{1,2,3} | F32 | 64×1×64 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.4.res{r}.snake1.alpha` r∈{1,2,3} | F32 | 1×1×64 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.4.res{r}.snake2.alpha` r∈{1,2,3} | F32 | 1×1×64 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.4.snake.alpha`  | F32 | 1×1×128 | 1 | 1 |
| `audio_vae.decoder.blocks.layers.5.conv_t.bias`  | F32 | 32 | 1 | 1 |
| `audio_vae.decoder.blocks.layers.5.conv_t.weight`  | F32 | 32×4×64 | 1 | 1 |
| `audio_vae.decoder.blocks.layers.5.res{r}.conv1.bias` r∈{1,2,3} | F32 | 32 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.5.res{r}.conv1.weight` r∈{1,2,3} | F32 | 32×7×1 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.5.res{r}.conv2.bias` r∈{1,2,3} | F32 | 32 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.5.res{r}.conv2.weight` r∈{1,2,3} | F32 | 32×1×32 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.5.res{r}.snake1.alpha` r∈{1,2,3} | F32 | 1×1×32 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.5.res{r}.snake2.alpha` r∈{1,2,3} | F32 | 1×1×32 | 3 | 1 |
| `audio_vae.decoder.blocks.layers.5.snake.alpha`  | F32 | 1×1×64 | 1 | 1 |
| `audio_vae.decoder.conv_in.layers.0.bias`  | F32 | 64 | 1 | 1 |
| `audio_vae.decoder.conv_in.layers.0.weight`  | F32 | 64×7×1 | 1 | 1 |
| `audio_vae.decoder.conv_in.layers.1.bias`  | F32 | 2048 | 1 | 1 |
| `audio_vae.decoder.conv_in.layers.1.weight`  | F32 | 2048×1×64 | 1 | 1 |
| `audio_vae.decoder.conv_out.bias`  | F32 | 1 | 1 | 1 |
| `audio_vae.decoder.conv_out.weight`  | F32 | 1×7×32 | 1 | 1 |
| `audio_vae.decoder.snake_out.alpha`  | F32 | 1×1×32 | 1 | 1 |
| `audio_vae.decoder.sr_cond_layers.0.bias_embed.weight`  | F32 | 4×2048 | 1 | 1 |
| `audio_vae.decoder.sr_cond_layers.0.scale_embed.weight`  | F32 | 4×2048 | 1 | 1 |
| `audio_vae.decoder.sr_cond_layers.1.bias_embed.weight`  | F32 | 4×1024 | 1 | 1 |
| `audio_vae.decoder.sr_cond_layers.1.scale_embed.weight`  | F32 | 4×1024 | 1 | 1 |
| `audio_vae.decoder.sr_cond_layers.2.bias_embed.weight`  | F32 | 4×512 | 1 | 1 |
| `audio_vae.decoder.sr_cond_layers.2.scale_embed.weight`  | F32 | 4×512 | 1 | 1 |
| `audio_vae.decoder.sr_cond_layers.3.bias_embed.weight`  | F32 | 4×256 | 1 | 1 |
| `audio_vae.decoder.sr_cond_layers.3.scale_embed.weight`  | F32 | 4×256 | 1 | 1 |
| `audio_vae.decoder.sr_cond_layers.4.bias_embed.weight`  | F32 | 4×128 | 1 | 1 |
| `audio_vae.decoder.sr_cond_layers.4.scale_embed.weight`  | F32 | 4×128 | 1 | 1 |
| `audio_vae.decoder.sr_cond_layers.5.bias_embed.weight`  | F32 | 4×64 | 1 | 1 |
| `audio_vae.decoder.sr_cond_layers.5.scale_embed.weight`  | F32 | 4×64 | 1 | 1 |

Tensors in group: **146**


**Total: 820 tensors.**
