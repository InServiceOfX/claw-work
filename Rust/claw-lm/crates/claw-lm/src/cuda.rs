//! Owning, checked C ABI adapter. All GPU computations live in CUDA C++.
use crate::{
    Assets,
    generation::{Attention, Decoder},
};
use anyhow::{Context, Result, ensure};
use std::{
    ffi::{CStr, CString, c_char, c_void},
    ptr::NonNull,
};
#[repr(C)]
struct NativeConfig {
    hidden: i32,
    intermediate: i32,
    layers: i32,
    query_heads: i32,
    kv_heads: i32,
    head_dim: i32,
    vocabulary: i32,
    context: i32,
    chunk: i32,
    original_context: i32,
    epsilon: f32,
    query_scale_beta: f32,
    theta: f64,
    factor: f64,
    beta_fast: f64,
    beta_slow: f64,
    mscale: f64,
    mscale_all_dim: f64,
}
unsafe extern "C" {
    fn clm_device_count(count: *mut i32, error: *mut c_char, capacity: usize) -> i32;
    fn clm_device_info(
        index: i32,
        name: *mut c_char,
        name_size: usize,
        major: *mut i32,
        minor: *mut i32,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
    fn clm_create(
        c: *const NativeConfig,
        device: i32,
        bytes: usize,
        error: *mut c_char,
        capacity: usize,
    ) -> *mut c_void;
    fn clm_upload(
        d: *mut c_void,
        name: *const c_char,
        data: *const c_void,
        length: usize,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
    fn clm_forward(
        d: *mut c_void,
        tokens: *const u32,
        count: usize,
        causal: i32,
        commit: i32,
        last: i32,
        logits: *mut f32,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
    fn clm_crop(d: *mut c_void, length: usize, error: *mut c_char, capacity: usize) -> i32;
    fn clm_destroy(d: *mut c_void);
}
pub struct CudaDecoder {
    handle: NonNull<c_void>,
    vocabulary: usize,
    context: usize,
    prefix: usize,
}
// SAFETY: exclusive mutable access serializes calls; C++ selects its device on each call.
unsafe impl Send for CudaDecoder {}
impl Drop for CudaDecoder {
    fn drop(&mut self) {
        unsafe { clm_destroy(self.handle.as_ptr()) }
    }
}
fn check(code: i32, error: &[c_char; 1024]) -> Result<()> {
    if code != 0 {
        let text = unsafe { CStr::from_ptr(error.as_ptr()) }.to_string_lossy();
        anyhow::bail!("CUDA backend: {text}");
    }
    Ok(())
}
impl CudaDecoder {
    pub fn new(assets: &Assets, device: i32, context: usize) -> Result<Self> {
        let c = &assets.config;
        let r = &c.rope_parameters;
        ensure!(
            context > 0 && context <= c.max_position_embeddings && context <= i32::MAX as usize,
            "Invalid CUDA context limit"
        );
        let config = NativeConfig {
            hidden: c.hidden_size as i32,
            intermediate: c.intermediate_size as i32,
            layers: c.num_hidden_layers as i32,
            query_heads: c.num_attention_heads as i32,
            kv_heads: c.num_key_value_heads as i32,
            head_dim: c.head_dim as i32,
            vocabulary: c.vocab_size as i32,
            context: context as i32,
            chunk: 128,
            original_context: r.original_max_position_embeddings as i32,
            epsilon: c.rms_norm_eps,
            query_scale_beta: r.llama_4_scaling_beta as f32,
            theta: r.rope_theta,
            factor: r.factor,
            beta_fast: r.beta_fast,
            beta_slow: r.beta_slow,
            mscale: r.mscale,
            mscale_all_dim: r.mscale_all_dim,
        };
        let mut error = [0; 1024];
        let raw = unsafe {
            clm_create(
                &config,
                device,
                assets.weights.byte_len(),
                error.as_mut_ptr(),
                error.len(),
            )
        };
        if raw.is_null() {
            check(1, &error)?;
        }
        let engine = Self {
            handle: NonNull::new(raw).context("Null CUDA handle")?,
            vocabulary: c.vocab_size,
            context,
            prefix: 0,
        };
        for name in assets.weights.names() {
            let t = assets.weights.tensor(name)?;
            let name = CString::new(name)?;
            let code = unsafe {
                clm_upload(
                    engine.handle.as_ptr(),
                    name.as_ptr(),
                    t.bytes.as_ptr().cast(),
                    t.bytes.len(),
                    error.as_mut_ptr(),
                    error.len(),
                )
            };
            check(code, &error)?;
        }
        Ok(engine)
    }
}
impl Decoder for CudaDecoder {
    fn vocab_size(&self) -> usize {
        self.vocabulary
    }
    fn context_limit(&self) -> usize {
        self.context
    }
    fn cache_len(&self) -> usize {
        self.prefix
    }
    fn reset(&mut self) -> Result<()> {
        self.crop(0)
    }
    fn crop(&mut self, length: usize) -> Result<()> {
        ensure!(length <= self.prefix, "Invalid cache crop");
        let mut error = [0; 1024];
        let code = unsafe {
            clm_crop(
                self.handle.as_ptr(),
                length,
                error.as_mut_ptr(),
                error.len(),
            )
        };
        check(code, &error)?;
        self.prefix = length;
        Ok(())
    }
    fn forward(
        &mut self,
        tokens: &[u32],
        mode: Attention,
        commit: bool,
        last: bool,
    ) -> Result<Vec<Vec<f32>>> {
        ensure!(
            !tokens.is_empty() && tokens.len() <= 128 && self.prefix + tokens.len() <= self.context,
            "Invalid CUDA forward size"
        );
        let rows = if last { 1 } else { tokens.len() };
        let mut result = vec![0.; rows * self.vocabulary];
        let mut error = [0; 1024];
        let code = unsafe {
            clm_forward(
                self.handle.as_ptr(),
                tokens.as_ptr(),
                tokens.len(),
                i32::from(mode == Attention::Causal),
                i32::from(commit),
                i32::from(last),
                result.as_mut_ptr(),
                error.as_mut_ptr(),
                error.len(),
            )
        };
        check(code, &error)?;
        if commit {
            self.prefix += tokens.len();
        }
        Ok(result
            .chunks_exact(self.vocabulary)
            .map(<[f32]>::to_vec)
            .collect())
    }
}

#[derive(Debug, serde::Serialize)]
pub struct DeviceInfo {
    pub index: i32,
    pub name: String,
    pub major: i32,
    pub minor: i32,
    pub supported: bool,
}
pub fn devices() -> Result<Vec<DeviceInfo>> {
    let mut error = [0; 1024];
    let mut count = 0;
    check(
        unsafe { clm_device_count(&mut count, error.as_mut_ptr(), error.len()) },
        &error,
    )?;
    let mut devices = Vec::new();
    for index in 0..count {
        let mut name = [0; 256];
        let mut major = 0;
        let mut minor = 0;
        check(
            unsafe {
                clm_device_info(
                    index,
                    name.as_mut_ptr(),
                    name.len(),
                    &mut major,
                    &mut minor,
                    error.as_mut_ptr(),
                    error.len(),
                )
            },
            &error,
        )?;
        let name = unsafe { CStr::from_ptr(name.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        let supported = crate::assets::supported_gpu(major, minor, &name);
        devices.push(DeviceInfo {
            index,
            name,
            major,
            minor,
            supported,
        });
    }
    Ok(devices)
}
