//! Streaming synthetic-canary check of image archives, including compressed layer blobs.
use crate::Result;
use std::io::{BufRead, BufReader, Read};
fn contains(mut input: impl Read, needle: &[u8]) -> Result<bool> {
    if needle.len() < 8 || needle.len() > 256 {
        return Err("Use an 8..256-byte synthetic canary".into());
    }
    let mut overlap = Vec::new();
    let mut buf = [0; 65536];
    loop {
        let n = input.read(&mut buf)?;
        if n == 0 {
            return Ok(false);
        }
        overlap.extend_from_slice(&buf[..n]);
        if memchr::memmem::find(&overlap, needle).is_some() {
            return Ok(true);
        }
        if overlap.len() > needle.len() {
            overlap.drain(..overlap.len() - needle.len() + 1);
        }
    }
}
pub fn image_archive_contains(reader: impl Read, needle: &[u8]) -> Result<bool> {
    let mut archive = tar::Archive::new(reader);
    let mut count = 0;
    for entry in archive.entries()? {
        count += 1;
        if count > 100_000 {
            return Err("Image archive entry bound exceeded".into());
        }
        let entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let mut input = BufReader::new(entry);
        let head = input.fill_buf()?;
        let found = if head.starts_with(&[0x1f, 0x8b]) {
            contains(flate2::read::GzDecoder::new(input), needle)?
        } else if head.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) {
            contains(zstd::stream::read::Decoder::new(input)?, needle)?
        } else {
            contains(input, needle)?
        };
        if found {
            return Ok(true);
        }
    }
    Ok(false)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finds_compressed_layer_and_cross_chunk_canaries() {
        use std::io::Write;
        let marker = b"synthetic-layer-canary";
        let mut payload = vec![b'a'; 65530];
        payload.extend_from_slice(marker);
        assert!(contains(&payload[..], marker).unwrap());
        for gzip in [true, false] {
            let mut compressed = Vec::new();
            if gzip {
                let mut w =
                    flate2::write::GzEncoder::new(&mut compressed, flate2::Compression::fast());
                w.write_all(&payload).unwrap();
                w.finish().unwrap();
            } else {
                compressed = zstd::stream::encode_all(&payload[..], 1).unwrap();
            }
            let mut outer = tar::Builder::new(Vec::new());
            let mut header = tar::Header::new_gnu();
            header.set_size(compressed.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            outer
                .append_data(&mut header, "blobs/layer", &compressed[..])
                .unwrap();
            let archive = outer.into_inner().unwrap();
            assert!(image_archive_contains(&archive[..], marker).unwrap());
            assert!(!image_archive_contains(&archive[..], b"absent-marker-value").unwrap());
        }
    }
}
