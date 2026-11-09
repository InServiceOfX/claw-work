use crate::{gguf::Gguf, hash_bytes, write_json_new};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File},
    io::{BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TensorEdit {
    pub name: String,
    pub shape: Vec<u64>,
    pub dtype: String,
    pub form: String,
    pub changed_entries: usize,
    pub base_sha256: String,
    pub edited_sha256: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub tensors: Vec<TensorEdit>,
    pub changed_tensors: usize,
    pub changed_entries_total: usize,
}
pub struct Patch {
    pub manifest: Manifest,
    arrays: Vec<(Vec<usize>, Vec<u16>)>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TensorReceipt {
    pub name: String,
    pub base_sha256: String,
    pub edited_sha256: String,
    pub changed_entries: usize,
    pub relative_frobenius_change: f64,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub base_sha256: String,
    pub edited_sha256: String,
    pub bytes_compared: u64,
    pub changed_bytes: u64,
    pub changed_entries: usize,
    pub tensors: Vec<TensorReceipt>,
}

fn npy(bytes: &[u8], descr: &str, width: usize, count: usize) -> Result<Vec<u8>> {
    ensure!(
        bytes.len() >= 10 && &bytes[..6] == b"\x93NUMPY",
        "invalid NPY magic"
    );
    let (start, n) = match bytes[6..8] {
        [1, 0] => (10, u16::from_le_bytes(bytes[8..10].try_into()?) as usize),
        [2, 0] => {
            ensure!(bytes.len() >= 12, "truncated NPY");
            (12, u32::from_le_bytes(bytes[8..12].try_into()?) as usize)
        }
        _ => bail!("unsupported NPY version"),
    };
    ensure!(
        n <= 8192 && start + n <= bytes.len(),
        "invalid NPY header length"
    );
    let header = std::str::from_utf8(&bytes[start..start + n])?;
    // Parse this deliberately small data format; never eval Python header text.
    let normalized = header.replace('"', "'").replace(' ', "");
    ensure!(
        normalized.contains(&format!("'descr':'{descr}'"))
            && normalized.contains("'fortran_order':False"),
        "unsupported NPY dtype/order"
    );
    let shape = normalized
        .split("'shape':(")
        .nth(1)
        .context("missing NPY shape")?
        .split(')')
        .next()
        .context("bad NPY shape")?;
    let dims: Vec<_> = shape.split(',').filter(|s| !s.is_empty()).collect();
    ensure!(
        dims.len() == 1 && dims[0].parse::<usize>()? == count,
        "NPY count/shape mismatch"
    );
    let payload = count.checked_mul(width).context("NPY length overflow")?;
    ensure!(
        start + n + payload == bytes.len(),
        "truncated or trailing NPY data"
    );
    Ok(bytes[start + n..].to_vec())
}

pub fn gguf_name(name: &str) -> Result<String> {
    let layer = name
        .strip_prefix("model.language_model.layers.")
        .or_else(|| name.strip_prefix("model.layers."))
        .context("unsupported model tensor prefix")?;
    let layer = layer
        .strip_suffix(".mlp.down_proj.weight")
        .context("only MLP down_proj may be edited")?;
    ensure!(
        !layer.is_empty() && layer.bytes().all(|b| b.is_ascii_digit()),
        "invalid layer"
    );
    let layer: usize = layer.parse()?;
    ensure!(layer < 1024, "invalid layer");
    Ok(format!("blk.{layer}.ffn_down.weight"))
}

impl Patch {
    pub fn load(manifest: &Path, archive: &Path) -> Result<Self> {
        ensure!(
            fs::metadata(manifest)?.len() <= 1024 * 1024,
            "manifest too large"
        );
        let manifest: Manifest = serde_json::from_reader(File::open(manifest)?)?;
        ensure!(
            !manifest.tensors.is_empty()
                && manifest.tensors.len() <= 64
                && manifest.changed_tensors == manifest.tensors.len(),
            "invalid tensor count"
        );
        ensure!(
            fs::metadata(archive)?.len() <= 64 * 1024 * 1024,
            "patch archive too large"
        );
        let mut z = zip::ZipArchive::new(File::open(archive)?)?;
        ensure!(
            z.len() == manifest.tensors.len() * 2,
            "unexpected NPZ members"
        );
        let mut arrays = Vec::new();
        let mut names = HashSet::new();
        let mut total = 0usize;
        for (i, t) in manifest.tensors.iter().enumerate() {
            ensure!(
                t.form == "sparse" && t.dtype == "bfloat16" && t.shape.len() == 2,
                "only sparse BF16 matrix edits supported"
            );
            ensure!(names.insert(gguf_name(&t.name)?), "duplicate target tensor");
            let elements = t.shape.iter().try_fold(1u64, |n, d| {
                ensure!(*d > 0, "zero patch dimension");
                n.checked_mul(*d).context("patch shape overflow")
            })?;
            ensure!(
                elements <= 128 * 1024 * 1024
                    && t.changed_entries > 0
                    && t.changed_entries as u64 <= elements,
                "invalid patch size"
            );
            for h in [&t.base_sha256, &t.edited_sha256] {
                ensure!(
                    h.len() == 64
                        && h.bytes()
                            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                    "invalid SHA-256"
                );
            }
            let mut read_array = |name: &str, descr: &str, width: usize| -> Result<Vec<u8>> {
                let f = z.by_name(name)?;
                ensure!(f.size() <= 64 * 1024 * 1024, "oversized compressed array");
                let mut b = Vec::new();
                f.take(64 * 1024 * 1024 + 1).read_to_end(&mut b)?;
                ensure!(b.len() <= 64 * 1024 * 1024, "NPZ decompression limit");
                npy(&b, descr, width, t.changed_entries)
            };
            let idx = read_array(&format!("t{i}_idx.npy"), "<i4", 4)?;
            let val = read_array(&format!("t{i}_val.npy"), "<u2", 2)?;
            let indices: Vec<usize> = idx
                .chunks_exact(4)
                .map(|b| i32::from_le_bytes(b.try_into().expect("chunk")))
                .map(|n| usize::try_from(n).context("negative sparse index"))
                .collect::<Result<_>>()?;
            ensure!(
                indices.iter().all(|n| (*n as u64) < elements)
                    && indices.windows(2).all(|p| p[0] < p[1]),
                "duplicate, unordered or out-of-bounds sparse index"
            );
            let values: Vec<u16> = val
                .chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect();
            ensure!(
                values.iter().all(|b| b & 0x7f80 != 0x7f80),
                "nonfinite BF16 patch value"
            );
            total = total
                .checked_add(indices.len())
                .context("patch total overflow")?;
            ensure!(total <= 8_000_000, "total sparse entry limit");
            arrays.push((indices, values));
        }
        ensure!(
            total == manifest.changed_entries_total,
            "manifest changed count mismatch"
        );
        Ok(Self { manifest, arrays })
    }

    fn prepared(&self, base: &Path) -> Result<Vec<Prepared>> {
        let g = Gguf::inspect(base)?;
        let mut result = Vec::new();
        let mut total_bytes = 0u64;
        for (spec, (idx, val)) in self.manifest.tensors.iter().zip(&self.arrays) {
            let name = gguf_name(&spec.name)?;
            let tensor = g.tensor(&name)?;
            total_bytes = total_bytes
                .checked_add(tensor.bf16_len()?)
                .context("edit size overflow")?;
            ensure!(
                total_bytes <= 512 * 1024 * 1024,
                "total edited tensor memory limit"
            );
            ensure!(
                tensor.dimensions == spec.shape.iter().rev().copied().collect::<Vec<_>>(),
                "{name}: shape mismatch"
            );
            let original = g.read_bf16(base, tensor)?;
            ensure!(
                hash_bytes(&original) == spec.base_sha256,
                "{name}: base tensor hash mismatch; wrong revision"
            );
            ensure!(
                original
                    .chunks_exact(2)
                    .all(|b| u16::from_le_bytes([b[0], b[1]]) & 0x7f80 != 0x7f80),
                "nonfinite base tensor"
            );
            let mut edited = original.clone();
            let mut bytes = BTreeMap::new();
            for (&index, &value) in idx.iter().zip(val) {
                let start = index * 2;
                let replacement = value.to_le_bytes();
                ensure!(
                    original[start..start + 2] != replacement,
                    "no-op entry in sparse patch"
                );
                edited[start..start + 2].copy_from_slice(&replacement);
                for (j, v) in replacement.iter().enumerate() {
                    bytes.insert(tensor.offset + start as u64 + j as u64, *v);
                }
            }
            ensure!(
                hash_bytes(&edited) == spec.edited_sha256,
                "{name}: edited tensor hash mismatch"
            );
            let mut delta = 0f64;
            let mut norm = 0f64;
            for (b, e) in original.chunks_exact(2).zip(edited.chunks_exact(2)) {
                let bf = |x: &[u8]| {
                    f64::from(f32::from_bits(
                        u32::from(u16::from_le_bytes([x[0], x[1]])) << 16,
                    ))
                };
                let (b, e) = (bf(b), bf(e));
                delta += (b - e).powi(2);
                norm += b.powi(2);
            }
            ensure!(norm > 0., "zero base tensor norm");
            result.push(Prepared {
                offset: tensor.offset,
                edited,
                bytes,
                receipt: TensorReceipt {
                    name,
                    base_sha256: spec.base_sha256.clone(),
                    edited_sha256: spec.edited_sha256.clone(),
                    changed_entries: idx.len(),
                    relative_frobenius_change: (delta / norm).sqrt(),
                },
            });
        }
        Ok(result)
    }

    /// Validate all expected entries and compare every byte, including metadata/padding.
    pub fn audit(&self, base: &Path, edited: &Path) -> Result<Receipt> {
        let prepared = self.prepared(base)?;
        let g = Gguf::inspect(edited)?;
        let mut allowed = BTreeMap::new();
        for p in &prepared {
            let t = g.tensor(&p.receipt.name)?;
            ensure!(t.offset == p.offset, "edited tensor offset changed");
            ensure!(
                g.read_bf16(edited, t)? == p.edited,
                "edited tensor differs from expected patch"
            );
            allowed.extend(p.bytes.iter().map(|(k, v)| (*k, *v)));
        }
        let mut a = BufReader::new(File::open(base)?);
        let mut b = BufReader::new(File::open(edited)?);
        let len = a.get_ref().metadata()?.len();
        ensure!(
            b.get_ref().metadata()?.len() == len,
            "checkpoint length changed"
        );
        let mut ab = vec![0; 1024 * 1024];
        let mut bb = vec![0; ab.len()];
        let mut offset = 0u64;
        let mut changed = 0u64;
        let mut ah = Sha256::new();
        let mut bh = Sha256::new();
        while offset < len {
            let n = (len - offset).min(ab.len() as u64) as usize;
            a.read_exact(&mut ab[..n])?;
            b.read_exact(&mut bb[..n])?;
            ah.update(&ab[..n]);
            bh.update(&bb[..n]);
            for i in 0..n {
                if ab[i] != bb[i] {
                    ensure!(
                        allowed.get(&(offset + i as u64)) == Some(&bb[i]),
                        "unauthorized byte difference at {}",
                        offset + i as u64
                    );
                    changed += 1;
                }
            }
            offset += n as u64;
        }
        ensure!(changed > 0, "no changed checkpoint bytes");
        Ok(Receipt {
            base_sha256: format!("{:x}", ah.finalize()),
            edited_sha256: format!("{:x}", bh.finalize()),
            bytes_compared: len,
            changed_bytes: changed,
            changed_entries: self.manifest.changed_entries_total,
            tensors: prepared.into_iter().map(|p| p.receipt).collect(),
        })
    }

    pub fn apply(&self, base: &Path, out_dir: &Path) -> Result<(PathBuf, Receipt)> {
        crate::ensure_external_output(out_dir)?;
        let source = fs::canonicalize(base)?;
        ensure!(source.is_file(), "source must be a regular file");
        let parent = fs::canonicalize(out_dir.parent().context("output needs parent")?)?;
        ensure!(
            !parent.starts_with(source.parent().context("source parent")?),
            "output must be outside source directory"
        );
        let initial_hash = crate::hash_file(&source)?;
        let prepared = self.prepared(&source)?; // Reject all invalid edits before any output is made.
        fs::create_dir(out_dir).context("output directory must be fresh")?;
        let mut temp = tempfile::NamedTempFile::new_in(out_dir)?;
        let mut input = File::open(&source)?;
        std::io::copy(&mut input, &mut temp)?;
        for p in prepared {
            temp.seek(SeekFrom::Start(p.offset))?;
            temp.write_all(&p.edited)?;
        }
        temp.flush()?;
        temp.as_file().sync_all()?;
        let receipt = self.audit(&source, temp.path())?;
        ensure!(
            receipt.base_sha256 == initial_hash,
            "source changed during reconstruction"
        );
        let output = out_dir.join("model.gguf");
        temp.persist_noclobber(&output)
            .context("checkpoint publish failed")?;
        write_json_new(&out_dir.join("receipt.json"), &receipt)?;
        Ok((output, receipt))
    }
}
struct Prepared {
    offset: u64,
    edited: Vec<u8>,
    bytes: BTreeMap<u64, u8>,
    receipt: TensorReceipt,
}
