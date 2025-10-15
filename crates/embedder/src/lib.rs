//! Real embedding via Candle + BGE-small-en-v1.5.
//! Keeps the same API signature: text -> [f32; 384]

use anyhow::{Context, Result};
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use tokenizers::Tokenizer;

pub const D: usize = 384;

static INIT: Lazy<Mutex<Option<(Tokenizer, BertModel, Device)>>> = Lazy::new(|| Mutex::new(None));

fn get_model_and_tokenizer() -> Result<&'static Mutex<Option<(Tokenizer, BertModel, Device)>>> {
    let mut guard = INIT.lock().unwrap();
    if guard.is_none() {
        eprintln!("[embedder] Initializing model (first time only)...");

        // Initialize device
        eprintln!("[embedder] Setting up device...");
        let device = Device::cuda_if_available(0)
            .context("initializing device")?;
        eprintln!("[embedder] Using device: {:?}", device);

        // Load tokenizer
        eprintln!("[embedder] Loading tokenizer...");
        let tokenizer = Tokenizer::from_file("crates/embedder/models/tokenizer.json")
            .map_err(|e| anyhow::anyhow!("loading tokenizer: {}", e))?;

        // Load config
        eprintln!("[embedder] Loading config...");
        let config_path = "crates/embedder/models/config.json";
        let config_json = std::fs::read_to_string(config_path)
            .context("reading config.json")?;
        let config: Config = serde_json::from_str(&config_json)
            .context("parsing config")?;

        // Load model weights
        eprintln!("[embedder] Loading model weights (this may take a moment)...");
        let weights_path = "crates/embedder/models/model.safetensors";
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, &device)
                .context("loading safetensors")?
        };

        eprintln!("[embedder] Building BERT model...");
        let model = BertModel::load(vb, &config)
            .context("creating BERT model")?;

        eprintln!("[embedder] Model ready!");
        *guard = Some((tokenizer, model, device));
    }
    Ok(&INIT)
}

/// Compute the [CLS] embedding (normalized) for given text.
pub fn embed_text(text: &str) -> Result<[f32; D]> {
    let init_mutex = get_model_and_tokenizer()?;
    let guard = init_mutex.lock().unwrap();
    let (tokenizer, model, device) = guard.as_ref().unwrap();

    // Tokenize input
    let encoding = tokenizer
        .encode(text, true)
        .map_err(|e| anyhow::anyhow!("tokenization failed: {}", e))?;

    // Truncate to 512 tokens max (model's max_position_embeddings)
    let max_len = 512;
    let token_ids: Vec<u32> = encoding.get_ids().iter().take(max_len).copied().collect();
    let token_type_ids: Vec<u32> = encoding.get_type_ids().iter().take(max_len).copied().collect();
    let attention_mask: Vec<u32> = encoding.get_attention_mask().iter().take(max_len).copied().collect();

    // Create tensors
    let token_ids = Tensor::new(token_ids, device)?
        .unsqueeze(0)?; // [1, seq_len]
    let token_type_ids = Tensor::new(token_type_ids, device)?
        .unsqueeze(0)?; // [1, seq_len]
    let attention_mask = Tensor::new(attention_mask, device)?
        .unsqueeze(0)?; // [1, seq_len]

    // Forward pass - only pass attention mask to the attention mechanism
    let embeddings = model.forward(&token_ids, &token_type_ids, Some(&attention_mask))?;

    // Extract [CLS] token (first token)
    // embeddings is [batch_size, seq_len, hidden_size]
    // We want [0, 0, :] which is the CLS token of the first batch item
    let cls_embedding = embeddings.narrow(0, 0, 1)?.narrow(1, 0, 1)?.squeeze(0)?.squeeze(0)?;

    // Convert to f32 vec
    let emb_vec = cls_embedding.to_vec1::<f32>()?;

    // Normalize
    let norm = (emb_vec.iter().map(|x| x * x).sum::<f32>())
        .sqrt()
        .max(1e-6);

    let mut out = [0f32; D];
    for (i, &v) in emb_vec.iter().enumerate().take(D) {
        out[i] = v / norm;
    }

    Ok(out)
}
