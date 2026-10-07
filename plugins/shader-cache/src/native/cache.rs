//! Shader-cache records and atomic replace-on-write persistence.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
pub const MAX_RECORD: usize = 32 * 1024 * 1024;
const MAGIC: &[u8; 8] = b"REKSHD01";
static SERIAL: AtomicU64 = AtomicU64::new(0);
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub kind: u32,
    pub name: Vec<u8>,
    pub digest: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    key: String,
    result: i32,
    compile_us: u64,
    dependencies: Vec<Dependency>,
    code_len: usize,
    warnings_len: Option<usize>,
}
#[derive(Debug)]
pub struct Record {
    pub key: String,
    pub result: i32,
    pub compile_us: u64,
    pub dependencies: Vec<Dependency>,
    pub code: Vec<u8>,
    pub warnings: Option<Vec<u8>>,
}
fn encode(r: &Record) -> std::io::Result<Vec<u8>> {
    let meta = toml::to_string(&Metadata {
        key: r.key.clone(),
        result: r.result,
        compile_us: r.compile_us,
        dependencies: r.dependencies.clone(),
        code_len: r.code.len(),
        warnings_len: r.warnings.as_ref().map(Vec::len),
    })
    .map_err(std::io::Error::other)?;
    let mut data = Vec::new();
    data.extend(MAGIC);
    data.extend((meta.len() as u32).to_le_bytes());
    data.extend(meta.as_bytes());
    data.extend(&r.code);
    if let Some(w) = &r.warnings {
        data.extend(w);
    }
    if data.len() > MAX_RECORD {
        return Err(std::io::Error::other("record too large"));
    }
    let checksum = Sha256::digest(&data);
    data.extend(checksum);
    Ok(data)
}
fn decode(bytes: &[u8], key: &str) -> Option<Record> {
    if bytes.len() < 44 || bytes.len() > MAX_RECORD + 32 || &bytes[..8] != MAGIC {
        return None;
    }
    let end = bytes.len() - 32;
    if Sha256::digest(&bytes[..end]).as_slice() != &bytes[end..] {
        return None;
    }
    let n = u32::from_le_bytes(bytes[8..12].try_into().ok()?) as usize;
    if n > 2 * 1024 * 1024 {
        return None;
    }
    let data_start = 12usize.checked_add(n)?;
    let m: Metadata = toml::from_str(std::str::from_utf8(bytes.get(12..data_start)?).ok()?).ok()?;
    if m.key != key
        || m.result < 0
        || m.code_len == 0
        || m.dependencies.len() > 512
        || m.dependencies.iter().any(|d| {
            d.name.is_empty() || d.name.len() > 32768 || d.name.contains(&0) || d.digest.len() != 64
        })
    {
        return None;
    }
    let code_end = data_start.checked_add(m.code_len)?;
    if code_end.checked_add(m.warnings_len.unwrap_or(0))? != end {
        return None;
    }
    Some(Record {
        key: m.key,
        result: m.result,
        compile_us: m.compile_us,
        dependencies: m.dependencies,
        code: bytes.get(data_start..code_end)?.to_vec(),
        warnings: m.warnings_len.map(|_| bytes[code_end..end].to_vec()),
    })
}
pub fn read(path: &Path, key: &str) -> Option<Record> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take((MAX_RECORD + 33) as u64)
        .read_to_end(&mut bytes)
        .ok()?;
    decode(&bytes, key)
}
pub fn write(path: &Path, r: &Record, used: &AtomicU64, budget: u64) -> std::io::Result<()> {
    let bytes = encode(r)?;
    let reserved = bytes.len() as u64;
    if used
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
            n.checked_add(reserved).filter(|n| *n <= budget)
        })
        .is_err()
    {
        return Ok(()); // A full cache never interferes with the original compiler.
    }
    let replaced = std::fs::metadata(path).map_or(0, |m| m.len());
    let temp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        f.write_all(&bytes)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&temp, path)
    })();
    let _ = std::fs::remove_file(temp);
    used.fetch_sub(
        if result.is_ok() {
            replaced.min(reserved)
        } else {
            reserved
        },
        Ordering::AcqRel,
    );
    result
}
pub fn usage(folder: &Path) -> u64 {
    std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "shader"))
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}
pub fn trim(folder: &Path, budget: u64) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    let mut files: Vec<_> = entries
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if p.extension()?.to_str()? != "shader" {
                return None;
            }
            let m = e.metadata().ok()?;
            Some((m.modified().ok()?, m.len(), p))
        })
        .collect();
    files.sort_by_key(|e| e.0);
    let mut total: u64 = files.iter().map(|e| e.1).sum();
    for (_, len, path) in files {
        if total <= budget {
            break;
        }
        if std::fs::remove_file(path).is_ok() {
            total = total.saturating_sub(len);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corruption_truncation_and_input_identity_are_rejected() {
        let record = Record {
            key: "a".repeat(64),
            result: 0,
            compile_us: 12,
            dependencies: vec![Dependency {
                kind: 0,
                name: b"include.h".to_vec(),
                digest: "b".repeat(64),
            }],
            code: vec![1, 2, 3],
            warnings: Some(vec![4, 0]),
        };
        let bytes = encode(&record).unwrap();
        let read = decode(&bytes, &record.key).unwrap();
        assert_eq!(read.code, record.code);
        assert_eq!(read.warnings, record.warnings);
        assert!(decode(&bytes, &"c".repeat(64)).is_none());
        for len in 0..bytes.len() {
            assert!(decode(&bytes[..len], &record.key).is_none());
        }
        let mut corrupt = bytes;
        corrupt[20] ^= 1;
        assert!(decode(&corrupt, &record.key).is_none());
    }
}
