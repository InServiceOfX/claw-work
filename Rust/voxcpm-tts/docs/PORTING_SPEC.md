# VoxCPM2 → Rust/candle (Metal) porting specification

Status: authoritative reference for the native engine. Use it to implement and verify each
module without rereading the Python sources.

Sources read (pin these when you re-verify):

| source | revision | files |
|---|---|---|
| `Blaizzy/mlx-audio` | `ecd43b9` (2026-11-07), installed wheel **0.6.4** in `reference/.venv` | `mlx_audio/tts/models/voxcpm2/{config,minicpm,encoder,dit,audio_vae,voxcpm2}.py` |
| `OpenBMB/VoxCPM` (PyTorch original) | `4b8dd44` (2026-11-05) | `src/voxcpm/model/voxcpm2.py`, `modules/{minicpm4,locenc,locdit,audiovae/audio_vae_v2.py,layers}`, `core.py`, `utils/text_normalize.py`, `model/utils.py` |
| HF `mlx-community/VoxCPM2-bf16` | snapshot in the HF cache | `config.json`, `tokenizer.json`, `tokenizer_config.json`, `special_tokens_map.json`, `model*.safetensors` |
| docs | voxcpm.readthedocs.io (usage guide, cookbook, architecture) | — |

Companion docs: `docs/WEIGHTS_INVENTORY.md` (every tensor and its layout) and
`reference/dump_golden.py` (golden tensors; see §12).

Notation: `B` batch (always 1 at inference), `S` prefill sequence length, `P = 4` patch size,
`D = 64` latent dim, `H = 2048` LM hidden, `Hd = 1024` LocEnc/LocDiT hidden. Shapes are written
`[a, b, c]`. "PT" means the OpenBMB PyTorch code and "MLX" means mlx-audio.

---

## 1. Hyperparameters (this checkpoint)

### 1.1 Top level

| name | value | notes |
|---|---|---|
| `architecture` | `voxcpm2` | |
| `patch_size` P | 4 | latent frames per LM step |
| `feat_dim` D | 64 | VAE latent channels |
| `scalar_quantization_latent_dim` | 512 | FSQ bottleneck |
| `scalar_quantization_scale` | 9 | FSQ levels: round(tanh·9)/9 → 19 levels per dim |
| `residual_lm_num_layers` | 8 | RALM depth |
| `residual_lm_no_rope` | **true** | RALM has no positional encoding |
| `max_length` | 8192 | PT static KV-cache capacity (positions) |
| special token ids | `audio_start=101`, `audio_end=102`, `ref_audio_start=103` (`<|audio_prompt_start|>`), `ref_audio_end=104` | hard-coded in both implementations |

### 1.2 `lm_config` (TSLM / base_lm; the other transformer stacks inherit and override it)

| name | value |
|---|---|
| hidden_size | 2048 |
| intermediate_size | 6144 |
| num_hidden_layers | 28 |
| num_attention_heads | 16 |
| num_key_value_heads | 2 (GQA, 8 query heads per KV head) |
| kv_channels (head_dim) | 128 → q width 2048, kv width 256 |
| vocab_size | 73448 |
| rms_norm_eps | 1e-5 |
| rope_theta | 10000 |
| rope_scaling | `longrope`, `original_max_position_embeddings=32768`, `short_factor == long_factor` (64 floats, 0.9978 … 31.024; listed in config.json) |
| max_position_embeddings | 32768 |
| **use_mup** | **false** → **no** embedding scale (`scale_emb=12` is ignored), **no** residual scale (`scale_depth=1.4`/`sqrt(L)` is ignored), `dim_model_base=256` unused |
| bos/eos | 1 / 2 (unused at inference: no BOS is prepended) |

### 1.3 Derived stacks (`lm_config` with overrides)

| stack | layers | hidden | ffn | heads (q/kv) | head_dim | RoPE | attention |
|---|---|---|---|---|---|---|---|
| `base_lm` (TSLM) | 28 | 2048 | 6144 | 16/2 | 128 | LongRoPE | causal |
| `residual_lm` (RALM) | 8 | 2048 | 6144 | 16/2 | 128 | **none** | causal |
| `feat_encoder.encoder` (LocEnc) | 12 | 1024 | 4096 | 16/2 | 128 (q width 2048 ≠ hidden) | LongRoPE (pos 0..4) | **bidirectional** |
| `feat_decoder.estimator.decoder` (LocDiT) | 12 | 1024 | 4096 | 16/2 | 128 | LongRoPE (pos 0..10) | **bidirectional** |

All four stacks use the same `rms_norm_eps=1e-5`, `use_mup=false`, and RoPE tables.

### 1.4 CFM / DiT

| name | value |
|---|---|
| solver | Euler, `inference_timesteps` default **10** |
| `cfg_value` | default **2.0** (mlx-audio enforces ≥2.0 when called through `cfg_scale`; PT allows 1.0–3.0) |
| `mean_mode` | false → `dt` input to the DiT is always 0 |
| sway sampling coef | 1.0 |
| CFG-Zero* | on (`use_cfg_zero_star=True`): optimized scale and zero velocity for the first `max(1, int(0.04·(n+1)))` = 1 step |
| `sigma_min`, `t_scheduler` | 1e-6, `log-norm` (training only) |
| temperature | 1.0 |

### 1.5 AudioVAE V2

| name | value |
|---|---|
| encoder_dim | 128 |
| encoder_rates | [2, 5, 8, 8] → hop **640** samples @16 kHz → 25 Hz latents |
| latent_dim | 64 |
| decoder_dim | 2048 |
| decoder_rates | [8, 6, 5, 2, 2, 2] → hop **1920** samples @48 kHz → 25 Hz |
| depthwise | true |
| sample_rate (encode) | 16000 |
| out_sample_rate (decode) | 48000 |
| sr_bin_boundaries | [20000, 30000, 40000] → 4 buckets; 48 kHz → bucket **3** |
| cond_type | `scale_bias`, `cond_out_layer=false` |
| use_noise_block | false |

