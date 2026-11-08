//! AudioVAE V2: 16 kHz encoder and 48 kHz sample-rate-conditioned decoder
//! (PORTING_SPEC §6.9–6.10). Always f32. OWNER: VAE port.
//!
//! Layout: activations are `[1, C, T]` (batch 1, channels-first) throughout.
//! MLX checkpoint weights are permuted at load (see WEIGHTS_INVENTORY.md).
//!
//! Every convolution in the codec is causal, so both directions run as a
//! *stateful stream*: each causal conv keeps its last `2p − op` input samples
//! and each transposed conv keeps its last input frame (PT
//! `StreamingVAEDecoder`). A one-shot call is the same code with a fresh
//! (zero) state, so chunked processing preserves the causal signal up to f32
//! roundoff. Intermediate activation sizes follow the chunk size instead of
//! the signal length; the returned waveform still grows with audio duration.
//!
//! Op choices (candle 0.11.1 Metal):
//! * depthwise k=7 convs: 7 shifted `narrow` + broadcast-mul + add (candle's
//!   grouped conv splits into `C` separate convs, which is far too slow);
//! * pointwise (k=1) convs: a plain matmul;
//! * other dense convs: candle `conv1d` (im2col + GEMM) on the state-padded input;
//! * transposed convs: candle `conv_transpose1d` with padding 0 (col2im fast
//!   path) on `[prev frame | chunk]`, then the leading `s` and trailing `s`
//!   samples are dropped (PT causal trim; `2·ceil(s/2) − s%2 = s` for every s).

use anyhow::{Result, ensure};
use candle_core::{D, DType, Device, Tensor};
use candle_nn::VarBuilder;

use super::config::VaeConfig;

/// Input samples per latent frame at 16 kHz (`prod(encoder_rates)`).
pub const ENC_HOP: usize = 640;
/// Output samples per latent frame at 48 kHz (`prod(decoder_rates)`).
pub const DEC_HOP: usize = 1920;
/// Latent frames per decode chunk in [`AudioVae::decode`] (2 s of audio).
/// Bounds intermediate activation sizes independently of the output length.
pub const DEFAULT_DECODE_CHUNK: usize = 50;
/// Latent frames per encode chunk in [`AudioVae::encode`] (2.56 s of audio).
pub const DEFAULT_ENCODE_CHUNK: usize = 64;

const DILATIONS: [usize; 3] = [1, 3, 9];

// ---------------------------------------------------------------------------
// Streaming state and tracing
// ---------------------------------------------------------------------------

/// Carried causal context for one pass through the encoder or decoder. Slots
/// are visited in a fixed order per chunk, so they are indexed by position.
#[derive(Default)]
struct Stream {
    slots: Vec<Option<Tensor>>,
    cur: usize,
}

impl Stream {
    fn begin_chunk(&mut self) {
        self.cur = 0;
    }

    /// `[ctx | x]` along time, where `ctx` is the last `p` samples seen by this
    /// slot (zeros at the start of the stream). The slot keeps the new last `p`.
    fn left_context(&mut self, x: &Tensor, p: usize) -> Result<Tensor> {
        let i = self.cur;
        self.cur += 1;
        if self.slots.len() <= i {
            self.slots.resize(i + 1, None);
        }
        let prev = match self.slots[i].take() {
            Some(t) => t,
            None => {
                let (b, c, _) = x.dims3()?;
                Tensor::zeros((b, c, p), x.dtype(), x.device())?
            }
        };
        let full = Tensor::cat(&[&prev, x], 2)?;
        let len = full.dim(2)?;
        // Copy so the slot does not pin the whole (possibly large) `full` buffer.
        self.slots[i] = Some(full.narrow(2, len - p, p)?.copy()?);
        Ok(full)
    }
}

/// Optional intermediate capture for the golden unit tests.
#[derive(Default)]
struct Trace {
    on: bool,
    items: Vec<(String, Tensor)>,
}

impl Trace {
    fn rec(&mut self, name: impl FnOnce() -> String, t: &Tensor) {
        if self.on {
            self.items.push((name(), t.clone()));
        }
    }
}

// ---------------------------------------------------------------------------
// Layers
// ---------------------------------------------------------------------------

/// `x + 1/(α+1e-9) · sin(αx)²`, α per channel.
struct Snake {
    alpha: Tensor,     // [1, C, 1]
    inv_alpha: Tensor, // [1, C, 1] = 1/(α+1e-9), computed on the CPU in f32
}

impl Snake {
    fn load(vb: VarBuilder, c: usize) -> Result<Self> {
        let alpha = vb.get((1, 1, c), "alpha")?.reshape((1, c, 1))?;
        let dev = alpha.device().clone();
        let a: Vec<f32> = alpha.flatten_all()?.to_device(&Device::Cpu)?.to_vec1()?;
        let inv: Vec<f32> = a.iter().map(|&v| 1.0 / (v + 1e-9)).collect();
        let inv_alpha = Tensor::from_vec(inv, (1, c, 1), &dev)?;
        Ok(Self { alpha, inv_alpha })
    }

    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let s = x.broadcast_mul(&self.alpha)?.sin()?.sqr()?;
        Ok((x + s.broadcast_mul(&self.inv_alpha)?)?)
    }
}

