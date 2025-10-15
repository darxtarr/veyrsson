//! Phase 3b – Offline HNSW build and search.
//! Deterministic single-threaded index built from ReDB embeddings.

use anyhow::Result;
use redb::{Database, TableDefinition, ReadableTable};
use mentat_embedder::{embed_text, D};
use hnsw_rs::prelude::*;
use serde::{Serialize, Deserialize};
use std::{fs, path::Path};

const EMBEDS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("embeds");

#[derive(Serialize, Deserialize)]
pub struct HnswHeader {
    pub n: usize,
    pub d: usize,
}

pub struct Retriever {
    db: Database,
    hnsw: Option<Hnsw<'static, f32, DistCosine>>,
}

impl Retriever {
    pub fn open_default() -> Result<Self> {
        let db = Database::builder().open("index/kv.redb")?;
        Ok(Self { db, hnsw: None })
    }

    pub fn build_hnsw(&mut self, out_path: &str) -> Result<()> {
        let tx = self.db.begin_read()?;
        let table = tx.open_table(EMBEDS)?;

        let mut data: Vec<Vec<f32>> = Vec::new();
        let mut ids: Vec<String> = Vec::new();

        for item in table.iter()? {
            let (key, val) = item?;
            let key_bytes: &[u8] = key.value();
            let id_hex = hex::encode(key_bytes);
            let val_bytes: &[u8] = val.value();

            // Convert bytes to Vec<f32>
            let float_slice = unsafe {
                std::slice::from_raw_parts(val_bytes.as_ptr() as *const f32, D)
            };
            let v = float_slice.to_vec();

            data.push(v);
            ids.push(id_hex);
        }

        println!("Building HNSW index for {} vectors...", data.len());

        let ef_c = 200;
        let m = 16;
        let dist = DistCosine {};
        let mut hnsw = Hnsw::<f32, DistCosine>::new(m, data.len(), 16, ef_c, dist);

        for (i, v) in data.iter().enumerate() {
            hnsw.insert((v, i));
        }
        hnsw.set_searching_mode(true);

        fs::create_dir_all(Path::new(out_path).parent().unwrap())?;
        let dir_path = Path::new(out_path).parent().unwrap();
        let file_name = Path::new(out_path).file_name().unwrap().to_str().unwrap();
        hnsw.file_dump(dir_path, file_name)?;
        let hdr = HnswHeader { n: data.len(), d: D };
        fs::write(format!("{}.hdr", out_path), bincode::serialize(&hdr)?)?;
        println!("Saved HNSW index to {}/{}.hnsw", dir_path.display(), file_name);
        self.hnsw = Some(hnsw);
        Ok(())
    }

    pub fn load_hnsw(&mut self, _path: &str) -> Result<()> {
        // For now, rebuild the index from ReDB data
        // TODO: implement proper serialization when hnsw_rs supports it better
        self.build_hnsw_internal()?;
        Ok(())
    }

    fn build_hnsw_internal(&mut self) -> Result<()> {
        let tx = self.db.begin_read()?;
        let table = tx.open_table(EMBEDS)?;

        let mut data: Vec<Vec<f32>> = Vec::new();

        for item in table.iter()? {
            let (_, val) = item?;
            let val_bytes: &[u8] = val.value();

            let float_slice = unsafe {
                std::slice::from_raw_parts(val_bytes.as_ptr() as *const f32, D)
            };
            let v = float_slice.to_vec();
            data.push(v);
        }

        let ef_c = 200;
        let m = 16;
        let dist = DistCosine {};
        let mut hnsw = Hnsw::<f32, DistCosine>::new(m, data.len(), 16, ef_c, dist);

        for (i, v) in data.iter().enumerate() {
            hnsw.insert((v, i));
        }
        hnsw.set_searching_mode(true);

        self.hnsw = Some(hnsw);
        Ok(())
    }

    pub fn search(&self, query: &str, topk: usize) -> Result<Vec<(usize, f32)>> {
        let q = embed_text(query)?;
        let q_vec: Vec<f32> = q.to_vec();
        let h = self.hnsw.as_ref().ok_or_else(|| anyhow::anyhow!("HNSW not loaded"))?;
        let res = h.search(&q_vec, topk, 16);
        let hits: Vec<(usize, f32)> = res.iter().map(|ne| (ne.d_id, ne.distance)).collect();
        Ok(hits)
    }
}