### 1.6 Rates (memorize these)

* 1 latent frame = 640 input samples @16k = 1920 output samples @48k = **40 ms**.
* 1 LM step = 1 patch = 4 frames = 2560 input samples = **7680 output samples = 160 ms** (6.25 Hz LM).
* Golden example: 20 text tokens produce 30 patches = 4.80 s. Typical English rate is about 1.5 patches per token.

---

## 2. Data flow overview

```
text ──tokenize──► ids ─► embed_tokens ─┐ (text positions)
                                         ├─► combined_embed [S,H] ─► TSLM(28L, causal) ─► h [S,H]
ref wav ─16k─► VAE enc ─► latents ─patch─► LocEnc ─► enc_to_lm_proj ─┘ (audio positions)
                                                     │
 h ─► (FSQ at audio positions) = enc_out ─► lm_hidden = enc_out[-1]
 concat(enc_out, audio_mask·feat_embed) ─► fusion_concat_proj ─► RALM(8L, causal, no rope) ─► residual_hidden = last

loop (one patch per iteration):
  mu = [lm_to_dit_proj(lm_hidden) ; res_to_dit_proj(residual_hidden)]          [2·Hd]
  pred = CFM-Euler(LocDiT, mu, cond = previous patch [D,P], noise z [D,P])    [P,D]
  curr_embed = enc_to_lm_proj(LocEnc(pred))                                    [H]
  stop = argmax(stop_head(silu(stop_proj(lm_hidden))))   (break if i > min_len and stop==1)
  lm_hidden = FSQ(TSLM.step(curr_embed))                  (KV cached)
  residual_hidden = RALM.step(fusion_concat_proj([lm_hidden ; curr_embed]))   (KV cached)
all patches ─► [T·P, D] ─► VAE decoder (sr bucket 3) ─► tanh ─► 48 kHz waveform
```

---

## 3. Text pipeline

### 3.1 Pre-normalization (PT `core.py::_generate`, applied always)

```
text = text.replace("\n", " ")
text = re.sub(r"\s+", " ", text)          # collapse whitespace runs, NOT stripped
```
Upstream rejects empty or whitespace-only text. Optional `normalize=True` (default **false**) runs
`TextNormalizer` (wetext zh/en TN + inflect number spelling + markdown/emoji cleanup).
That needs Python-only dependencies (`wetext`). For the Rust port, leave it out of scope at first.
A small Rust pass is enough: strip markdown and emoji, then spell out digits for English with a
number-to-words crate. The docs recommend it only when "numbers are being read digit by digit".

### 3.2 Tokenizer

* `tokenizer.json` is a HF *fast* tokenizer: `BPE`, `byte_fallback=true`, vocab 73440 + 11
  added tokens (73448 embedding rows). The normalizer is `Sequence[Prepend("▁"),
  Replace(" " → "▁")]`, with no pre-tokenizer. The post-processor adds `<s>` (id 1). **Do not
  use it**: VoxCPM2 calls `tokenizer.tokenize()` + `convert_tokens_to_ids()` and never adds BOS
  or EOS.
* **The Rust `tokenizers` crate loads `tokenizer.json` directly.** Use
  `tokenizer.encode(text, /*add_special_tokens=*/false)` and take `.get_tokens()` /
  `.get_ids()`. `reference/golden/tokens.json` shows it matches transformers for every case
  **except a leading space**. transformers' Llama "legacy" path drops one leading space
  (`" Hello"` → `["▁Hello"]`), while the raw tokenizer gives `["▁▁", "Hello"]`. Rule for Rust:
  **if the text starts with `' '`, remove exactly one leading space before encoding.** (§3.1
  collapses whitespace runs, so at most one leading space exists.)
* Special tokens 101–104 are **not** in `tokenizer.json`'s added_tokens list (they live only
  in `tokenizer_config.json`). If user text contains a literal `<|audio_start|>`, it is
  tokenized as plain text. Inject the ids numerically; never by string.

### 3.3 Multi-character CJK splitting (must replicate)

After tokenization, each token is post-processed as follows:

```
clean = token.replace("▁", "")
if clean in MULTICHAR_CJK:                 # PT rule (authoritative)
    emit each char of clean as its own token (the "▁" marker is DROPPED)
else:
    emit token
ids = [vocab[t] for t in tokens]           # convert_tokens_to_ids; missing → unk id 0
```
`MULTICHAR_CJK` = {vocab tokens `t` with `len(t) >= 2` and every char in U+4E00..U+9FFF}
(19,400 entries). Build it once from `tokenizer.json` `model.vocab`.
MLX uses a looser test: it splits whenever `clean` is ≥2 chars and all in U+4E00..U+9FFF,
whether or not it is in the vocab. The two rules differ only for `▁XY` tokens whose bare `XY` is
not in the vocab. **Follow PT.** Example: `"你好世界"` → `['▁','你','好','世','界']`.
`"Mixed 中文"` → `['▁Mixed','中','文']` (the space marker is lost, as in the reference).

### 3.4 Voice design / style control

There is no separate channel. The description is a plain-text prefix in parentheses:
`text = f"({description}){text}"` (MLX `instruct=` does exactly this). It works in zero-shot
mode (voice design) and with reference audio ("controllable cloning", e.g.
`"(slightly faster, cheerful tone)..."`). Non-verbal tags such as `[laughing]`, `[sigh]`, and
`[Uhm]` are plain text too. The upstream retry and max-length heuristics count the description
tokens as part of `target_text`.

---

## 4. Reference / prompt audio preprocessing

1. **Load and downmix to mono** (`mean` over channels). Accept wav, mp3, and flac (product
   requirement). Use `symphonia` (all formats) or `hound` (wav).
2. **Resample to 16 kHz.** PT uses `librosa.load(sr=16000)` (soxr_hq). MLX uses a Kaiser-windowed
   polyphase sinc (`kaiser_best`). Any high-quality band-limited resampler works (e.g. `rubato`
   `SincFixedIn`, sinc_len ≥ 128, Blackman-Harris). Golden `ref_audio_16k` lets you test the
   resampler with a loose metric (SNR ≥ 30 dB) and the VAE with exact input.
