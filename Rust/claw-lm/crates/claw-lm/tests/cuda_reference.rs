#![cfg(feature = "cuda")]
mod common;
use claw_lm::{Decoder, cpu::CpuDecoder, cuda::CudaDecoder, generation::Attention};
#[test]
#[ignore = "Requires an explicitly selected supported GPU: CLAW_LM_TEST_DEVICE"]
fn cuda_matches_scalar_and_preserves_prefix() {
    let device = std::env::var("CLAW_LM_TEST_DEVICE")
        .expect("Set explicit CUDA device")
        .parse()
        .unwrap();
    let dir = common::fixture();
    let assets = common::assets(&dir);
    let mut cpu = CpuDecoder::new(common::assets(&dir), 64).unwrap();
    let mut gpu = CudaDecoder::new(&assets, device, 64).unwrap();
    let prompt = (1..=17).collect::<Vec<_>>();
    let compare = |a: &Vec<Vec<f32>>, b: &Vec<Vec<f32>>| {
        let max = a
            .iter()
            .flatten()
            .zip(b.iter().flatten())
            .map(|(a, b)| (a - b).abs())
            .fold(0., f32::max);
        eprintln!("Native/scalar mixed-precision max absolute logit difference={max}");
        assert!(max < 0.0001);
    };
    let c = cpu.forward(&prompt, Attention::Causal, true, true).unwrap();
    let g = gpu.forward(&prompt, Attention::Causal, true, true).unwrap();
    compare(&c, &g);
    let c = cpu
        .forward(&[3, 4, 5], Attention::Bidirectional, false, false)
        .unwrap();
    let first = gpu
        .forward(&[3, 4, 5], Attention::Bidirectional, false, false)
        .unwrap();
    compare(&c, &first);
    assert_eq!(gpu.cache_len(), 17);
    assert_eq!(
        first,
        gpu.forward(&[3, 4, 5], Attention::Bidirectional, false, false)
            .unwrap()
    );
    gpu.forward(&[3, 4, 5], Attention::Causal, true, true)
        .unwrap();
    gpu.crop(17).unwrap();
    assert_eq!(
        first,
        gpu.forward(&[3, 4, 5], Attention::Bidirectional, false, false)
            .unwrap()
    );
    gpu.reset().unwrap();
    assert_eq!(gpu.cache_len(), 0);
}
