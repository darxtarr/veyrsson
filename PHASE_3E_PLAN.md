# Phase 3e: Incremental Embedding Cache

## Goal
Skip re-embedding identical chunks between runs. Turn 2m49s full rebuild into <30s cached rebuild.

## 1. Tag and Fingerprint

Lock reproducible baseline:
```bash
git tag -a v0.3d-cpu-stable -m "Mentat Candle embedder, release build baseline"
git push origin v0.3d-cpu-stable
```

Create fingerprint for regression checks:
```bash
cargo run -p mentat -- test-embed "hello world" > tests/fingerprint.txt
```

Store:
- commit_hash
- rustc_version
- cpu_model
- os_release
- embed_checksum

Future runs matching checksum = deterministic success.

## 2. Incremental Cache Architecture

### Schema (ReDB)

New table in `mentat-store`:
```rust
pub struct EmbeddingCache {
    pub chunk_hash: [u8; 32],     // blake3
    pub dim: u16,                 // 384
    pub embedding: Vec<f32>,      // serialized directly
}
```

Index by `chunk_hash`.

### Hash Function
```rust
use blake3;

fn chunk_hash(text: &str) -> [u8; 32] {
    blake3::hash(text.as_bytes()).into()
}
```

### Pipeline Flow
```
┌──────────────┐
│ ingest file  │
└──────┬───────┘
       │
       ▼
 ┌──────────────┐
 │ chunk text   │
 └──────┬───────┘
       │
       ▼
┌────────────────────────┐
│ compute hash           │
│ if exists in cache →↩︎ │───────┐
└──────────┬─────────────┘       │
           ▼                     │
    run embedder → store result  │
           │                     │
           ▼                     │
  cache.insert(hash, embedding)  │
           │                     │
           ▼                     │
   index embedding into store ───┘
```

### Code Sketch
```rust
let h = chunk_hash(&chunk.text);

if let Some(vec) = store.get_cached_embedding(&h)? {
    embeddings.push(vec);
} else {
    let vec = embedder.embed_text(&chunk.text)?;
    store.put_cached_embedding(&h, &vec)?;
    embeddings.push(vec);
}
```

### ReDB Table Setup
```rust
pub const CF_CACHE: &str = "embedding_cache";

impl Store {
    pub fn get_cached_embedding(&self, hash: &[u8; 32]) -> Result<Option<Vec<f32>>> { ... }
    pub fn put_cached_embedding(&self, hash: &[u8; 32], emb: &[f32]) -> Result<()> { ... }
}
```

### CLI Feedback
When indexing:
```
[index] File 4/33: ...
  cache hit 42 / miss 5
```

Persist `.cache_stats.json` after each run with totals.

## 3. Validation Script

`scripts/bench_cache.sh`:
```bash
#!/usr/bin/env bash
cargo run -p mentat --release -- index . > bench_cache.log
sleep 1
cargo run -p mentat --release -- index . >> bench_cache.log
grep "cache hit" bench_cache.log
```

Expected: second run ≈ 10–20s, >95% hits.

## 4. Documentation

`docs/incremental_indexing.md`:
- Phase: 3e
- Purpose: reuse embeddings for unchanged chunks
- Key: blake3(chunk) → [f32;384]
- Benchmark: full 2m49s → cached <30s

## Expected Impact

- First run: 2m49s (unchanged)
- Second run (no changes): ~10-20s (cache hits)
- Incremental updates: proportional to changed files only
- Cache invalidation: automatic via content hashing