3. Apply **no amplitude normalization** in either implementation. Apply no trimming by default
   (`trim_silence_vad=False`). The optional VAD trim works like this: frame=2048, hop=512,
   threshold=`max|y|·10^(-35/20)` on frame RMS. Keep [first voiced frame·hop, (last+1)·hop+1536],
   then widen by 200 ms on each side.
4. **Pad to a multiple of `P·640 = 2560` samples** with zeros. Reference audio pads on the **right**.
   Continuation prompt audio pads on the **left** (so the last patch ends on real audio). The VAE
   `preprocess` would also right-pad to 640, but that is a no-op after this step.
5. **VAE encode** (§6.9) gives `[T', 64]` (channels-last), where `T' = len/640`. Reshape to
   `[T'/4, 4, 64]` = `[R, P, D]` patches. PT's `feat.view(64, -1, P).permute(1, 2, 0)` is the
   same patching on channels-first data: patch `r` holds frames `4r..4r+3`.

Docs recommend 5–30 s of clean single-speaker reference audio. The golden test_en.wav is 48 kHz,
5.28 s, which gives 84483 samples @16k, padded with 2557 zeros to 87040, so R = 34 patches.

---

## 5. Sequence layout (prefill)

Every mode builds four aligned length-`S` arrays: `text_token[S]` (i32), `audio_feat[S,P,D]`,
`text_mask[S]`, and `audio_mask[S]` (0/1, complementary). Text positions carry `audio_feat=0`.
Audio positions carry `text_token=0`. `ids` = §3 output of the text, and `T_txt = len(ids)+1`.

| mode | layout (positions left → right) |
|---|---|
| zero-shot / voice design | `ids…, 101` — all text |
| **reference (voice clone; product default)** | `103, [0]×R, 104, ids…, 101`. audio_feat: `0, ref_patches(R), 0, 0×T_txt`. text_mask `1,0×R,1,1×T_txt`. audio_mask the complement |
| continuation (prompt wav + its transcript) | `ids(prompt_text+target_text)…, 101, [0]×Rp`. audio_feat: `0×T_txt, prompt_patches(Rp)`. The sequence ends on audio |
| reference + continuation | `103,[0]×R,104, ids(prompt_text+text)…,101, [0]×Rp`. feats `0,ref,0, 0×T_txt, prompt` |

Golden (reference mode): `S = 57 = 1 + 34 + 1 + 20 + 1`.

The text of continuation modes is `prompt_text + target_text`, concatenated as raw strings before
tokenization.

---

## 6. Modules

### 6.1 Shared MiniCPM-4 transformer (one implementation, four configurations)

```
fn layer(x[B,L,h]) :
    r = x
    a = RMSNorm(x, w_in, eps=1e-5)
    q = a @ Wq^T  → [B,L,16,128]   k = a @ Wk^T → [B,L,2,128]   v = a @ Wv^T → [B,L,2,128]
    if rope: q,k = apply_rope(q,k, pos)          # rotate-half (NOT interleaved)
    (k,v) = concat(cache, (k,v)) along L           # if cached
    o = softmax(q·k^T / sqrt(128) + mask) · v      # GQA: kv head j serves q heads 8j..8j+7
    x = r + o.reshape(B,L,2048) @ Wo^T             # use_mup=false → plain residual
    r = x
    x = r + Wdown( silu(Wgate a2) * (Wup a2) ),  a2 = RMSNorm(x, w_post)
stack: h = embeddings; for l in layers: h = layer(h); out = RMSNorm(h, w_final)
```
* **RMSNorm**: `x · rsqrt(mean(x²) + 1e-5) · w`. PT computes the variance in f32 and casts back
  before multiplying by `w`. MLX `mx.fast.rms_norm` is equivalent. `candle_nn::ops::rms_norm`
  (fused Metal) is fine.
* **LongRoPE** (MiniCPM): `inv_freq[j] = 10000^(-2j/128)`, j=0..63. `factor = short_factor`
  (selected when the max position ≤ 32768, so always here; long == short anyway).
  `freqs[p,j] = p / factor[j] · inv_freq[j]` (compute in f32). `emb = concat(freqs, freqs)`
  → `[L,128]`. `cos = cos(emb)·sf`, `sin = sin(emb)·sf`, where
  `sf = sqrt(1 + ln(32768/32768)/ln(32768)) = 1.0`.
  Apply as `q' = q·cos + rotate_half(q)·sin`, with `rotate_half(x) = concat(-x[64:], x[:64])`.
  This is the **non-interleaved** form = `candle_nn::rotary_emb::rope` with half-width cos/sin
  `[L,64]` (= `cos(freqs)`). PT applies RoPE in f32 and casts back.
* **Positions**: TSLM prefill uses 0..S-1. Generation step `i` (0-based) feeds position `S+i`.
  LocEnc and LocDiT always use 0..L-1 on their short sequences (5 and 11). RALM has none.
* **Masks**: TSLM and RALM prefill are causal (upper triangle −inf). Steps with L=1 need no mask
  (they attend to the whole cache). LocEnc and LocDiT have **no mask** (full bidirectional).
* **Final norm**: always applied to the stack output (all four stacks). The KV cache stores
  post-RoPE `k` and raw `v`.
* Numerics observed: the TSLM residual stream reaches **|x| ≈ 1.8·10⁴** at layer 27 (massive
  activations; RMS 288). bf16 and f32 handle the range. **Do not use f16** (65504 max, and
  precision loss).

### 6.2 TSLM (base_lm) prefill

```
text_embed   = embed_tokens[text_token] · 1.0           # [S,H]  (use_mup=false → scale 1)
feat_embed   = enc_to_lm_proj(LocEnc(audio_feat))       # [S,H]  (LocEnc runs on ALL S patches, zeros included)
combined     = text_mask[:,None]·text_embed + audio_mask[:,None]·feat_embed
h            = TSLM(combined, causal)                   # [S,H] after final norm
enc_out      = audio_mask·FSQ(h) + text_mask·h          # FSQ only at audio positions
lm_hidden    = enc_out[S-1]                             # = h[S-1] unless the sequence ends on audio (continuation)
```

