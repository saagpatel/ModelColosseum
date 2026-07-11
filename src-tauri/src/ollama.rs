use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tokio::sync::mpsc;

const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

fn client() -> &'static Client {
    static HTTP: OnceLock<Client> = OnceLock::new();
    HTTP.get_or_init(Client::new)
}

/// Read the configured Ollama URL from settings, falling back to the default.
pub fn get_base_url() -> String {
    if let Ok(conn) = crate::db::get_db().lock() {
        if let Ok(url) = conn.query_row(
            "SELECT value FROM settings WHERE key = 'ollama_url'",
            [],
            |row| row.get::<_, String>(0),
        ) {
            if !url.is_empty() {
                return url.trim_end_matches('/').to_string();
            }
        }
    }
    DEFAULT_OLLAMA_URL.to_string()
}

fn require_local_base_url(base: &str) -> Result<(), String> {
    let url = reqwest::Url::parse(base).map_err(|e| format!("invalid Ollama URL: {e}"))?;
    let is_local = url
        .host_str()
        .map(|host| {
            let normalized = host.trim_matches(['[', ']']);
            normalized.eq_ignore_ascii_case("localhost")
                || normalized
                    .parse::<std::net::IpAddr>()
                    .map(|address| address.is_loopback())
                    .unwrap_or(false)
        })
        .unwrap_or(false);
    if !is_local {
        return Err(
            "ModelColosseum evaluation calls are restricted to local Ollama endpoints".into(),
        );
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaModel {
    pub name: String,
    pub size: Option<u64>,
    #[serde(default)]
    pub digest: Option<String>,
    #[serde(default)]
    pub modified_at: Option<String>,
    pub details: Option<OllamaModelDetails>,
    #[serde(default)]
    pub capabilities: Vec<String>,
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

#[derive(Debug, Deserialize)]
struct VersionResponse {
    version: String,
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
#[allow(dead_code)]
pub struct StreamChunk {
    pub model: Option<String>,
    pub response: Option<String>,
    pub done: bool,
    pub total_duration: Option<u64>,
    pub eval_count: Option<u64>,
    pub eval_duration: Option<u64>,
}

pub async fn health_check() -> Result<bool, String> {
    let base = get_base_url();
    require_local_base_url(&base)?;
    match client()
        .get(&base)
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
    {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(_) => Ok(false),
    }
}

pub async fn list_models() -> Result<Vec<OllamaModel>, String> {
    let base = get_base_url();
    require_local_base_url(&base)?;
    let resp = client()
        .get(format!("{base}/api/tags"))
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
    let base = get_base_url();
    require_local_base_url(&base)?;
    let resp = client()
        .post(format!("{base}/api/show"))
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
    pub think: Option<bool>,
    pub seed: Option<u64>,
}

pub async fn get_version() -> Result<String, String> {
    let base = get_base_url();
    require_local_base_url(&base)?;
    let response = client()
        .get(format!("{base}/api/version"))
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .map_err(|e| format!("failed to reach Ollama version endpoint: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Ollama version endpoint returned {}",
            response.status()
        ));
    }
    response
        .json::<VersionResponse>()
        .await
        .map(|payload| payload.version)
        .map_err(|e| format!("failed to parse Ollama version: {e}"))
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
    if let Some(think) = req.think {
        body["think"] = serde_json::Value::Bool(think);
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
    if let Some(seed) = req.seed {
        options.insert("seed".into(), serde_json::Value::Number(seed.into()));
    }
    if !options.is_empty() {
        body["options"] = serde_json::Value::Object(options);
    }

    let base = get_base_url();
    require_local_base_url(&base)?;
    let resp = client()
        .post(format!("{base}/api/generate"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluation_endpoints_must_be_local() {
        assert!(require_local_base_url("http://localhost:11434").is_ok());
        assert!(require_local_base_url("http://127.0.0.1:11434").is_ok());
        assert!(require_local_base_url("http://[::1]:11434").is_ok());
        assert!(require_local_base_url("https://example.com").is_err());
        assert!(require_local_base_url("http://192.168.1.10:11434").is_err());
    }
}
