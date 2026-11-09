mod common;
use claw_weight_space::{edit::Patch, gguf::Gguf, hash_bytes, hash_file};
use serde_json::json;
use std::{fs, io::Write, path::Path};
use zip::write::SimpleFileOptions;

fn string(out: &mut Vec<u8>, s: &str) {
    out.extend((s.len() as u64).to_le_bytes());
    out.extend(s.as_bytes());
}
fn gguf(path: &Path, words: &[u16]) {
    let mut b = b"GGUF".to_vec();
    b.extend(3u32.to_le_bytes());
    b.extend(2u64.to_le_bytes());
    b.extend(1u64.to_le_bytes());
    string(&mut b, "general.architecture");
    b.extend(8u32.to_le_bytes());
    string(&mut b, "test");
    for (name, offset) in [
        ("blk.15.ffn_down.weight", 0u64),
        ("token_embd.weight", 32u64),
    ] {
        string(&mut b, name);
        b.extend(2u32.to_le_bytes());
        b.extend(3u64.to_le_bytes());
        b.extend(2u64.to_le_bytes());
        b.extend(30u32.to_le_bytes());
        b.extend(offset.to_le_bytes());
    }
    b.resize(b.len().next_multiple_of(32), 0);
    for w in words {
        b.extend(w.to_le_bytes());
    }
    b.resize(b.len() + 20, 0);
    for _ in 0..6 {
        b.extend(0x3f80u16.to_le_bytes());
    }
    fs::write(path, b).unwrap();
}
fn npy(descr: &str, count: usize, data: &[u8]) -> Vec<u8> {
    let mut header =
        format!("{{'descr': '{descr}', 'fortran_order': False, 'shape': ({count},), }}");
    while !(10 + header.len() + 1).is_multiple_of(64) {
        header.push(' ');
    }
    header.push('\n');
    let mut b = b"\x93NUMPY\x01\x00".to_vec();
    b.extend((header.len() as u16).to_le_bytes());
    b.extend(header.as_bytes());
    b.extend(data);
    b
}
fn patch_files(
    root: &Path,
    indices: &[i32],
    values: &[u16],
    wrong_hash: bool,
) -> (std::path::PathBuf, std::path::PathBuf) {
    let original = [0x3f80u16; 6];
    let mut modified = original;
    for (&idx, &v) in indices.iter().zip(values) {
        if (0..6).contains(&idx) {
            modified[idx as usize] = v;
        }
    }
    let raw = |w: &[u16]| w.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<_>>();
    let m = root.join("manifest.json");
    fs::write(&m,serde_json::to_vec(&json!({"tensors":[{
        "name":"model.language_model.layers.15.mlp.down_proj.weight","shape":[2,3],"dtype":"bfloat16","form":"sparse","changed_entries":indices.len(),
        "base_sha256":if wrong_hash {"0".repeat(64)} else {hash_bytes(&raw(&original))},"edited_sha256":hash_bytes(&raw(&modified))
    }],"changed_tensors":1,"changed_entries_total":indices.len()})).unwrap()).unwrap();
    let p = root.join("edit.npz");
    let mut zip = zip::ZipWriter::new(fs::File::create(&p).unwrap());
    zip.start_file("t0_idx.npy", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(&npy(
        "<i4",
        indices.len(),
        &indices
            .iter()
            .flat_map(|i| i.to_le_bytes())
            .collect::<Vec<_>>(),
    ))
    .unwrap();
    zip.start_file("t0_val.npy", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(&npy("<u2", values.len(), &raw(values)))
        .unwrap();
    zip.finish().unwrap();
    (m, p)
}
#[test]
fn exact_copy_and_full_audit_reject_collateral_changes() {
    let root = common::tempdir();
    let src = root.path().join("source");
    fs::create_dir(&src).unwrap();
    let base = src.join("base.gguf");
    gguf(&base, &[0x3f80; 6]);
    let original = hash_file(&base).unwrap();
    let (m, p) = patch_files(&src, &[1, 4], &[0x4000, 0x4040], false);
    let patch = Patch::load(&m, &p).unwrap();
    let (edited, receipt) = patch.apply(&base, &root.path().join("out")).unwrap();
    assert_eq!(receipt.changed_entries, 2);
    assert_eq!(receipt.bytes_compared, fs::metadata(&base).unwrap().len());
    assert_eq!(hash_file(&base).unwrap(), original);
    assert!(patch.apply(&base, &root.path().join("out")).is_err());
    let good = fs::read(&edited).unwrap();
    for offset in [
        4,
        Gguf::inspect(&base)
            .unwrap()
            .tensor("token_embd.weight")
            .unwrap()
            .offset as usize,
        good.len() - 1,
    ] {
        let mut bad = good.clone();
        bad[offset] ^= 1;
        fs::write(&edited, bad).unwrap();
        assert!(patch.audit(&base, &edited).is_err());
    }
    fs::write(&edited, &good[..good.len() - 1]).unwrap();
    assert!(patch.audit(&base, &edited).is_err());
    assert_eq!(hash_file(&base).unwrap(), original);
}
#[test]
fn wrong_revision_and_output_alias_fail_without_writing() {
    let root = common::tempdir();
    let src = root.path().join("source");
    fs::create_dir(&src).unwrap();
    let base = src.join("base.gguf");
    gguf(&base, &[0x3f80; 6]);
    let original = fs::read(&base).unwrap();
    let (m, p) = patch_files(&src, &[1], &[0x4000], true);
    let patch = Patch::load(&m, &p).unwrap();
    assert!(patch.apply(&base, &root.path().join("bad")).is_err());
    assert!(!root.path().join("bad").exists());
    assert!(patch.apply(&base, &src.join("out")).is_err());
    #[cfg(unix)]
    {
        let alias = root.path().join("alias");
        std::os::unix::fs::symlink(&src, &alias).unwrap();
        assert!(patch.apply(&base, &alias.join("out")).is_err());
    }
    assert_eq!(fs::read(&base).unwrap(), original);
}
#[test]
fn invalid_sparse_and_nonfinite_edits_rejected() {
    for (indices, values) in [
        (vec![1, 1], vec![0x4000, 0x4040]),
        (vec![-1], vec![0x4000]),
        (vec![6], vec![0x4000]),
        (vec![1], vec![0x7f80]),
        (vec![1], vec![0x7fc0]),
    ] {
        let root = common::tempdir();
        let (m, p) = patch_files(root.path(), &indices, &values, false);
        assert!(Patch::load(&m, &p).is_err());
    }
}
#[test]
fn truncated_and_overflowing_gguf_is_rejected() {
    let root = common::tempdir();
    let path = root.path().join("model");
    gguf(&path, &[0x3f80; 6]);
    let valid = fs::read(&path).unwrap();
    for n in [0, 4, 8, 16, valid.len() - 1] {
        fs::write(&path, &valid[..n]).unwrap();
        assert!(Gguf::inspect(&path).is_err());
    }
    let mut huge = valid;
    huge[8..16].copy_from_slice(&u64::MAX.to_le_bytes());
    fs::write(&path, huge).unwrap();
    assert!(Gguf::inspect(&path).is_err());
}

#[test]
fn protected_names_quantized_targets_and_noop_patches_rejected() {
    let t = common::tempdir();
    let src = t.path().join("source");
    fs::create_dir(&src).unwrap();
    let base = src.join("base.gguf");
    gguf(&base, &[0x3f80; 6]);
    let (m, p) = patch_files(&src, &[1], &[0x4000], false);
    let mut spec: serde_json::Value = serde_json::from_slice(&fs::read(&m).unwrap()).unwrap();
    spec["tensors"][0]["name"] = "model.language_model.embed_tokens.weight".into();
    fs::write(&m, serde_json::to_vec(&spec).unwrap()).unwrap();
    assert!(Patch::load(&m, &p).is_err());
    let (m, p) = patch_files(&src, &[1], &[0x3f80], false);
    let patch = Patch::load(&m, &p).unwrap();
    assert!(patch.apply(&base, &t.path().join("noop")).is_err());
    assert!(!t.path().join("noop").exists());
    let (m, p) = patch_files(&src, &[1], &[0x4000], false);
    let patch = Patch::load(&m, &p).unwrap();
    let mut b = fs::read(&base).unwrap();
    let name = b"blk.15.ffn_down.weight";
    let pos = b.windows(name.len()).position(|x| x == name).unwrap() + name.len() + 4 + 16;
    b[pos..pos + 4].copy_from_slice(&8u32.to_le_bytes());
    fs::write(&base, b).unwrap();
    assert!(patch.apply(&base, &t.path().join("quantized")).is_err());
    assert!(!t.path().join("quantized").exists());
}