### 6.3 FSQ (`fsq_layer`, ScalarQuantizationLayer, inference path)

`y = out_proj( round( tanh( in_proj(x) ) · 9 ) / 9 )`, with in_proj 2048→512 (+b) and
out_proj 512→2048 (+b). Rounding is half-to-even in torch/MLX. Metal `round()` rounds half away
from zero. Exact .5 ties essentially never occur, but use `rint` semantics if you write a
kernel. **Sensitivity**: in the golden run, 36 of 14848 pre-round values lie within 1e-3 of a
rounding boundary. A tiny upstream difference can flip a level (±1/9), and the AR trajectory
then diverges. That is expected, so test with teacher forcing (§11).

### 6.4 RALM (residual_lm) and fusion

```
prefill:  res_in = fusion_concat_proj( concat(enc_out, audio_mask·feat_embed) )   # [S,4096]→[S,2048]
          residual_hidden = RALM(res_in, causal)[S-1]
step:     res_in = fusion_concat_proj( concat(lm_hidden_new, curr_embed) )          # [1,4096]
          residual_hidden = RALM.step(res_in)                                        # KV cached, no RoPE
```
`fusion_concat_proj`: Linear 4096→2048 with bias. Concatenation order is **[LM part, LocEnc part]**.

### 6.5 LocEnc (feat_encoder)

Input `[B,T,P,D]`. For every patch independently:
```
x   = in_proj(patch)                      # [P,Hd] = Linear(64→1024,+b) per frame
seq = concat(special_token[1,Hd], x)      # [5,Hd]  CLS first
out = LocEncStack(seq, no mask, rope pos 0..4)[0]     # CLS output after final norm  → [Hd]
```
Batch all patches as `[B·T, 5, Hd]`. The caller applies `enc_to_lm_proj` (Linear 1024→2048, +b).

### 6.6 LocDiT (feat_decoder.estimator, VoxCPMLocDiTV2)

Inputs: `x [N,64,4]` (noisy patch), `mu [N,2048]`, `t [N]`, `cond [N,64,4]` (previous patch),
`dt [N]` (always 0). Here N = 2 (CFG batch).
```
xp   = in_proj(x^T)            # [N,4,1024]   (Linear 64→1024, +b)
cp   = cond_proj(cond^T)       # [N,4,1024]
temb = time_mlp( sinus(t) ) + delta_time_mlp( sinus(dt) )      # [N,1024]
       sinus(t): half=512, f_j = exp(-j·ln(10000)/511), j=0..511; a = 1000·t·f_j;  [sin(a) , cos(a)]
       TimestepEmbedding: Linear(1024→1024,+b) → SiLU → Linear(1024→1024,+b)
mu_t = mu.reshape(N, 2, 1024)          # token 0 = lm_to_dit_proj(lm_hidden), token 1 = res_to_dit_proj(residual_hidden)
seq  = concat(mu_t[2], temb[1], cp[4], xp[4])   # [N,11,1024]
h    = DiTStack(seq, no mask, rope pos 0..10)   # after final norm
v    = out_proj(h[:, 7:11]) → transpose → [N,64,4]     (Linear 1024→64, +b)
```
`sinus(0)` = [0×512, 1×512], so `delta_time_mlp(sinus(0))` is a constant vector. Precompute it once.

### 6.7 CFM sampler (UnifiedCFM.sample + solve_euler)

```
z      ~ N(0,1) [1,64,4] · temperature(1.0)                 # ONE draw per AR step (the only randomness)
s_k    = 1 - k/n,  k=0..n          (n = inference_timesteps = 10)
t_k    = s_k + (cos(π/2·s_k) - 1 + s_k)                     # sway sampling, coef 1
       = [1, .956434, .909017, .853990, .787785, .707107, .609017, .491007, .351057, .187688, 0]
x = z; t = t_0; dt = t_0 - t_1
for step in 1..=n:
    if step <= 1:                     # zero_init_steps = max(1, int(0.04·(n+1))) = 1 for n ≤ 24
        v = 0
    else:
        out = DiT([x;x], [mu;0], [t;t], [cond;cond], [0;0])     # batch 2: cond / uncond (mu zeroed)
        vp, vn = out[0], out[1]
        st = <vp,vn> / (||vn||² + 1e-8)                        # over all 256 elements (CFG-Zero*)
        v  = vn·st + cfg·(vp - vn·st)
    x = x - dt·v ;  t = t - dt
    if step < n: dt = t - t_{step+1}
return x        # [1,64,4] → transpose → pred_feat [1,4,64]
```
That is 9 DiT evaluations (batch 2) per AR step at n=10. `t` is the scalar timestep fed to
`sinus` (scaled by 1000 inside). PT builds `t_span` in the model dtype (bf16 on CUDA). MLX uses
f32. Use f32 for the schedule.

### 6.8 Stop predictor

`logits = stop_head( silu( stop_proj(lm_hidden) ) )`. stop_proj: 2048→2048 (+b).
stop_head: 2048→2 (no bias). `stop = argmax(logits)` (1 = stop). It is evaluated on the
`lm_hidden` that **produced the current patch** (§7). Golden logits run from (25.5, −32.7) down
to (−0.31, 0.13) at the final step.

### 6.9 AudioVAE V2 encoder (16 kHz → 25 Hz × 64)

Building blocks. Lay activations out `[B,C,T]` in candle and permute MLX weights (see the
inventory). All encoder and decoder weights are f32. **Keep the VAE in f32** (PT forces it too).

* `Snake(x) = x + (1/(α+1e-9)) · sin(α·x)²`, with α per channel.
* `CausalConv1d(k, stride s, dilation d, groups g, padding p, output_padding op)`: left zero-pad
  by `2p − op`, then a valid conv. The PyTorch semantics are authoritative (see §9.1).
