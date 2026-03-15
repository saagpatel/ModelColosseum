use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

const OLLAMA_BASE: &str = "http://localhost:11434";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaModel {
    pub name: String,
    pub size: Option<u64>,
    pub details: Option<OllamaModelDetails>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaModelDetails {
    pub parameter_size: Option<String>,
    pub quantization_level: Option<String>,
    pub family: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TagsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShowResponse {
    pub details: Option<ShowDetails>,
    pub modelfile: Option<String>,
    pub parameters: Option<String>,
    pub template: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShowDetails {
    pub parameter_size: Option<String>,
    pub quantization_level: Option<String>,
    pub family: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StreamChunk {
    pub model: Option<String>,
    pub response: Option<String>,
    pub done: bool,
    pub total_duration: Option<u64>,
    pub eval_count: Option<u64>,
    pub eval_duration: Option<u64>,
}

pub async fn health_check() -> Result<bool, String> {
    let client = Client::new();
    match client
        .get(OLLAMA_BASE)
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
    {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(_) => Ok(false),
    }
}

pub async fn list_models() -> Result<Vec<OllamaModel>, String> {
    let client = Client::new();
    let resp = client
        .get(format!("{OLLAMA_BASE}/api/tags"))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("failed to reach Ollama: {e}"))?;

    let tags: TagsResponse = resp
        .json()
        .await
        .map_err(|e| format!("failed to parse Ollama response: {e}"))?;

    Ok(tags.models)
}

pub async fn show_model(name: &str) -> Result<ShowResponse, String> {
    let client = Client::new();
    let resp = client
        .post(format!("{OLLAMA_BASE}/api/show"))
        .json(&serde_json::json!({ "name": name }))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("failed to reach Ollama: {e}"))?;

    resp.json::<ShowResponse>()
        .await
        .map_err(|e| format!("failed to parse show response: {e}"))
}

pub struct GenerateRequest {
    pub model: String,
    pub prompt: String,
    pub system: Option<String>,
    pub num_predict: Option<u32>,
    pub temperature: Option<f64>,
}

/// Streams tokens from Ollama's /api/generate endpoint.
/// Returns a receiver that yields individual tokens, and a final StreamChunk with stats.
pub async fn generate_stream(
    req: GenerateRequest,
) -> Result<mpsc::Receiver<Result<StreamChunk, String>>, String> {
    let (tx, rx) = mpsc::channel::<Result<StreamChunk, String>>(256);

    let mut body = serde_json::json!({
        "model": req.model,
        "prompt": req.prompt,
        "stream": true,
    });

    if let Some(system) = &req.system {
        body["system"] = serde_json::Value::String(system.clone());
    }

    let mut options = serde_json::Map::new();
    if let Some(num_predict) = req.num_predict {
        options.insert(
            "num_predict".into(),
            serde_json::Value::Number(num_predict.into()),
        );
    }
    if let Some(temp) = req.temperature {
        if let Some(n) = serde_json::Number::from_f64(temp) {
            options.insert("temperature".into(), serde_json::Value::Number(n));
        }
    }
    if !options.is_empty() {
        body["options"] = serde_json::Value::Object(options);
    }

    let client = Client::new();
    let resp = client
        .post(format!("{OLLAMA_BASE}/api/generate"))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("failed to start generate stream: {e}"))?;

    tokio::spawn(async move {
        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(bytes) => {
                    buffer.push_str(&String::from_utf8_lossy(&bytes));

                    // Process complete NDJSON lines
                    while let Some(newline_pos) = buffer.find('\n') {
                        let line = buffer[..newline_pos].trim().to_string();
                        buffer = buffer[newline_pos + 1..].to_string();

                        if line.is_empty() {
                            continue;
                        }

                        match serde_json::from_str::<StreamChunk>(&line) {
                            Ok(chunk) => {
                                if tx.send(Ok(chunk)).await.is_err() {
                                    return; // receiver dropped
                                }
                            }
                            Err(e) => {
                                let _ = tx
                                    .send(Err(format!("failed to parse NDJSON chunk: {e}")))
                                    .await;
                                return;
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(format!("stream error: {e}"))).await;
                    return;
                }
            }
        }
    });

    Ok(rx)
}
