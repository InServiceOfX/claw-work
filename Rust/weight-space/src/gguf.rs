use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;
use std::{
    collections::HashSet,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
};

const MAX_HEADER: u64 = 256 * 1024 * 1024;
const MAX_STRING: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct Tensor {
    pub name: String,
    /// GGUF fastest axis first, unlike HF shape notation.
    pub dimensions: Vec<u64>,
    pub storage_type: u32,
    pub offset: u64,
    pub span: u64,
}
impl Tensor {
    pub fn bf16_len(&self) -> Result<u64> {
        ensure!(
            self.storage_type == 30,
            "{}: only unquantized BF16 is editable (type {})",
            self.name,
            self.storage_type
        );
        self.dimensions.iter().try_fold(2u64, |n, d| {
            n.checked_mul(*d).context("tensor size overflow")
        })
    }
}
#[derive(Debug, Serialize)]
pub struct Gguf {
    pub architecture: String,
    pub alignment: u64,
    pub data_start: u64,
    pub file_len: u64,
    pub tensors: Vec<Tensor>,
}

struct Parser {
    r: BufReader<File>,
    len: u64,
    items: u64,
}
impl Parser {
    fn bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        ensure!(
            self.r
                .stream_position()?
                .checked_add(N as u64)
                .is_some_and(|x| x <= self.len.min(MAX_HEADER)),
            "header exceeds bounds"
        );
        let mut b = [0; N];
        self.r.read_exact(&mut b)?;
        Ok(b)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.bytes()?))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.bytes()?))
    }
    fn skip(&mut self, n: u64) -> Result<()> {
        let end = self
            .r
            .stream_position()?
            .checked_add(n)
            .context("offset overflow")?;
        ensure!(
            end <= self.len.min(MAX_HEADER),
            "truncated or oversized header"
        );
        self.r.seek(SeekFrom::Start(end))?;
        Ok(())
    }
    fn string(&mut self, retain: bool) -> Result<String> {
        let n = self.u64()?;
        ensure!(n <= MAX_STRING, "oversized string");
        if !retain {
            self.skip(n)?;
            return Ok(String::new());
        }
        ensure!(
            self.r
                .stream_position()?
                .checked_add(n)
                .is_some_and(|x| x <= self.len.min(MAX_HEADER)),
            "truncated string"
        );
        let mut buf = vec![0; n as usize];
        self.r.read_exact(&mut buf)?;
        String::from_utf8(buf).context("non-UTF8 GGUF name")
    }
    fn value(&mut self, ty: u32, depth: u8) -> Result<()> {
        self.items += 1;
        ensure!(
            depth < 8 && self.items <= 10_000_000,
            "metadata nesting/count limit"
        );
        match ty {
            0 | 1 | 7 => self.skip(1)?,
            2 | 3 => self.skip(2)?,
            4..=6 => self.skip(4)?,
            10..=12 => self.skip(8)?,
            8 => {
                self.string(false)?;
            }
            9 => {
                let subtype = self.u32()?;
                let n = self.u64()?;
                ensure!(n <= 10_000_000, "oversized metadata array");
                for _ in 0..n {
                    self.value(subtype, depth + 1)?;
                }
            }
            _ => bail!("invalid metadata type {ty}"),
        }
        Ok(())
    }
}

impl Gguf {
    pub fn inspect(path: &Path) -> Result<Self> {
        let f = File::open(path)?;
        let len = f.metadata()?.len();
        let mut p = Parser {
            r: BufReader::new(f),
            len,
            items: 0,
        };
        ensure!(&p.bytes::<4>()? == b"GGUF", "invalid GGUF magic");
        ensure!(p.u32()? == 3, "only little-endian GGUF v3 supported");
        let nt = p.u64()?;
        let nk = p.u64()?;
        ensure!(
            nt > 0 && nt <= 100_000 && nk <= 1_000_000,
            "invalid descriptor counts"
        );
        let mut alignment = 32u64;
        let mut architecture = String::new();
        let mut keys = HashSet::new();
        for _ in 0..nk {
            let key = p.string(true)?;
            ensure!(keys.insert(key.clone()), "duplicate metadata key");
            let ty = p.u32()?;
            match key.as_str() {
                "general.alignment" => {
                    ensure!(ty == 4, "alignment must be uint32");
                    alignment = u64::from(p.u32()?);
                }
                "general.architecture" => {
                    ensure!(ty == 8, "architecture must be string");
                    architecture = p.string(true)?;
                }
                _ => p.value(ty, 0)?,
            }
        }
        ensure!(
            alignment.is_power_of_two() && alignment <= 4096,
            "invalid GGUF alignment"
        );
        let mut tensors = Vec::with_capacity(nt as usize);
        let mut names = HashSet::new();
        for _ in 0..nt {
            let name = p.string(true)?;
            ensure!(
                !name.is_empty() && names.insert(name.clone()),
                "empty/duplicate tensor name"
            );
            let nd = p.u32()?;
            ensure!((1..=4).contains(&nd), "invalid tensor dimensions");
            let mut dimensions = Vec::new();
            let mut elements = 1u64;
            for _ in 0..nd {
                let d = p.u64()?;
                ensure!(d > 0, "zero dimension");
                elements = elements.checked_mul(d).context("shape overflow")?;
                dimensions.push(d);
            }
            let storage_type = p.u32()?;
            let offset = p.u64()?;
            ensure!(
                matches!(storage_type,0..=3|6..=30|34..=35|39..=42),
                "invalid/removed GGML storage type"
            );
            ensure!(offset % alignment == 0, "unaligned tensor offset");
            tensors.push(Tensor {
                name,
                dimensions,
                storage_type,
                offset,
                span: 0,
            });
        }
        let pos = p.r.stream_position()?;
        let data_start = pos
            .checked_add(alignment - 1)
            .context("alignment overflow")?
            & !(alignment - 1);
        ensure!(data_start < len, "missing GGUF data");
        tensors.sort_by_key(|t| t.offset);
        for i in 0..tensors.len() {
            let start = data_start
                .checked_add(tensors[i].offset)
                .context("tensor offset overflow")?;
            let end = if i + 1 < tensors.len() {
                data_start
                    .checked_add(tensors[i + 1].offset)
                    .context("tensor offset overflow")?
            } else {
                len
            };
            ensure!(
                start < end && end <= len,
                "overlapping or out-of-bounds tensor storage"
            );
            tensors[i].offset = start;
            tensors[i].span = end - start;
            let width = match tensors[i].storage_type {
                0 => Some(4),
                1 | 30 => Some(2),
                _ => None,
            };
            if let Some(width) = width {
                let bytes = tensors[i].dimensions.iter().try_fold(width, |n: u64, d| {
                    n.checked_mul(*d).context("tensor size overflow")
                })?;
                ensure!(bytes <= tensors[i].span, "overlapping/truncated tensor");
            }
        }
        Ok(Self {
            architecture,
            alignment,
            data_start,
            file_len: len,
            tensors,
        })
    }
    pub fn tensor(&self, name: &str) -> Result<&Tensor> {
        self.tensors
            .iter()
            .find(|t| t.name == name)
            .with_context(|| format!("missing tensor {name}"))
    }
    pub fn read_bf16(&self, path: &Path, t: &Tensor) -> Result<Vec<u8>> {
        let n = t.bf16_len()?;
        ensure!(n <= 256 * 1024 * 1024 && n <= t.span, "tensor read limit");
        let mut f = File::open(path)?;
        f.seek(SeekFrom::Start(t.offset))?;
        let mut b = vec![0; n as usize];
        f.read_exact(&mut b)?;
        Ok(b)
    }
}