* `ResidualUnit(C, d)`: `y = conv1x1(Snake(dwconv7_d(Snake(x))))` with dwconv7_d = k=7,
  dilation d, groups=C, left pad 6d. Then `x + y`.

```
x [1,1,L]  (L multiple of 640)
conv_in: CausalConv1d(1→128, k=7, pad=3)  (left pad 6)
for (Cin, s) in [(128,2),(256,5),(512,8),(1024,8)]:
    x = Res(Cin,1); Res(Cin,3); Res(Cin,9); Snake(Cin)
    x = CausalConv1d(Cin→2Cin, k=2s, stride=s, padding=ceil(s/2), output_padding=s%2)
        left pad = 2·ceil(s/2) − s%2  = 2, 5, 8, 8
fc_mu: CausalConv1d(2048→64, k=3, pad=1) (left pad 2)          → [1,64,L/640]
```
Lengths: L → L → L/2 → L/10 → L/80 → L/640. Encoder output is `mu` (no sampling, no logvar).

### 6.10 AudioVAE V2 decoder (25 Hz × 64 → 48 kHz)

```
z [1,64,T]
conv_in: dwconv(64, k=7, groups=64, left pad 6) → conv1x1(64→2048)
bucket = #{b in [20000,30000,40000] : 48000 >= b} = 3
for i, s in enumerate([8,6,5,2,2,2]):  Cin = 2048/2^i, Cout = Cin/2
    x = x · scale_embed_i[bucket][:,None] + bias_embed_i[bucket][:,None]      # SR conditioning, on block INPUT
    x = Snake(Cin)(x)
    x = ConvTranspose1d(Cin→Cout, k=2s, stride=s, padding=0)(x)              # length (T−1)s + 2s
    x = x[..., : len − (2·ceil(s/2) − s%2)]                                  # causal trim from the RIGHT → T·s
    x = Res(Cout,1); Res(Cout,3); Res(Cout,9)                                 # depthwise, groups=Cout
x = Snake(32); CausalConv1d(32→1, k=7, left pad 6); tanh
```
Output length is exactly `T · 1920` samples. The sr-conditioning tables are trained, not
identity (layer-0 scale ≈ 0.77). Do not skip them.
Channel ladder: 2048 →(×8) 1024 →(×6) 512 →(×5) 256 →(×2) 128 →(×2) 64 →(×2) 32 → 1.

---

## 7. Generation loop (exact order of operations)

```
prefill (§6.2, §6.4) → lm_hidden, residual_hidden, KV caches (TSLM S positions, RALM S positions)
prefix_cond = audio_feat[S-1]                 # [P,D]; zeros unless the sequence ends on prompt audio
pred_seq = []
if continuation (audio_mask[S-1]==1):
    ctx = min(streaming_prefix_len−1 = 3, #audio positions);  pred_seq = last ctx prompt patches
for i in 0..max_len:
    mu   = concat(lm_to_dit_proj(lm_hidden), res_to_dit_proj(residual_hidden))   # [2048]
    pred = CFM(mu, cond = prefix_cond^T, z_i)                                     # [P,D]
    curr = enc_to_lm_proj(LocEnc(pred))                                            # [H]
    pred_seq.push(pred)
    stop = argmax(stop_head(silu(stop_proj(lm_hidden))))
    if i > min_len and stop == 1: break            # the patch from THIS iteration is kept
    h = TSLM.step(curr, pos = S+i)                 # final-normed
    lm_hidden = FSQ(h)                             # FSQ applied to every generated position
    residual_hidden = RALM.step(fusion_concat_proj(concat(lm_hidden, curr)))
    prefix_cond = pred
latents = concat(pred_seq) → [len·P, D];  wav = VAE.decode(latents)
if continuation: wav = wav[ctx·7680 :]          # drop the re-decoded prompt context
```
* **Min length**: `min_len=2` → the earliest break is i=3, so at least 4 patches (0.64 s).
* **Max length**: MLX uses `max_tokens=2000` (≈320 s). PT caps at
  **`min(int(len(target_ids)·6.0 + 10), max_len)`** (with `max_len` 4096 in `VoxCPM.generate`).
  This cap is always on, even without retry. Golden: 20 tokens → cap 130. **Use the PT cap.**
* **KV cache**: TSLM keeps 28 layers × (k,v) `[2 heads, pos, 128]`. RALM keeps 8 layers. Each
  step appends one position. Max positions = S + max_len (PT static capacity 8192; RoPE valid to
  32768). Memory: TSLM 28·2·2·128 = 14336 values/position (57 KB f32).
* **Randomness**: only `z_i` (one `[64,4]` standard normal per AR step, drawn before the Euler
  loop). VAE NoiseBlock is off. LocEnc's `special_token` is a loaded weight. Golden runs
  dump every `z_i`, so Rust takes an injectable `NoiseSource` (golden replay or seeded RNG).
  The MLX RNG (threefry) does not need to be reproduced.
* **Batch**: B=1, with the CFG pair batched inside the DiT (N=2).
* **Streaming** (optional): PT's `StreamingVAEDecoder` decodes patch by patch. Each causal conv
  keeps its last `2p − op` input samples. Each transposed conv keeps 1 previous input frame
  (`(K−1)//s = 1`), drops `s` leading output samples, and trims the causal tail. This is exactly
  equivalent to a full decode. It also bounds memory for long outputs (§10).

---

## 8. Long text, retries, chunking

Upstream facts:
* The model code has **no** internal text chunking. The usage guide says long text "is one of
  the easiest ways to trigger unstable behavior (gradual speed-up or buzzing, OOM from KV growth,
  generations that never stop)". The recommended pattern is to **split into segments, generate
  each independently with the same `reference_wav_path`, and `np.concatenate`**.