/// Causal depthwise conv, k=7, dilation `d`, left pad `6d`.
struct DwConv {
    taps: Vec<Tensor>, // 7 × [1, C, 1]
    bias: Tensor,      // [1, C, 1]
    dilation: usize,
}

impl DwConv {
    const K: usize = 7;

    fn load(vb: VarBuilder, c: usize, dilation: usize) -> Result<Self> {
        // MLX [C, 7, 1] → w[c, k].
        let w = vb.get((c, Self::K, 1), "weight")?.reshape((c, Self::K))?;
        let taps = (0..Self::K)
            .map(|k| Ok(w.narrow(1, k, 1)?.contiguous()?.reshape((1, c, 1))?))
            .collect::<Result<Vec<_>>>()?;
        let bias = vb.get(c, "bias")?.reshape((1, c, 1))?;
        Ok(Self {
            taps,
            bias,
            dilation,
        })
    }

    fn pad(&self) -> usize {
        (Self::K - 1) * self.dilation
    }

    fn forward(&self, x: &Tensor, st: &mut Stream) -> Result<Tensor> {
        let n = x.dim(2)?;
        let xp = st.left_context(x, self.pad())?;
        // y[t] = b + Σ_k w[k] · xp[t + k·d]  (cross-correlation on the padded input)
        let mut acc = xp
            .narrow(2, 0, n)?
            .broadcast_mul(&self.taps[0])?
            .broadcast_add(&self.bias)?;
        for (k, w) in self.taps.iter().enumerate().skip(1) {
            acc = (acc + xp.narrow(2, k * self.dilation, n)?.broadcast_mul(w)?)?;
        }
        Ok(acc)
    }
}

/// Pointwise (k=1) conv as a matmul.
struct PwConv {
    w: Tensor,    // [out, in]
    bias: Tensor, // [1, out, 1]
}

impl PwConv {
    fn load(vb: VarBuilder, cin: usize, cout: usize) -> Result<Self> {
        let w = vb.get((cout, 1, cin), "weight")?.reshape((cout, cin))?;
        let bias = vb.get(cout, "bias")?.reshape((1, cout, 1))?;
        Ok(Self { w, bias })
    }

    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let x0 = x.squeeze(0)?.contiguous()?;
        Ok(self
            .w
            .matmul(&x0)?
            .unsqueeze(0)?
            .broadcast_add(&self.bias)?)
    }
}

/// Dense causal conv (k>1, optional stride): left context `pad`, then a valid
/// strided conv via candle `conv1d`.
struct CausalConv {
    w: Tensor,    // [out, in, k] (PyTorch layout)
    bias: Tensor, // [1, out, 1]
    stride: usize,
    pad: usize,
}

impl CausalConv {
    fn load(
        vb: VarBuilder,
        cin: usize,
        cout: usize,
        k: usize,
        stride: usize,
        pad: usize,
    ) -> Result<Self> {
        // MLX [out, k, in] → [out, in, k].
        let w = vb
            .get((cout, k, cin), "weight")?
            .transpose(1, 2)?
            .contiguous()?;
        let bias = vb.get(cout, "bias")?.reshape((1, cout, 1))?;
        Ok(Self {
            w,
            bias,
            stride,
            pad,
        })
    }

    fn forward(&self, x: &Tensor, st: &mut Stream) -> Result<Tensor> {
        let xp = st.left_context(x, self.pad)?;
        Ok(xp
            .conv1d(&self.w, 0, self.stride, 1, 1)?
            .broadcast_add(&self.bias)?)
    }
}

/// Causal transposed conv, kernel 2s, stride s: output length `n·s`.
struct CausalConvT {
    w: Tensor,    // [in, out, 2s] (PyTorch layout)
    bias: Tensor, // [1, out, 1]
    stride: usize,
}

impl CausalConvT {
    fn load(vb: VarBuilder, cin: usize, cout: usize, stride: usize) -> Result<Self> {
        // MLX [out, k, in] → [in, out, k].
        let w = vb
            .get((cout, 2 * stride, cin), "weight")?
            .permute((2, 0, 1))?
            .contiguous()?;
        let bias = vb.get(cout, "bias")?.reshape((1, cout, 1))?;
        Ok(Self { w, bias, stride })
    }

