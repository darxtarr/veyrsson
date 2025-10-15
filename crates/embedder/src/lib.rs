//! Pseudo-embedder: deterministic 384-d float vector from text bytes.
//! Seeded by blake3(text), PRNG = xorshift64* -> map to [-1,1].

use anyhow::Result;
use blake3::Hasher;

pub const D: usize = 384;

pub fn embed_text(text: &str) -> Result<[f32; D]> {
    // seed from blake3 of text
    let mut h = Hasher::new();
    h.update(text.as_bytes());
    let seed = u64::from_le_bytes(h.finalize().as_bytes()[..8].try_into().unwrap());
    let mut x = seed | 1; // avoid zero
    let mut out = [0f32; D];
    for i in 0..D {
        // xorshift64*
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        let v = x.wrapping_mul(2685821657736338717);
        // map to [-1, 1]
        let f = ((v as f64 / u64::MAX as f64) * 2.0 - 1.0) as f32;
        out[i] = f;
    }
    Ok(out)
}