* `utils/text_normalize.split_paragraph` (from CosyVoice) exists but is never called. It splits
  on `。？！；：、.?!;` (zh) or `.?!;:` (en), with an optional comma split. It packs sentences
  greedily up to `token_max_n=80` once a chunk exceeds `token_min_n=60`, and merges a trailing
  chunk shorter than `merge_len=20` into the previous one. zh length = characters; en length =
  tokens. A closing quote right after punctuation sticks to the sentence.
* **retry_badcase** (`VoxCPM.generate` default **true**, max 3 tries, ratio 6.0): after
  generation, if `n_patches ≥ 6.0 · len(target_ids)`, retry with `seed+1` (a new noise stream).
  With the `6·len+10` cap, a run that hits the cap counts as a bad case (a babble that never
  stops). Streaming mode disables retries.
* `merge_prompt_cache(original, new_text, new_audio_feat)` appends generated latents and text to
  the prompt cache (a continuation context) "to stabilize voice". The context grows without bound.
* CFG guidance from the docs: 2.0 default; 1.5–1.6 "help stabilize noisy long-form output".
  Timesteps 4–30.

**Recommended Rust policy (product):**
1. Normalize whitespace (§3.1). Split into sentences on the punctuation sets above (and `…`,
   `！？` for zh). Pack them into chunks of ≤ 80 tokens (min 60, merge tail < 20). Hard-split any
   single sentence over 80 tokens at commas. This is the CosyVoice/VoxCPM recipe.
2. Generate each chunk independently in **reference mode with the same reference latents**.
   Encode the reference once and reuse the patches and their LocEnc embeddings. Each chunk gets
   a fresh prefill and KV cache. Use the PT max-length cap per chunk, and retry with seed+1 when
   `n ≥ 6·len` (up to 3 tries).
3. Stitch at 48 kHz with a short equal-power crossfade (≈10–20 ms) or a fixed inter-sentence
   pause (≈100–200 ms). Then delete the per-chunk files (product requirement).
4. Optional quality mode, later: pass the previous chunk's last few generated patches (plus their
   text) as a continuation prompt along with the reference ("ref_continuation"). That gives better
   prosodic continuity. Cap it at 1 previous chunk to avoid drift.
5. ETA: expect ≈1.5 patches per text token (cap 6). Measure per-step latency during the first
   chunk and extrapolate.

---

## 9. Reference discrepancies (MLX vs PyTorch): what Rust must do

| # | item | PyTorch (authoritative) | mlx-audio 0.6.4 | Rust | impact (measured) |
|---|---|---|---|---|---|
| 1 | VAE **encoder** stride-5 downsample left pad | `2·ceil(5/2) − 1 = 5` | `2·ceil(5/2) = 6` (ignores `output_padding`) | **5** | ref latents differ by up to **1.57** abs (absmax 5.05). dump_golden.py patches MLX to PT semantics by default |
| 2 | VAE **decoder** stride-5 ConvTranspose | output length `5T` | `output_padding=1` applied twice → `5T+1`, so the waveform is **8 samples too long** | `T·1920` samples | the first `T·1920` samples are bit-identical (verified). Compare `golden.wav_48k[:T·1920]` |
| 3 | CJK multi-char split | split only if bare token ∈ vocab multichar set | split any all-CJK ≥2-char token | PT rule | rare |
| 4 | max length | `min(int(6·len+10), max_len)` + retry | 2000 patches, no retry | PT | babbling protection |
| 5 | resampler | librosa/soxr_hq | Kaiser polyphase | any HQ sinc | tiny |
| 6 | dtypes | bf16 on CUDA; **forced f32 on MPS** ("bf16 drift in the AR loop gives glitched output and endless retries") | f32 activations (bf16 weights promoted) | see §10.2 | — |
| 7 | t_span dtype | model dtype | f32 | f32 | tiny |
| 8 | leading space tokenization | (transformers 4.x fast) | transformers 5.x drops 1 leading space | drop 1 leading space | edge |

---

## 10. candle / Metal implementation notes and risks

### 10.1 Op support (candle 0.11.1, `cfcbd71`)

| need | candle status | recommendation |
|---|---|---|
| Linear (bf16/f32) | `candle_nn::Linear`, Metal GEMM (MLX-derived) | fine |
| RMSNorm | `candle_nn::ops::rms_norm` fused Metal | fine (eps 1e-5) |
| RoPE (rotate-half) | `candle_nn::rotary_emb::rope` (`[B,H,T,D]`, cos/sin `[T,D/2]`) | fine. Precompute tables for pos 0..max |
| SDPA | `candle_nn::ops::sdpa` Metal: GQA ok, `seq=1` vector kernel, masks ok for seq>1, `do_causal` flag | first write the naive path (repeat_kv + matmul + softmax_last_dim) as the correctness oracle, then switch to fused sdpa |
| SiLU, tanh, sin, cos, exp, argmax, embedding | yes | fine |
| `round` | Metal `round()` = half away from zero | fine (ties never occur in practice) |
| Conv1d, dilation | Metal im2col1d + GEMM, dilation supported | fine for dense convs (k=1, conv_in, downsampling, fc_mu) |
| **Depthwise conv1d (groups=C up to 1024)** | `conv1d` with groups>1 **chunks into C separate convs**, which is catastrophically slow | **custom op**: `y = Σ_{k=0..6} w[:,k] ⊙ x[:, t − (6−k)·d]` on the left-padded input (7 `narrow` + broadcast-mul + add, or a small Metal kernel). There are 31 depthwise convs: 12 encoder + 18 decoder residual units + decoder conv_in |
| ConvTranspose1d | fast col2im (GEMM + overlap-add) path **only if padding=0, output_padding=0, dilation=1** | always call with padding 0 and do the causal right-trim manually (§6.10) |
| im2col memory | k=7 convs on long 48 kHz signals expand ×7 | decode in streaming chunks (§7) for long outputs, or chunk along T with state carry |
| Mixed-dtype matmul | **not supported** (no bf16×f32 promotion, unlike MLX) | choose one compute dtype per stack, see 10.2 |
| RNG | `Tensor::randn` on Metal | wrap behind a `NoiseSource` trait; tests inject golden noise |