    fn forward(&self, x: &Tensor, st: &mut Stream) -> Result<Tensor> {
        let n = x.dim(2)?;
        let s = self.stride;
        // One frame of context: (K−1)//s = 1. Output length (n+1−1)·s + 2s = (n+2)s;
        // drop s leading (context) and s trailing (causal trim) samples.
        let xp = st.left_context(x, 1)?.contiguous()?;
        let y = xp.conv_transpose1d(&self.w, 0, 0, s, 1, 1)?;
        Ok(y.narrow(2, s, n * s)?.broadcast_add(&self.bias)?)
    }
}

/// `x + conv1x1(Snake(dwconv7_d(Snake(x))))`.
struct ResUnit {
    snake1: Snake,
    conv1: DwConv,
    snake2: Snake,
    conv2: PwConv,
}

impl ResUnit {
    fn load(vb: VarBuilder, c: usize, dilation: usize) -> Result<Self> {
        Ok(Self {
            snake1: Snake::load(vb.pp("snake1"), c)?,
            conv1: DwConv::load(vb.pp("conv1"), c, dilation)?,
            snake2: Snake::load(vb.pp("snake2"), c)?,
            conv2: PwConv::load(vb.pp("conv2"), c, c)?,
        })
    }

    fn forward(&self, x: &Tensor, st: &mut Stream) -> Result<Tensor> {
        let h = self.snake1.forward(x)?;
        let h = self.conv1.forward(&h, st)?;
        let h = self.snake2.forward(&h)?;
        let h = self.conv2.forward(&h)?;
        Ok((x + h)?)
    }
}

fn load_res(vb: &VarBuilder, c: usize) -> Result<Vec<ResUnit>> {
    DILATIONS
        .iter()
        .enumerate()
        .map(|(r, &d)| ResUnit::load(vb.pp(format!("res{}", r + 1)), c, d))
        .collect()
}

// ---------------------------------------------------------------------------
// Encoder
// ---------------------------------------------------------------------------

struct EncBlock {
    res: Vec<ResUnit>,
    snake: Snake,
    conv: CausalConv,
}

struct Encoder {
    conv_in: CausalConv,
    blocks: Vec<EncBlock>,
    fc_mu: CausalConv,
    hop: usize,
}

impl Encoder {
    fn load(vb: VarBuilder, cfg: &VaeConfig) -> Result<Self> {
        let mut c = cfg.encoder_dim;
        let conv_in = CausalConv::load(vb.pp("conv_in"), 1, c, 7, 1, 6)?;
        let mut blocks = Vec::new();
        for (i, &s) in cfg.encoder_rates.iter().enumerate() {
            let vbb = vb.pp(format!("blocks.layers.{i}"));
            // PT left pad 2·ceil(s/2) − s%2 (spec §9 row 1: 5 for s=5, not MLX's 6).
            let pad = 2 * s.div_ceil(2) - s % 2;
            blocks.push(EncBlock {
                res: load_res(&vbb, c)?,
                snake: Snake::load(vbb.pp("snake"), c)?,
                conv: CausalConv::load(vbb.pp("conv"), c, 2 * c, 2 * s, s, pad)?,
            });
            c *= 2;
        }
        let fc_mu = CausalConv::load(vb.pp("fc_mu"), c, cfg.latent_dim, 3, 1, 2)?;
        Ok(Self {
            conv_in,
            blocks,
            fc_mu,
            hop: cfg.encoder_rates.iter().product(),
        })
    }

    /// `x` `[1, 1, n]` with `n % hop == 0` → `[1, latent, n/hop]`.
    fn forward(&self, x: &Tensor, st: &mut Stream, tr: &mut Trace) -> Result<Tensor> {
        st.begin_chunk();
        let mut x = self.conv_in.forward(x, st)?;
        tr.rec(|| "conv_in_out".into(), &x);
        for (i, b) in self.blocks.iter().enumerate() {
            for (r, ru) in b.res.iter().enumerate() {
                x = ru.forward(&x, st)?;
                tr.rec(|| format!("block{i}_res{}_out", r + 1), &x);
            }
            x = b.snake.forward(&x)?;
            tr.rec(|| format!("block{i}_snake_out"), &x);
            x = b.conv.forward(&x, st)?;
            tr.rec(|| format!("block{i}_conv_out"), &x);
        }
        let mu = self.fc_mu.forward(&x, st)?;
        tr.rec(|| "fc_mu_out".into(), &mu);
        Ok(mu)
    }
}

// ---------------------------------------------------------------------------
// Decoder
// ---------------------------------------------------------------------------

struct DecBlock {
    sr_scale: Tensor, // [1, Cin, 1] (row `bucket` of scale_embed)
    sr_bias: Tensor,  // [1, Cin, 1]
    snake: Snake,
    conv_t: CausalConvT,
    res: Vec<ResUnit>,
}

struct Decoder {
    conv_in_dw: DwConv,
    conv_in_pw: PwConv,
    blocks: Vec<DecBlock>,
    snake_out: Snake,
    conv_out: CausalConv,
}

