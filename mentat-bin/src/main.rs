use std::{env, fs, path::Path};
use anyhow::Result;

fn main() {
    if let Err(e) = real_main() {
        eprintln!("Error: {e:?}");
    }
}

fn real_main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("ingest") => {
            let target = args.get(2).map(String::as_str).unwrap_or(".");
            let chunks = mentat_ingest::ingest(target)?;
            let _ = mentat_ingest::dump_json(&chunks);
            println!("Ingested {} files", chunks.len());
        }
        Some("index") => {
            let target = args.get(2).map(String::as_str).unwrap_or(".");
            run_index(target)?;
        }
        Some("search") => {
            let q = args.get(2).map(String::as_str).unwrap_or("");
            let retr = mentat_retriever::Retriever::open_default()?;
            let results = retr.search(q, 5)?;
            println!("Top results for: \"{}\"", q);
            for (id, sim) in results {
                println!("{:6.3}  {}", sim, id);
            }
        }
        Some("build-hnsw") => {
            let mut retr = mentat_retriever::Retriever::open_default()?;
            retr.build_hnsw("index/embeds")?;
        }
        Some("search-hnsw") => {
            let q = args.get(2).map(String::as_str).unwrap_or("");
            let mut retr = mentat_retriever::Retriever::open_default()?;
            retr.load_hnsw("index/embeds.hnsw")?;
            let results = retr.search(q, 5)?;
            println!("HNSW results for: \"{}\"", q);
            for (i, d) in results {
                println!("{:6.3}  id[{}]", d, i);
            }
        }
        _ => {
            println!("mentat veyrsson — condensed stub");
            println!("USAGE:");
            println!("  mentat ingest <path>   # list files + hashes (json manifest)");
            println!("  mentat index  <path>   # build ReDB index (files, chunks, embeds)");
            println!("  mentat search <query>  # brute-force search");
            println!("  mentat build-hnsw      # build HNSW index from embeddings");
            println!("  mentat search-hnsw <query> # query via HNSW");
        }
    }
    Ok(())
}

fn run_index(path: &str) -> Result<()> {
    // 1) ingest
    let files = mentat_ingest::ingest(path)?;
    // 2) open store
    let store = mentat_store::Store::open_default()?;
    // 3) for each file, chunk + embed
    let root = Path::new(path);
    for f in files {
        // write file meta
        let fhash = hex_to32(&f.hash)?;
        store.put_file(fhash, &mentat_store::FileMeta { path: relativize(&f.path, root), size: f.size })?;
        // chunk
        let spans = mentat_chunker::chunk_file(&f.path)?;
        if spans.is_empty() { continue; }
        let data = fs::read(&f.path)?;
        for s in spans {
            // chunk id = blake3(file_hash || start || end)
            let mut id_src = Vec::with_capacity(32 + 16);
            id_src.extend_from_slice(&fhash);
            id_src.extend_from_slice(&s.start.to_le_bytes());
            id_src.extend_from_slice(&s.end.to_le_bytes());
            let chunk_id = mentat_store::blake32(&id_src);

            // embed from raw slice
            let slice = &data[s.start..s.end];
            let text = String::from_utf8_lossy(slice);
            let emb = mentat_embedder::embed_text(&text)?;
            store.put_chunk(chunk_id, &mentat_store::ChunkMeta {
                file_hash: fhash,
                start: s.start,
                end: s.end,
                span_hash: hex_to32(&s.hash)?,
            })?;
            store.put_embed(chunk_id, &emb)?;
        }
    }
    println!("Index built at ./index/kv.redb");
    Ok(())
}

fn hex_to32(h: &str) -> Result<[u8;32]> {
    let bytes = hex::decode(h)?;
    let arr: [u8;32] = bytes.as_slice().try_into().map_err(|_| anyhow::anyhow!("bad len"))?;
    Ok(arr)
}

fn relativize(p: &str, root: &Path) -> String {
    let pp = Path::new(p);
    match pp.strip_prefix(root) {
        Ok(r) => r.display().to_string(),
        Err(_) => p.to_string(),
    }
}

// temp explicit uses
use anyhow;
use hex;
use mentat_ingest;
use mentat_chunker;
use mentat_store;
use mentat_embedder;
use mentat_retriever;