### 10.2 Precision plan

* **Verification build**: load every weight as **f32**. That means ≈9.5 GB of RAM, which the
  target Macs have. It matches the golden policy (bf16 weights upcast exactly, f32 activations).
  Use it for the tight tolerances in §11.
* **Production**: (a) f32 everywhere (the safe default, matches MLX 0.6.4 numerics), or (b) bf16
  weights and activations for the TSLM, RALM, LocEnc, and DiT, with RMSNorm, softmax, and RoPE in
  f32. The VAE always runs in f32. Measured: TSLM prefill in bf16 vs f32 gives max abs 0.031 and
  mean abs 4e-4 on the final-norm output (RMS 1.56). Before you adopt (b), A/B the stop length and
  listen, given the PT warning about bf16 on Apple GPUs (§9 row 6).

### 10.3 Performance budget per AR step

Each step runs 1 TSLM token step (28 layers) + 1 RALM step (8 layers) + LocEnc on 1 patch
(12 layers, seq 5) + 9 DiT evals (12 layers, seq 11, batch 2). The DiT dominates the FLOP count
(MLX profiling: "54% of step time"). Real time requires < 160 ms per step. Kernel-launch
overhead dominates at these tiny shapes, so fuse where possible: precompute the constant
`delta_time_mlp` output, keep the CFG pair batched, and avoid host syncs except the stop
`argmax`. The prefill is a one-off.

---

## 11. Suggested Rust decomposition and numerical test points

```
crates/voxcpm-core (lib)
  config.rs        serde structs for config.json (§1)
  weights.rs       safetensors loading (mmap, both shards), MLX→candle conv permutes, dtype policy
  text/tokenize.rs tokenizers-crate wrapper + leading-space rule + CJK split (§3)
  text/segment.rs  sentence split / chunk packing (§8)
  audio/io.rs      wav/mp3/flac decode (symphonia), mono, resample to 16k (rubato), wav write
  nn/minicpm.rs    RMSNorm, LongRoPE tables, Attention(GQA, cache), MLP, Stack{causal|bidir, rope?}
  nn/locenc.rs     LocEnc
  nn/locdit.rs     SinusoidalPosEmb, TimestepEmbedding, LocDiTV2
  nn/cfm.rs        Euler + CFG-Zero* + sway schedule, NoiseSource
  nn/fsq.rs        FSQ
  vae/{snake,conv,encoder,decoder}.rs   depthwise op, causal conv, convT+trim, SR cond, streaming state
  model.rs         VoxCPM2: prefill builders per mode (§5), step loop (§7), stop, caps, retry
crates/voxcpm-cli  CLI: chunking, progress/ETA, stitching, cleanup
```

Test points use the golden files (§12). Metric `rel = ‖a−b‖₂ / ‖b‖₂`. Unless stated otherwise,
tolerances are for the **f32 build**, with bf16-build tolerances in brackets.

| # | test | inputs (golden) | compare | tol |
|---|---|---|---|---|
| T1 | tokenizer | `tokens.json`, `meta.text_ids` | `ids_final` exact | exact |
| T2 | RoPE tables | — | `04:rope_cos/sin`, `03:patch0_rope_cos/sin` | 1e-6 abs |
| T3 | VAE encoder unit | `01:input` | every `blockN_*_out`, `fc_mu_out` (transpose to `[C,T]`) | rel 1e-5 |
| T4 | VAE encoder full | `00:ref_audio_16k_padded` | `02:ref_latents` | rel 1e-5 |
| T5 | resampler | `00:ref_audio_raw` (48k) | `00:ref_audio_16k` | SNR ≥ 30 dB |
| T6 | LocEnc | `03:input` `[57,4,64]` | `patch0_seq_in`, `patch0_layerXX_out`, `locenc_out`, `enc_to_lm_out` | rel 1e-5 [2e-2] |
| T7 | TSLM prefill | `04:combined_embed` | `layerXX_out` (rel; huge magnitudes), `final_norm_out`, `k/v_cache_layer0` | rel 1e-4 [2e-2] |
| T8 | FSQ + mix | `04:final_norm_out`, masks | `fsq_out_all_positions`, `enc_outputs_mixed`, `lm_hidden0`, `fsq_last_quant` | rel 1e-4; quant codes equal except ≤ 1e-3-from-boundary |
| T9 | RALM prefill | `05:residual_input` (also rebuild it from T8) | `layerXX_out`, `final_norm_out`, `residual_hidden0` | rel 1e-4 [2e-2] |
| T10 | DiT single eval | `07:step0_first_eval_seq_in` → stack; and from raw (`x_per_step[1]`, mu, t, cond) | `seq_in`, `seq_out_normed`, `velocity`, `t_sinusoid`, `t_emb`, `dt_emb` | rel 1e-5 [2e-2] |
| T11 | CFM sample | `06:mu[i]`, `06:cond[i]`, `06:noise[i]` | `07:stepN_x_per_step`, `v_pos`, `v_neg`, `st_star`, `06:pred_feat[i]` | rel 1e-4 [3e-2] |
| T12 | LocEnc on generated patch | `06:pred_feat[i]` | `06:curr_embed[i]` | rel 1e-5 |
| T13 | stop head | `06:lm_hidden[i]` | `06:stop_logits[i]` (argmax equal) | 1e-3 abs |
| T14 | TSLM/RALM step, **teacher-forced** | prefill caches + golden `curr_embed[0..i]` | `06:lm_step_out_pre_fsq[i]`, `06:lm_hidden[i+1]`, `06:residual_hidden[i+1]` | rel 1e-4 [2e-2] |
| T15 | VAE decoder unit | `08:input` `[8,64]` | `conv_in_out`, `blockN_srcond_out`, `blockN_conv_t_out`, `blockN_out`, `wav`. **Note**: MLX block2+ are 1/2/4/8 samples long (§9 row 2), so compare prefixes `[:8·s_cum]` | rel 1e-5 |
| T16 | VAE decode full | `09:latents` | `09:wav_48k[:120·1920]` | max abs 1e-4 |
| T17 | streaming decode | `09:latents` patch by patch | full-decode output | max abs 1e-5 |
| T18 | end-to-end, injected noise | inputs + `06:noise` | step count = `meta.n_ar_steps` (30), `pred_feat` per step | first ~5 steps rel 1e-3. Later steps may diverge via FSQ flips (accept if T14 passes) |
| T19 | end-to-end smoke | any text | audio duration plausible, stop reached before the cap, listening test | — |

