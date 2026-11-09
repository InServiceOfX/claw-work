use claw_lm::{Assets, Config};
use serde_json::json;
use std::{collections::BTreeMap, io::Write};
pub fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let config = json!({"model_type":"nemotron_labs_diffusion","hidden_size":32,"intermediate_size":64,"num_hidden_layers":2,"num_attention_heads":2,"num_key_value_heads":1,"head_dim":128,"vocab_size":32,"max_position_embeddings":64,"rms_norm_eps":0.00001,"mask_token_id":31,"eos_token_id":30,"hidden_act":"silu","dlm_paradigm":"bidirectional","sliding_window":null,"rope_parameters":{"rope_type":"yarn","rope_theta":1000000.,"factor":4.,"original_max_position_embeddings":16,"beta_fast":32.,"beta_slow":1.,"mscale":1.,"mscale_all_dim":1.,"llama_4_scaling_beta":0.1}});
    std::fs::write(
        dir.path().join("config.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    let c: Config = serde_json::from_value(config).unwrap();
    c.validate().unwrap();
    let mut header = BTreeMap::new();
    let mut bytes = Vec::new();
    let mut state = 71u32;
    for (name, shape) in c.tensor_shapes().unwrap() {
        let start = bytes.len();
        let size: usize = shape.iter().product();
        for _ in 0..size {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            let value = if name.contains("norm") {
                1.
            } else {
                ((state >> 8) as f32 / 16777216. - 0.5)
                    * if name.contains("embed") { 1. } else { 0.1 }
            };
            let bits = value.to_bits();
            let bf = ((bits.wrapping_add(0x7fff + ((bits >> 16) & 1))) >> 16) as u16;
            bytes.extend_from_slice(&bf.to_le_bytes());
        }
        header.insert(
            name,
            json!({"dtype":"BF16","shape":shape,"data_offsets":[start,bytes.len()]}),
        );
    }
    let mut text = serde_json::to_vec(&header).unwrap();
    while text.len() % 8 != 0 {
        text.push(b' ');
    }
    let mut file = std::fs::File::create(dir.path().join("model.safetensors")).unwrap();
    file.write_all(&(text.len() as u64).to_le_bytes()).unwrap();
    file.write_all(&text).unwrap();
    file.write_all(&bytes).unwrap();
    dir
}
#[allow(dead_code)]
pub fn assets(dir: &tempfile::TempDir) -> Assets {
    Assets::open(dir.path()).unwrap()
}