/// PT `torch.bucketize(sr, boundaries)` (right=False): number of boundaries `< sr`.
pub fn sr_bucket(boundaries: &[u32], sr: u32) -> usize {
    boundaries.iter().filter(|&&b| b < sr).count()
}

impl Decoder {
    fn load(vb: VarBuilder, cfg: &VaeConfig) -> Result<Self> {
        let latent = cfg.latent_dim;
        let conv_in_dw = DwConv::load(vb.pp("conv_in.layers.0"), latent, 1)?;
        let conv_in_pw = PwConv::load(vb.pp("conv_in.layers.1"), latent, cfg.decoder_dim)?;
        let n_buckets = cfg.sr_bin_boundaries.len() + 1;
        let bucket = sr_bucket(&cfg.sr_bin_boundaries, cfg.out_sample_rate);
        let mut blocks = Vec::new();
        let mut cin = cfg.decoder_dim;
        for (i, &s) in cfg.decoder_rates.iter().enumerate() {
            let cout = cin / 2;
            let vbb = vb.pp(format!("blocks.layers.{i}"));
            let vbs = vb.pp(format!("sr_cond_layers.{i}"));
            let row = |name: &str| -> Result<Tensor> {
                Ok(vbs
                    .get((n_buckets, cin), name)?
                    .narrow(0, bucket, 1)?
                    .reshape((1, cin, 1))?
                    .contiguous()?)
            };
            blocks.push(DecBlock {
                sr_scale: row("scale_embed.weight")?,
                sr_bias: row("bias_embed.weight")?,
                snake: Snake::load(vbb.pp("snake"), cin)?,
                conv_t: CausalConvT::load(vbb.pp("conv_t"), cin, cout, s)?,
                res: load_res(&vbb, cout)?,
            });
            cin = cout;
        }
        let snake_out = Snake::load(vb.pp("snake_out"), cin)?;
        let conv_out = CausalConv::load(vb.pp("conv_out"), cin, 1, 7, 1, 6)?;
        Ok(Self {
            conv_in_dw,
            conv_in_pw,
            blocks,
            snake_out,
            conv_out,
        })
    }

