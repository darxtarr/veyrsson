use anyhow::Result;
use serde::{Deserialize, Serialize};
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

#[derive(Deserialize)]
struct Request {
    cmd: String,
    #[serde(default)]
    query: String,
    #[serde(default = "default_topk")]
    topk: usize,
    #[serde(default)]
    text: String,
}

fn default_topk() -> usize {
    5
}

#[derive(Serialize)]
struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    ok: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    results: Option<Vec<(usize, f32)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    embedding: Option<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    eprintln!("[mentatd] Initializing...");

    // Load retriever with HNSW
    eprintln!("[mentatd] Loading retriever...");
    let mut retriever = mentat_retriever::Retriever::open_default()?;

    // Try to load HNSW index if it exists (check for .data file)
    if std::path::Path::new("index/embeds.hnsw.data").exists() {
        eprintln!("[mentatd] Loading HNSW index...");
        retriever.load_hnsw("index/embeds.hnsw")?;
        eprintln!("[mentatd] HNSW index loaded");
    } else {
        eprintln!("[mentatd] No HNSW index found, will build on first search");
        // Build HNSW in memory from embeddings
        eprintln!("[mentatd] Building HNSW index in memory...");
        let _ = retriever.build_hnsw("index/embeds")?;  // This builds but also saves
        eprintln!("[mentatd] HNSW index ready");
    }

    let retriever = Arc::new(Mutex::new(retriever));

    // Start TCP listener
    let listener = TcpListener::bind("127.0.0.1:6667").await?;
    eprintln!("[mentatd] Listening on 127.0.0.1:6667");
    eprintln!("[mentatd] Ready to accept connections");
    eprintln!("[mentatd] Press Ctrl+C to shutdown gracefully");

    // Setup signal handler with channel
    let mut signals = Signals::new(&[SIGINT, SIGTERM])?;
    let handle = signals.handle();

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

    // Spawn signal handler task
    tokio::spawn(async move {
        while let Some(signal) = signals.next().await {
            match signal {
                SIGINT | SIGTERM => {
                    eprintln!("\n[mentatd] Received shutdown signal, exiting...");
                    let _ = shutdown_tx.send(()).await;
                    return;
                }
                _ => {}
            }
        }
    });

    // Main accept loop
    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((mut socket, addr)) => {
                        let retriever = retriever.clone();
                        tokio::spawn(async move {
            eprintln!("[mentatd] Connection from {}", addr);

            let mut buf = vec![0u8; 65536]; // 64KB buffer
            let n = match socket.read(&mut buf).await {
                Ok(n) if n > 0 => n,
                Ok(_) => {
                    eprintln!("[mentatd] Empty request from {}", addr);
                    return;
                }
                Err(e) => {
                    eprintln!("[mentatd] Read error from {}: {}", addr, e);
                    return;
                }
            };

            let request: Request = match serde_json::from_slice(&buf[..n]) {
                Ok(req) => req,
                Err(e) => {
                    eprintln!("[mentatd] Parse error from {}: {}", addr, e);
                    let error_resp = Response {
                        ok: None,
                        results: None,
                        embedding: None,
                        error: Some(format!("Parse error: {}", e)),
                    };
                    let _ = socket.write_all(serde_json::to_string(&error_resp).unwrap().as_bytes()).await;
                    return;
                }
            };

            eprintln!("[mentatd] Command: {} from {}", request.cmd, addr);

            let response = match request.cmd.as_str() {
                "ping" => Response {
                    ok: Some(true),
                    results: None,
                    embedding: None,
                    error: None,
                },
                "search" => {
                    let retriever = retriever.lock().await;
                    match retriever.search(&request.query, request.topk) {
                        Ok(results) => Response {
                            ok: Some(true),
                            results: Some(results),
                            embedding: None,
                            error: None,
                        },
                        Err(e) => Response {
                            ok: None,
                            results: None,
                            embedding: None,
                            error: Some(format!("Search error: {}", e)),
                        },
                    }
                }
                "embed" => {
                    match mentat_embedder::embed_text(&request.text) {
                        Ok(emb) => Response {
                            ok: Some(true),
                            results: None,
                            embedding: Some(emb.to_vec()),
                            error: None,
                        },
                        Err(e) => Response {
                            ok: None,
                            results: None,
                            embedding: None,
                            error: Some(format!("Embed error: {}", e)),
                        },
                    }
                }
                _ => Response {
                    ok: None,
                    results: None,
                    embedding: None,
                    error: Some(format!("Unknown command: {}", request.cmd)),
                },
            };

            let response_json = serde_json::to_string(&response).unwrap();
            if let Err(e) = socket.write_all(response_json.as_bytes()).await {
                eprintln!("[mentatd] Write error to {}: {}", addr, e);
            }

                            eprintln!("[mentatd] Request completed for {}", addr);
                        });
                    }
                    Err(e) => {
                        eprintln!("[mentatd] Accept error: {}", e);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                eprintln!("[mentatd] Shutting down listener...");
                break;
            }
        }
    }

    // Cleanup
    handle.close();
    eprintln!("[mentatd] Shutdown complete");
    Ok(())
}
