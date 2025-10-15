//! Deterministic, lightweight chunker.
//! Strategy: split text files into ~6000 byte spans with 10% overlap.
//! Skips binary-ish data (NUL present) and tiny files emitted as single chunk.

use anyhow::Result;
use memchr::memchr;
use serde::Serialize;
use std::{fs, path::Path};

#[derive(Serialize, Clone)]
pub struct Span {
    pub path: String,
    pub start: usize,
    pub end: usize,
    pub hash: String, // blake3 of slice
}

const TARGET_BYTES: usize = 6000;
const OVERLAP_BYTES: usize = TARGET_BYTES / 10;

pub fn chunk_file<P: AsRef<Path>>(path: P) -> Result<Vec<Span>> {
    let path_ref = path.as_ref();
    let data = fs::read(path_ref)?;
    // crude binary gate: NUL byte present -> skip
    if memchr(0, &data).is_some() {
        return Ok(vec![]);
    }
    if data.is_empty() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    let mut off = 0usize;
    while off < data.len() {
        let end = (off + TARGET_BYTES).min(data.len());
        let slice = &data[off..end];
        let hash = blake3::hash(slice).to_hex().to_string();
        out.push(Span {
            path: display(path_ref),
            start: off,
            end,
            hash,
        });
        if end == data.len() { break; }
        let step = TARGET_BYTES - OVERLAP_BYTES;
        off = off.saturating_add(step);
    }
    Ok(out)
}

pub fn chunk_many<P: AsRef<Path>>(roots: &[P]) -> Result<Vec<Span>> {
    let mut all = Vec::new();
    for r in roots {
        let spans = chunk_file(r)?;
        all.extend(spans);
    }
    Ok(all)
}

fn display(p: &Path) -> String {
    let mut s = p.display().to_string();
    if cfg!(windows) { s = s.replace('\\', "/"); }
    s
}
