//! ReDB-backed index at ./index/kv.redb
//! Tables:
//!   files: key=blake3(file bytes), val=bincode(FileMeta)
//!   chunks: key=blake3(file bytes) + start..end, val=bincode(ChunkMeta)
//!   embeds: key=chunk_id, val=[f32; D] as bytes

use anyhow::Result;
use redb::{Database, ReadableTable, TableDefinition};
use serde::{Serialize, Deserialize};
use std::fs;
use std::collections::{HashMap, HashSet};
use bytemuck::cast_slice;

const FILES: TableDefinition<&[u8], &[u8]>  = TableDefinition::new("files");
const CHUNKS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("chunks");
const EMBEDS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("embeds");

#[derive(Serialize, Deserialize, Clone)]
pub struct FileMeta {
    pub path: String,
    pub size: usize,
    pub mtime: i64,       // seconds since epoch
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChunkMeta {
    pub file_hash: [u8; 32],
    pub start: usize,
    pub end: usize,
    pub span_hash: [u8; 32],
}

pub struct Store {
    db: Database,
}

impl Store {
    pub fn open_default() -> Result<Self> {
        fs::create_dir_all("index")?;
        let db = Database::builder().create("index/kv.redb")?;
        // create tables if not exist
        let tx = db.begin_write()?;
        { tx.open_table(FILES)?; tx.open_table(CHUNKS)?; tx.open_table(EMBEDS)?; }
        tx.commit()?;
        Ok(Self { db })
    }

    pub fn put_file(&self, file_hash: [u8;32], meta: &FileMeta) -> Result<()> {
        let tx = self.db.begin_write()?;
        {
            let mut t = tx.open_table(FILES)?;
            let val = bincode::serialize(meta)?;
            t.insert(file_hash.as_slice(), val.as_slice())?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn put_chunk(&self, chunk_id: [u8;32], meta: &ChunkMeta) -> Result<()> {
        let tx = self.db.begin_write()?;
        {
            let mut t = tx.open_table(CHUNKS)?;
            let val = bincode::serialize(meta)?;
            t.insert(chunk_id.as_slice(), val.as_slice())?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn put_embed(&self, chunk_id: [u8;32], emb: &[f32;384]) -> Result<()> {
        let tx = self.db.begin_write()?;
        {
            let mut t = tx.open_table(EMBEDS)?;
            let bytes = cast_slice::<f32, u8>(emb);
            t.insert(chunk_id.as_slice(), bytes)?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_known_hashes(&self) -> Result<HashSet<[u8; 32]>> {
        let tx = self.db.begin_read()?;
        let table = tx.open_table(FILES)?;
        let mut set = HashSet::new();
        for item in table.iter()? {
            let (key, _) = item?;
            let key_bytes: &[u8] = key.value();
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&key_bytes[..32]);
            set.insert(arr);
        }
        Ok(set)
    }

    pub fn get_file_meta_map(&self) -> Result<HashMap<[u8;32], FileMeta>> {
        let tx = self.db.begin_read()?;
        let table = tx.open_table(FILES)?;
        let mut map = HashMap::new();
        for item in table.iter()? {
            let (k, v) = item?;
            let mut key = [0u8; 32];
            key.copy_from_slice(k.value());
            let meta: FileMeta = bincode::deserialize(v.value())?;
            map.insert(key, meta);
        }
        Ok(map)
    }
}

// helpers
pub fn blake32(bytes: &[u8]) -> [u8;32] {
    blake3::hash(bytes).as_bytes().to_owned()
}
