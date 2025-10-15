use anyhow::Result;
use blake3::Hasher;
use serde::Serialize;
use std::{fs, path::Path};
use walkdir::WalkDir;
use globset::{Glob, GlobSet, GlobSetBuilder};

#[derive(Serialize)]
pub struct Chunk {
    pub path: String,
    pub hash: String,
    pub size: usize,
}

pub fn ingest<P: AsRef<Path>>(root: P) -> Result<Vec<Chunk>> {
    let mut out = Vec::new();
    let ignore = load_ignore(root.as_ref());

    for entry in WalkDir::new(root) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let path = entry.path();
            if should_ignore(path, &ignore) {
                continue;
            }
            let data = fs::read(path)?;
            let mut hasher = Hasher::new();
            hasher.update(&data);
            let hash = hasher.finalize().to_hex().to_string();
            out.push(Chunk {
                path: path.display().to_string(),
                hash,
                size: data.len(),
            });
        }
    }
    Ok(out)
}

pub fn dump_json(chunks: &[Chunk]) -> Result<()> {
    let json = serde_json::to_string_pretty(chunks)?;
    fs::write("ingest_manifest.json", json)?;
    Ok(())
}

fn load_ignore(root: &Path) -> GlobSet {
    let mut builder = GlobSetBuilder::new();

    // built-in defaults
    let builtins = [
        ".git/",
        "target/",
        "node_modules/",
        ".DS_Store",
        "Thumbs.db",
        "*.lock",
        "*.tmp",
        "*.log",
        "*.swp",
        "*.swo",
        "index/",
        ".claude/",
        ".vscode/",
        ".idea/",
        ".env",
        ".env.local",
    ];
    for p in &builtins {
        // Convert directory patterns to match contents
        let pattern = if p.ends_with('/') {
            format!("{}**", p)
        } else {
            p.to_string()
        };
        if let Ok(g) = Glob::new(&pattern) {
            builder.add(g);
        }
    }

    // optional .ingestignore in repo root
    let f = root.join(".ingestignore");
    if let Ok(txt) = fs::read_to_string(f) {
        for line in txt.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            // Convert directory patterns to match contents
            let pattern = if line.ends_with('/') {
                format!("{}**", line)
            } else {
                line.to_string()
            };
            if let Ok(g) = Glob::new(&pattern) {
                builder.add(g);
            }
        }
    }

    builder
        .build()
        .unwrap_or_else(|_| GlobSetBuilder::new().build().unwrap())
}

fn should_ignore(path: &Path, ignore: &GlobSet) -> bool {
    let path_str = path.to_string_lossy();
    let cleaned = path_str.strip_prefix("./").unwrap_or(&path_str);

    // Check if the path or any of its components match
    ignore.is_match(cleaned) ||
        path.components().any(|c| {
            let comp = c.as_os_str().to_string_lossy();
            ignore.is_match(comp.as_ref())
        })
}