---

## 12. Golden reference dumps

Script: `reference/dump_golden.py` (uses the shared `reference/.venv`: uv, Python 3.13,
mlx-audio 0.6.4, mlx 0.32.4). Output goes to `reference/golden/` (gitignored, ≈80 MB).

```
cd Rust/voxcpm-tts/reference
.venv/bin/python dump_golden.py                       # reference mode, test_en.wav, seed 1234 (~90 s on M5)
.venv/bin/python dump_golden.py --no-ref --out golden_zeroshot
.venv/bin/python dump_golden.py --vae-encoder-padding mlx --out golden_mlxpad   # raw mlx-audio VAE encoder
```
Options: `--text`, `--seed`, `--timesteps`, `--cfg`, `--min-len`, `--max-len`,
`--trace-steps K` (full CFM trace for the first K AR steps, default 2), `--ref-audio PATH`, and
`--skip-generate-check`.

Defaults: text `"Hello, this is a short test of the Voice C P M two model, running in Rust."`
(20 tokens). Reference = `test_en.wav` from the model snapshot. seed 1234, 10 timesteps,
cfg 2.0. **Result: S=57, 30 AR steps, stopped by the stop head, 4.80 s.**

How it works: the script re-implements the generation loop on the mlx-audio modules so it can
record intermediates. It draws `z_i` with `mx.random.normal((1,64,4))` in the same order as
`UnifiedCFM.sample`. It then calls `model.generate()` with the same seed and checks that the
waveforms agree. **Measured max abs diff = 0.0** (bit-identical), so the dumped loop *is* the
reference. Activations are f32, and weights are as stored (bf16 LM stack, f32 VAE). The VAE
encoder is patched to PyTorch padding (§9 row 1) unless you pass `--vae-encoder-padding mlx`.
`02_ref_latents.safetensors` stores both variants.

Files (keys; `[..]` = shape; VAE traces are channels-last `[T,C]`; CFM tensors are `[N,64,4]`;
patches are `[P=4,D=64]`):

| file | contents |
|---|---|
| `meta.json` | run params, `text_tokens`, `text_ids`, `pytorch_max_len_cap`, `seq_len`, `n_ar_steps`, discrepancy and check numbers, a full key/shape index of every file |
| `tokens.json` | 8 tokenizer cases: transformers tokens/ids, CJK-split tokens, final ids, tokenizers-crate tokens/ids |
| `00_inputs` | `ref_audio_raw[253448]` (48k), `ref_audio_16k[84483]`, `ref_audio_16k_padded[87040]`, `text_token[57]` i32, `text_mask[57]`, `audio_mask[57]`, `audio_feat[57,4,64]` |
| `01_vae_enc_unit` | `input[5120]` (first 2 patches of padded ref), `conv_in_out`, `block{0..3}_{res1,res2,res3,snake,conv}_out`, `fc_mu_out[8,64]` |
| `02_ref_latents` | `ref_latents[34,4,64]` (PT padding), `ref_latents_mlx_padding[34,4,64]` |
| `03_locenc` | `input`, `patch0_seq_in[5,1024]`, `patch0_rope_{cos,sin}[5,128]`, `patch0_layer{00..11}_out`, `patch0_final_norm_out`, `locenc_out[57,1024]`, `enc_to_lm_out[57,2048]` |
| `04_base_lm_prefill` | `text_embed`, `combined_embed`, `rope_{cos,sin}[57,128]`, `layer{00..27}_out[57,2048]` (pre-final-norm residual stream), `final_norm_out`, `fsq_out_all_positions`, `enc_outputs_mixed`, `lm_hidden0[1,2048]`, `fsq_last_tanh[1,512]`, `fsq_last_quant`, `k_cache_layer0[57,2,128]` (post-RoPE), `v_cache_layer0` |
| `05_residual_lm_prefill` | `residual_input[57,2048]`, `layer{00..07}_out`, `final_norm_out`, `residual_hidden0` |
| `06_steps` (row i = AR step i) | `lm_hidden[30,2048]`, `residual_hidden`, `mu[30,2048]`, `cond[30,64,4]`, **`noise[30,64,4]`**, `pred_feat[30,4,64]`, `curr_embed[30,2048]`, `stop_logits[30,2]`, `lm_step_out_pre_fsq[29,2048]` (TSLM step output before FSQ; row i = position S+i) |
| `07_cfm_trace` | per traced step N: `stepN_t_span[11]`, `t_used[10]`, `dt_used[10]`, `x_per_step[11,64,4]` (row 0 = z), `v_pos[10,64,4]`, `v_neg`, `st_star[10]` (row 0 = zero-init step), `first_eval_{t_sinusoid,t_emb,dt_emb,seq_in[2,11,1024],seq_out_normed,velocity[2,64,4]}` (first real DiT call, Euler step 2) |
| `08_vae_dec_unit` | `input[8,64]`, `conv_in_out[8,2048]`, `sr_idx`, `block{0..5}_{srcond_out,conv_t_out,out}`, `conv_out_pre_tanh`, `wav[15368]` (PT length 15360) |
| `09_decode` | `latents[120,64]`, `wav_48k[230408]` (PT length 230400) |
| `output.wav` | 48 kHz waveform for listening |

Read the files in Rust with `candle_core::safetensors::load(path, &device)`. All tensors are
f32 except `text_token` and `sr_idx`, which are i32 (`DType::I32`, supported by candle 0.11).
Convert them with `to_dtype(DType::U32)` before `index_select` / `embedding`.
