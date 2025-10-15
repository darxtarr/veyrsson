# Veyrsson Development Session Notes

## Session Summary
Implemented core RAG pipeline: Ingest → Index → Retrieve with HNSW acceleration.

## What Was Built

### Phase 1: File Ingestion (mentat-ingest)
- Walks directories, computes BLAKE3 hashes for files
- Outputs JSON manifest with file metadata
- Built-in ignore patterns (.git/, target/, node_modules/, etc.)
- Custom .ingestignore support (gitignore-style)
- Command: `mentat ingest <path>`

### Phase 2: Indexing Pipeline
**Chunker (mentat-chunker)**
- Splits text files into ~6000 byte chunks with 10% overlap
- Skips binary files (NUL byte detection)
- Generates BLAKE3 hash per chunk

**Embedder (mentat-embedder)**
- Deterministic pseudo-embedder (384-d vectors)
- Uses BLAKE3-seeded xorshift64* PRNG
- Placeholder for real model (Candle + BGE-small planned)

**Store (mentat-store)**
- ReDB key-value database at ./index/kv.redb
- Three tables: files, chunks, embeds
- Stores file metadata, chunk metadata, embeddings

**Integration**
- Command: `mentat index <path>` runs full pipeline
- Creates ./index/kv.redb with all data

### Phase 3: Retrieval
**Phase 3a: Brute Cosine Search**
- O(N) scan across all vectors
- Cosine similarity ranking
- Command: `mentat search <query>`
- Baseline for verification

**Phase 3b: HNSW Acceleration**
- Fast approximate nearest neighbor search
- hnsw_rs v0.3 with cosine distance
- Parameters: m=16, ef_c=200, 16 layers
- Commands:
  - `mentat build-hnsw` - build index
  - `mentat search-hnsw <query>` - fast search
- Currently rebuilds from ReDB on load (fast enough for now)

## Current State
- Working end-to-end RAG pipeline
- 26 source files indexed
- 22 chunks with embeddings
- HNSW index built and tested
- All commands functional

## File Structure
```
crates/
├── ingest/          # File scanning + ignore patterns
├── chunker/         # Text splitting (6KB chunks, 10% overlap)
├── embedder/        # Pseudo-embedder (384-d deterministic)
├── store/           # ReDB storage layer
├── retriever/       # Brute + HNSW search
├── condenser/       # (stub - future)
├── reasoner/        # (stub - future)
├── planner/         # (stub - future)
└── mnematode_client/ # (stub - future)
mentat-bin/          # CLI entry point
index/               # ReDB database + HNSW files
```

## Next Session: Phase 3c - Real Embeddings

### Goal
Replace pseudo-embedder with real model using Candle framework.

### Plan
1. **Add Candle dependencies** to mentat-embedder/Cargo.toml:
   ```toml
   candle-core = "0.7"
   candle-nn = "0.7"
   candle-transformers = "0.7"
   hf-hub = "0.3"
   tokenizers = "0.20"
   ```

2. **Download BGE-small model**:
   - Model: BAAI/bge-small-en-v1.5
   - Size: ~133MB
   - Embedding dim: 384 (matches current D constant)

3. **Update embed_text() in embedder/src/lib.rs**:
   - Keep same signature: `pub fn embed_text(text: &str) -> Result<[f32; D]>`
   - Load model once, cache in static/lazy_static
   - Use Candle to run inference
   - No changes needed upstream (store, retriever all work as-is)

4. **Test**:
   - Delete old index: `rm -rf index/`
   - Rebuild: `mentat index .`
   - Build HNSW: `mentat build-hnsw`
   - Search: `mentat search-hnsw "your query"`
   - Expect better similarity scores (semantic vs random)

### Important Notes
- Keep D=384 constant (BGE-small native dimension)
- Embedding quality will dramatically improve search results
- Consider caching model loading (first run slower)
- May want to add progress indicator for indexing

### Known Issues
- HNSW serialization currently rebuilds from ReDB (acceptable for <10K chunks)
- For larger scales, implement proper save/load of HNSW index

### Architecture Design
- Single-threaded, deterministic builds
- Offline indexing (no incremental updates yet)
- Clean separation: storage layer (ReDB) + search layer (HNSW)
- Ready for future phases: condenser, reasoner, planner

## Dependencies Installed This Session
All from crates.io, verified maintainers:
- redb v2: Embedded database
- hnsw_rs v0.3: Vector search
- blake3 v1: Hashing
- bincode v1: Serialization
- bytemuck v1: Zero-copy casts
- globset v0.4: Pattern matching
- memchr v2: Binary detection
- walkdir v2: Directory traversal

## Commands Reference
```bash
mentat ingest <path>        # Scan files, output manifest
mentat index <path>         # Full pipeline: chunk + embed + store
mentat search <query>       # Brute force cosine search
mentat build-hnsw           # Build HNSW index
mentat search-hnsw <query>  # Fast approximate search
```

## Testing
```bash
# Full workflow
cargo build
cargo run -p mentat -- index .
cargo run -p mentat -- build-hnsw
cargo run -p mentat -- search-hnsw "index creation"
```

## Code Quality
- No warnings (after cleanup)
- Clean error handling with anyhow::Result
- Deterministic (seeded PRNG, stable iteration)
- Type-safe (strong typing, no unsafe except controlled conversions)

Good luck with Phase 3c - real embeddings will make this shine!
