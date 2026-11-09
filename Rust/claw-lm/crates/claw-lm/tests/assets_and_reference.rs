mod common;
use claw_lm::{Assets, Decoder, cpu::CpuDecoder, generation::Attention};
#[test]
fn bounded_assets_reject_corruption() {
    let dir = common::fixture();
    let a = common::assets(&dir);
    assert_eq!(a.weights.names().count(), 21);
    drop(a);
    let path = dir.path().join("model.safetensors");
    let valid = std::fs::read(&path).unwrap();
    for bytes in [&valid[..4], &valid[..valid.len() - 1]] {
        std::fs::write(&path, bytes).unwrap();
        assert!(Assets::open(dir.path()).is_err());
    }
    let mut extra = valid.clone();
    extra.push(0);
    std::fs::write(&path, extra).unwrap();
    assert!(Assets::open(dir.path()).is_err());
    std::fs::write(&path, valid).unwrap();
    let config_path = dir.path().join("config.json");
    let mut c: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&config_path).unwrap()).unwrap();
    c["head_dim"] = serde_json::json!(96);
    std::fs::write(config_path, serde_json::to_vec(&c).unwrap()).unwrap();
    assert!(Assets::open(dir.path()).is_err());
}
#[test]
fn scalar_attention_cache_is_transactional() {
    let dir = common::fixture();
    let mut model = CpuDecoder::new(common::assets(&dir), 64).unwrap();
    model
        .forward(&[1, 2, 3], Attention::Causal, true, true)
        .unwrap();
    let expected = model
        .forward(&[4, 5], Attention::Bidirectional, false, false)
        .unwrap();
    assert_eq!(model.cache_len(), 3);
    let actual = model
        .forward(&[4, 5], Attention::Bidirectional, false, false)
        .unwrap();
    assert_eq!(expected, actual);
    model
        .forward(&[4, 5], Attention::Causal, true, true)
        .unwrap();
    model.crop(3).unwrap();
    let restored = model
        .forward(&[4, 5], Attention::Bidirectional, false, false)
        .unwrap();
    assert_eq!(expected, restored);
    assert!(model.crop(6).is_err());
    model.reset().unwrap();
    assert_eq!(model.cache_len(), 0);
}