    /// `z` `[1, latent, n]` → waveform `[1, 1, n·hop]` (after tanh).
    fn forward(&self, z: &Tensor, st: &mut Stream, tr: &mut Trace) -> Result<Tensor> {
        st.begin_chunk();
        let x = self.conv_in_dw.forward(z, st)?;
        let mut x = self.conv_in_pw.forward(&x)?;
        tr.rec(|| "conv_in_out".into(), &x);
        for (i, b) in self.blocks.iter().enumerate() {
            x = x.broadcast_mul(&b.sr_scale)?.broadcast_add(&b.sr_bias)?;
            tr.rec(|| format!("block{i}_srcond_out"), &x);
            x = b.snake.forward(&x)?;
            x = b.conv_t.forward(&x, st)?;
            tr.rec(|| format!("block{i}_conv_t_out"), &x);
            for ru in &b.res {
                x = ru.forward(&x, st)?;
            }
            tr.rec(|| format!("block{i}_out"), &x);
        }
        let x = self.snake_out.forward(&x)?;
        let x = self.conv_out.forward(&x, st)?;
        tr.rec(|| "conv_out_pre_tanh".into(), &x);
        Ok(x.tanh()?)
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct AudioVae {
    encoder: Encoder,
    decoder: Decoder,
    latent_dim: usize,
    dec_hop: usize,
}

/// Carried state for chunk-by-chunk decoding (PT `StreamingVAEDecoder`).
/// Feeding consecutive latent chunks through [`AudioVae::decode_stream`] yields
/// the samples of a one-shot decode of their concatenation up to f32 roundoff.
#[derive(Default)]
pub struct DecodeStream {
    st: Stream,
}

impl DecodeStream {
    pub fn new() -> Self {
        Self::default()
    }
}

impl AudioVae {
    /// `vb` at `audio_vae`, dtype f32.
    pub fn load(vb: VarBuilder, cfg: &VaeConfig) -> Result<Self> {
        ensure!(vb.dtype() == DType::F32, "AudioVAE must be loaded in f32");
        let enc_hop: usize = cfg.encoder_rates.iter().product();
        let dec_hop: usize = cfg.decoder_rates.iter().product();
        ensure!(
            enc_hop == ENC_HOP && dec_hop == DEC_HOP,
            "unexpected VAE hops {enc_hop}/{dec_hop}"
        );
        Ok(Self {
            encoder: Encoder::load(vb.pp("encoder"), cfg)?,
            decoder: Decoder::load(vb.pp("decoder"), cfg)?,
            latent_dim: cfg.latent_dim,
            dec_hop,
        })
    }

    /// `wav`: `[L]` f32 at 16 kHz, `L` a multiple of 640 (callers pad to 2560).
    /// Returns latents `[L/640, 64]` (channels-last, f32).
    pub fn encode(&self, wav: &Tensor) -> Result<Tensor> {
        self.encode_chunked(wav, DEFAULT_ENCODE_CHUNK)
    }

    /// [`Self::encode`] processing `chunk_frames` latent frames at a time
    /// (causal state is carried between chunks).
    pub fn encode_chunked(&self, wav: &Tensor, chunk_frames: usize) -> Result<Tensor> {
        let (mu, _) = self.encode_inner(wav, chunk_frames, false)?;
        Ok(mu)
    }

    fn encode_inner(
        &self,
        wav: &Tensor,
        chunk_frames: usize,
        trace: bool,
    ) -> Result<(Tensor, Trace)> {
        ensure!(chunk_frames > 0, "chunk_frames must be > 0");
        let len = wav.dims1()?;
        let hop = self.encoder.hop;
        ensure!(
            len > 0 && len % hop == 0,
            "encode: length {len} is not a positive multiple of {hop}"
        );
        let x = wav.to_dtype(DType::F32)?.reshape((1, 1, len))?;
        let mut st = Stream::default();
        let mut tr = Trace {
            on: trace,
            ..Default::default()
        };
        let chunk = chunk_frames * hop;
        let mut outs = Vec::new();
        let mut off = 0;
        while off < len {
            let n = chunk.min(len - off);
            outs.push(
                self.encoder
                    .forward(&x.narrow(2, off, n)?, &mut st, &mut tr)?,
            );
            off += n;
        }
        let mu = Tensor::cat(&outs, 2)?; // [1, 64, T]
        Ok((mu.squeeze(0)?.t()?.contiguous()?, tr))
    }

    /// `latents`: `[T, 64]` → waveform `[T·1920]` f32 at 48 kHz (after tanh).
    /// Intermediate activations are bounded by the decode chunk size.
    pub fn decode(&self, latents: &Tensor) -> Result<Tensor> {
        self.decode_chunked(latents, DEFAULT_DECODE_CHUNK)
    }

    /// [`Self::decode`] processing `chunk_frames` latent frames at a time.
    /// Equivalent up to f32 roundoff; `chunk_frames ≥ T` is a one-shot decode.
    pub fn decode_chunked(&self, latents: &Tensor, chunk_frames: usize) -> Result<Tensor> {
        ensure!(chunk_frames > 0, "chunk_frames must be > 0");
        let (t, _) = latents.dims2()?;
        let mut stream = DecodeStream::new();
        let mut outs = Vec::new();
        let mut off = 0;
        while off < t {
            let n = chunk_frames.min(t - off);
            outs.push(self.decode_stream(&mut stream, &latents.narrow(0, off, n)?)?);
            off += n;
        }
        Ok(Tensor::cat(&outs, 0)?)
    }

    /// Decodes the next `[n, 64]` latent chunk of a stream → `[n·1920]` samples.
    pub fn decode_stream(&self, stream: &mut DecodeStream, latents: &Tensor) -> Result<Tensor> {
        let (wav, _) = self.decode_inner(&mut stream.st, latents, false)?;
        Ok(wav)
    }

    fn decode_inner(
        &self,
        st: &mut Stream,
        latents: &Tensor,
        trace: bool,
    ) -> Result<(Tensor, Trace)> {
        let (t, d) = latents.dims2()?;
        ensure!(
            d == self.latent_dim,
            "decode: latent dim {d} != {}",
            self.latent_dim
        );
        ensure!(t > 0, "decode: empty latents");
        let z = latents
            .to_dtype(DType::F32)?
            .t()?
            .contiguous()?
            .unsqueeze(0)?; // [1, 64, T]
        let mut tr = Trace {
            on: trace,
            ..Default::default()
        };
        let wav = self.decoder.forward(&z, st, &mut tr)?;
        debug_assert_eq!(wav.dim(D::Minus1)?, t * self.dec_hop);
        Ok((wav.flatten_all()?, tr))
    }
}

// ---------------------------------------------------------------------------
// Tests (PORTING_SPEC §11: T3, T4, T15, T16, T17)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::Instant;

    use super::*;
    use crate::native::config::ModelConfig;
    use crate::native::{golden, weights};

    struct Ctx {
        vae: AudioVae,
        dev: Device,
    }

    fn setup() -> Result<Option<Ctx>> {
        let Some((_g, snap)) = golden::require() else {
            return Ok(None);
        };
        let dev = weights::device()?;
        let cfg = ModelConfig::load(&snap.join("config.json"))?;
        let vb = weights::var_builder(&snap, DType::F32, &dev)?;
        let vae = AudioVae::load(vb.pp("audio_vae"), &cfg.audio_vae_config)?;
        Ok(Some(Ctx { vae, dev }))
    }

    /// `[1, C, T]` → `[T, C]` (golden layout), truncated to the first `t` rows.
    fn to_tc(x: &Tensor) -> Result<Tensor> {
        Ok(x.squeeze(0)?.t()?.contiguous()?)
    }

    fn sync(dev: &Device) -> Result<()> {
        dev.synchronize()?;
        Ok(())
    }

    fn metal_allocated_mb(dev: &Device) -> Option<f64> {
        match dev {
            Device::Metal(m) => Some(m.metal_device().current_allocated_size() as f64 / 1e6),
            _ => None,
        }
    }

    fn compare_trace(
        tag: &str,
        trace: &[(String, Tensor)],
        gold: &HashMap<String, Tensor>,
        tol: f32,
    ) -> Result<()> {
        let mut worst = 0f32;
        for (name, ours) in trace {
            let g = gold
                .get(name)
                .unwrap_or_else(|| panic!("{tag}: golden key {name} missing"));
            let ours = to_tc(ours)?;
            let n = ours.dim(0)?;
            // MLX decoder blocks 2+ are a few samples longer (spec §9 row 2): compare prefixes.
            assert!(
                g.dim(0)? >= n,
                "{tag}/{name}: golden shorter ({:?} vs {:?})",
                g.dims(),
                ours.dims()
            );
            let g = g.narrow(0, 0, n)?;
            let rel = golden::rel_err(&ours, &g)?;
            let mae = golden::max_abs_err(&ours, &g)?;
            eprintln!(
                "{tag} {name:<22} {:?} rel {rel:.3e} maxabs {mae:.3e}",
                ours.dims()
            );
            worst = worst.max(rel);
            assert!(rel < tol, "{tag}/{name}: rel {rel:e} >= {tol:e}");
        }
        eprintln!("{tag}: worst rel {worst:.3e}");
        Ok(())
    }

    #[test]
    fn sr_bucket_matches_bucketize() {
        let b = [20000, 30000, 40000];
        assert_eq!(sr_bucket(&b, 48000), 3);
        assert_eq!(sr_bucket(&b, 16000), 0);
        assert_eq!(sr_bucket(&b, 20000), 0); // right=False: equal goes left
        assert_eq!(sr_bucket(&b, 24000), 1);
    }

    /// Independent scalar oracle for odd-stride causal transposed convolution.
    /// The first sample includes current input only; later samples overlap the
    /// previous frame. Streaming must neither leak future samples nor emit a tail.
    #[test]
    fn causal_transpose_matches_scalar_and_streams() -> Result<()> {
        let dev = Device::Cpu;
        let (cin, cout, frames, stride) = (2usize, 3usize, 5usize, 3usize);
        let kernel = 2 * stride;
        let input: Vec<f32> = (0..cin * frames).map(|i| (i as f32 - 4.) / 8.).collect();
        let weights: Vec<f32> = (0..cin * cout * kernel)
            .map(|i| ((i * 7 % 19) as f32 - 9.) / 16.)
            .collect();
        let bias = vec![0.125, -0.25, 0.5];
        let layer = CausalConvT {
            w: Tensor::from_vec(weights.clone(), (cin, cout, kernel), &dev)?,
            bias: Tensor::from_vec(bias.clone(), (1, cout, 1), &dev)?,
            stride,
        };
        let x = Tensor::from_vec(input.clone(), (1, cin, frames), &dev)?;
        let mut expected = vec![0f32; cout * frames * stride];
        for oc in 0..cout {
            for time in 0..frames * stride {
                let mut sum = bias[oc];
                for ic in 0..cin {
                    for frame in 0..frames {
                        if time >= frame * stride && time - frame * stride < kernel {
                            let k = time - frame * stride;
                            sum +=
                                input[ic * frames + frame] * weights[(ic * cout + oc) * kernel + k];
                        }
                    }
                }
                expected[oc * frames * stride + time] = sum;
            }
        }
        let expected = Tensor::from_vec(expected, (1, cout, frames * stride), &dev)?;
        for chunk in [1usize, 2, 3, frames] {
            let mut state = Stream::default();
            let mut parts = Vec::new();
            for off in (0..frames).step_by(chunk) {
                state.begin_chunk();
                parts.push(layer.forward(&x.narrow(2, off, chunk.min(frames - off))?, &mut state)?);
            }
            let got = Tensor::cat(&parts, 2)?;
            assert_eq!(got.dims(), expected.dims());
            assert!(golden::max_abs_err(&got, &expected)? < 1e-6);
        }
        Ok(())
    }

    /// Dilation and left context remain correct when chunks are shorter than
    /// the receptive field; channels must remain independent.
    #[test]
    fn depthwise_matches_scalar_and_streams() -> Result<()> {
        let dev = Device::Cpu;
        let (channels, frames, dilation) = (2usize, 11usize, 3usize);
        let input: Vec<f32> = (0..channels * frames)
            .map(|i| (i as f32 - 10.) / 8.)
            .collect();
        let taps: Vec<Tensor> = (0..DwConv::K)
            .map(|k| {
                Tensor::from_vec(
                    vec![k as f32 / 8., -(k as f32) / 4.],
                    (1, channels, 1),
                    &dev,
                )
            })
            .collect::<candle_core::Result<_>>()?;
        let layer = DwConv {
            taps,
            bias: Tensor::from_vec(vec![0.5f32, -0.25], (1, channels, 1), &dev)?,
            dilation,
        };
        let x = Tensor::from_vec(input.clone(), (1, channels, frames), &dev)?;
        let mut expected = vec![0f32; channels * frames];
        for channel in 0..channels {
            for time in 0..frames {
                let mut sum = if channel == 0 { 0.5 } else { -0.25 };
                for k in 0..DwConv::K {
                    let lag = (DwConv::K - 1 - k) * dilation;
                    if time >= lag {
                        let weight = if channel == 0 {
                            k as f32 / 8.
                        } else {
                            -(k as f32) / 4.
                        };
                        sum += input[channel * frames + time - lag] * weight;
                    }
                }
                expected[channel * frames + time] = sum;
            }
        }
        let expected = Tensor::from_vec(expected, (1, channels, frames), &dev)?;
        for chunk in [1usize, 2, 5, frames] {
            let mut state = Stream::default();
            let mut parts = Vec::new();
            for off in (0..frames).step_by(chunk) {
                state.begin_chunk();
                parts.push(layer.forward(&x.narrow(2, off, chunk.min(frames - off))?, &mut state)?);
            }
            assert!(golden::max_abs_err(&Tensor::cat(&parts, 2)?, &expected)? < 1e-6);
        }
        Ok(())
    }

    /// T3: encoder unit trace on the first 2 patches of the padded reference.
    #[test]
    fn t3_encoder_unit_trace() -> Result<()> {
        let Some(ctx) = setup()? else { return Ok(()) };
        let gold = golden::load("01_vae_enc_unit.safetensors", &ctx.dev)?;
        let input = &gold["input"];
        // One chunk (as traced), then a second run with small chunks for the final output.
        let (mu, tr) = ctx.vae.encode_inner(input, usize::MAX / ENC_HOP, true)?;
        compare_trace("T3", &tr.items, &gold, 1e-5)?;
        let rel = golden::rel_err(&mu, &gold["fc_mu_out"])?;
        assert!(rel < 1e-5, "T3 mu rel {rel:e}");
        let mu_chunked = ctx.vae.encode_chunked(input, 2)?;
        let d = golden::max_abs_err(&mu_chunked, &mu)?;
        eprintln!("T3 chunked(2 frames) vs one-shot encode maxabs {d:.3e}");
        assert!(d < 1e-5);
        Ok(())
    }

    /// T4: full reference encode (PT padding variant).
    #[test]
    fn t4_encoder_full() -> Result<()> {
        let Some(ctx) = setup()? else { return Ok(()) };
        let inputs = golden::load("00_inputs.safetensors", &ctx.dev)?;
        let lat = golden::load("02_ref_latents.safetensors", &ctx.dev)?;
        let wav = &inputs["ref_audio_16k_padded"];
        let want = lat["ref_latents"].reshape(((), 64))?;
        let _ = ctx.vae.encode(wav)?; // warm-up (kernel compilation)
        sync(&ctx.dev)?;
        let t0 = Instant::now();
        let got = ctx.vae.encode(wav)?;
        sync(&ctx.dev)?;
        let ms = t0.elapsed().as_secs_f64() * 1e3;
        assert_eq!(got.dims(), want.dims());
        let rel = golden::rel_err(&got, &want)?;
        let mae = golden::max_abs_err(&got, &want)?;
        let one_shot = ctx.vae.encode_chunked(wav, usize::MAX / ENC_HOP)?;
        let rel1 = golden::rel_err(&one_shot, &want)?;
        let mlx = lat["ref_latents_mlx_padding"].reshape(((), 64))?;
        let rel_mlx = golden::rel_err(&got, &mlx)?;
        eprintln!(
            "T4 encode {} samples ({:.2} s): rel {rel:.3e} maxabs {mae:.3e} (one-shot rel {rel1:.3e}; \
             vs MLX-padding latents rel {rel_mlx:.3e}) — {ms:.1} ms",
            wav.dims1()?,
            wav.dims1()? as f64 / 16000.0
        );
        assert!(rel < 1e-5, "T4 rel {rel:e}");
        assert!(rel1 < 1e-5, "T4 one-shot rel {rel1:e}");
        Ok(())
    }

    /// T15: decoder unit trace on 8 latent frames.
    #[test]
    fn t15_decoder_unit_trace() -> Result<()> {
        let Some(ctx) = setup()? else { return Ok(()) };
        let gold = golden::load("08_vae_dec_unit.safetensors", &ctx.dev)?;
        let sr_idx: Vec<i32> = gold["sr_idx"].to_device(&Device::Cpu)?.to_vec1()?;
        assert_eq!(sr_idx[0] as usize, 3);
        let mut st = Stream::default();
        let (wav, tr) = ctx.vae.decode_inner(&mut st, &gold["input"], true)?;
        compare_trace("T15", &tr.items, &gold, 1e-5)?;
        let n = wav.dims1()?;
        assert_eq!(n, 8 * DEC_HOP);
        let g = gold["wav"].narrow(0, 0, n)?;
        let rel = golden::rel_err(&wav, &g)?;
        let mae = golden::max_abs_err(&wav, &g)?;
        eprintln!("T15 wav rel {rel:.3e} maxabs {mae:.3e}");
        assert!(rel < 1e-5, "T15 wav rel {rel:e}");
        Ok(())
    }

    /// T16: full decode of the generated latents vs the reference waveform.
    /// T17: chunked / patch-by-patch streaming decode equals the one-shot decode.
    #[test]
    fn t16_t17_decode_full_and_streaming() -> Result<()> {
        let Some(ctx) = setup()? else { return Ok(()) };
        let gold = golden::load("09_decode.safetensors", &ctx.dev)?;
        let lat = &gold["latents"];
        let t = lat.dim(0)?;
        let want = gold["wav_48k"].narrow(0, 0, t * DEC_HOP)?;

        let full = ctx.vae.decode_chunked(lat, t)?;
        assert_eq!(full.dims1()?, t * DEC_HOP);
        let mae = golden::max_abs_err(&full, &want)?;
        let rel = golden::rel_err(&full, &want)?;
        eprintln!("T16 one-shot decode T={t}: maxabs {mae:.3e} rel {rel:.3e}");
        assert!(mae < 1e-4, "T16 maxabs {mae:e}");

        let dflt = ctx.vae.decode(lat)?;
        let mae_d = golden::max_abs_err(&dflt, &want)?;
        eprintln!("T16 default (chunk {DEFAULT_DECODE_CHUNK}) decode: maxabs {mae_d:.3e}");
        assert!(mae_d < 1e-4, "T16 default maxabs {mae_d:e}");

        // T17: patch by patch (4 frames, as PT streaming), odd sizes, and a single frame stream.
        for chunk in [4usize, 7, 1, DEFAULT_DECODE_CHUNK] {
            let wav = ctx.vae.decode_chunked(lat, chunk)?;
            let d = golden::max_abs_err(&wav, &full)?;
            eprintln!("T17 chunk {chunk:>3} vs one-shot: maxabs {d:.3e}");
            assert!(d < 1e-5, "T17 chunk {chunk}: maxabs {d:e}");
        }
        // Explicit DecodeStream API with uneven chunks.
        let mut s = DecodeStream::new();
        let mut parts = Vec::new();
        let mut off = 0;
        for n in [3usize, 4, 13, 50, 1].iter().cycle() {
            if off >= t {
                break;
            }
            let n = (*n).min(t - off);
            parts.push(ctx.vae.decode_stream(&mut s, &lat.narrow(0, off, n)?)?);
            off += n;
        }
        let d = golden::max_abs_err(&Tensor::cat(&parts, 0)?, &full)?;
        eprintln!("T17 DecodeStream uneven chunks vs one-shot: maxabs {d:.3e}");
        assert!(d < 1e-5);
        Ok(())
    }

    /// Timing: encode the 5 s reference, decode 10 s and 30 s of latents.
    #[test]
    fn timing_decode() -> Result<()> {
        let Some(ctx) = setup()? else { return Ok(()) };
        let gold = golden::load("09_decode.safetensors", &ctx.dev)?;
        let lat = &gold["latents"]; // 120 frames = 4.8 s
        let tile = |frames: usize| -> Result<Tensor> {
            let reps = frames.div_ceil(lat.dim(0)?);
            Ok(Tensor::cat(&vec![lat; reps], 0)?.narrow(0, 0, frames)?)
        };
        let _ = ctx.vae.decode(&lat.narrow(0, 0, 8)?)?; // warm-up
        sync(&ctx.dev)?;
        for (secs, chunk) in [
            (10usize, DEFAULT_DECODE_CHUNK),
            (30, DEFAULT_DECODE_CHUNK),
            (10, 25),
            (10, 100),
            (10, 250),
        ] {
            let frames = secs * 25;
            let z = tile(frames)?;
            sync(&ctx.dev)?;
            let t0 = Instant::now();
            let wav = ctx.vae.decode_chunked(&z, chunk)?;
            let _ = wav.to_device(&Device::Cpu)?;
            let dt = t0.elapsed().as_secs_f64();
            eprintln!(
                "TIMING decode {secs:>2} s ({frames} frames, chunk {chunk}): {:.0} ms, RTF {:.3}, metal pool {:?} MB",
                dt * 1e3,
                dt / secs as f64,
                metal_allocated_mb(&ctx.dev).map(|m| m.round())
            );
        }
        Ok(())
    }
}
