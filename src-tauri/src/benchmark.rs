use crate::db;
use crate::ollama::{self, GenerateRequest, StreamChunk};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::Emitter;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Managed state
// ---------------------------------------------------------------------------

pub struct ActiveBenchmarks(pub Arc<Mutex<HashMap<i64, CancellationToken>>>);

// ---------------------------------------------------------------------------
// Event payloads
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BenchmarkProgressPayload {
    pub run_id: i64,
    pub completed: i32,
    pub total: i32,
    pub current_model: String,
    pub current_prompt: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BenchmarkStreamPayload {
    pub run_id: i64,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BenchmarkCompletePayload {
    pub run_id: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BenchmarkErrorPayload {
    pub run_id: i64,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Data structs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Clone)]
pub struct TestSuite {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub is_default: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Prompt {
    pub id: i64,
    pub suite_id: i64,
    pub category: String,
    pub title: String,
    pub text: String,
    pub system_prompt: Option<String>,
    pub ideal_answer: Option<String>,
    pub eval_criteria: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ReorderItem {
    pub id: i64,
    pub sort_order: i64,
}

// ---------------------------------------------------------------------------
// Benchmark metrics
// ---------------------------------------------------------------------------

struct BenchmarkMetrics {
    tokens_generated: i64,
    time_to_first_token_ms: Option<i64>,
    total_time_ms: i64,
    tokens_per_second: Option<f64>,
}

// ---------------------------------------------------------------------------
// Stream helper
// ---------------------------------------------------------------------------

async fn benchmark_stream_and_collect(
    app: &tauri::AppHandle,
    run_id: i64,
    mut rx: tokio::sync::mpsc::Receiver<Result<StreamChunk, String>>,
    cancel_token: &CancellationToken,
    start: Instant,
) -> Result<(String, BenchmarkMetrics), String> {
    let mut buffer = String::new();
    let mut first_token_time: Option<i64> = None;
    let mut eval_count: Option<u64> = None;
    let mut eval_duration: Option<u64> = None;

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                return Err("cancelled".into());
            }
            chunk = rx.recv() => {
                match chunk {
                    Some(Ok(c)) => {
                        if let Some(ref text) = c.response {
                            if !text.is_empty() {
                                if first_token_time.is_none() {
                                    first_token_time = Some(start.elapsed().as_millis() as i64);
                                }
                                buffer.push_str(text);
                                let _ = app.emit("benchmark:stream", BenchmarkStreamPayload {
                                    run_id,
                                    token: text.clone(),
                                });
                            }
                        }
                        if c.done {
                            eval_count = c.eval_count;
                            eval_duration = c.eval_duration;
                            break;
                        }
                    }
                    Some(Err(e)) => return Err(e),
                    None => break,
                }
            }
        }
    }

    let total_time_ms = start.elapsed().as_millis() as i64;
    let tps = match (eval_count, eval_duration) {
        (Some(count), Some(dur)) if dur > 0 => {
            Some(count as f64 / (dur as f64 / 1_000_000_000.0))
        }
        _ => None,
    };

    Ok((
        buffer,
        BenchmarkMetrics {
            tokens_generated: eval_count.unwrap_or(0) as i64,
            time_to_first_token_ms: first_token_time,
            total_time_ms,
            tokens_per_second: tps,
        },
    ))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn update_run_status(run_id: i64, status: &str) -> Result<(), String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    conn.execute(
        "UPDATE benchmark_runs SET status = ?1, completed_at = datetime('now') WHERE id = ?2",
        rusqlite::params![status, run_id],
    )
    .map_err(|e| format!("update run status error: {e}"))?;
    Ok(())
}

fn cleanup_token(map: &Arc<Mutex<HashMap<i64, CancellationToken>>>, run_id: i64) {
    if let Ok(mut m) = map.lock() {
        m.remove(&run_id);
    }
}

// ---------------------------------------------------------------------------
// CRUD commands — test suites
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_test_suites() -> Result<Vec<TestSuite>, String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, description, is_default, created_at, updated_at
             FROM test_suites ORDER BY created_at ASC",
        )
        .map_err(|e| format!("query error: {e}"))?;

    let suites = stmt
        .query_map([], |row| {
            Ok(TestSuite {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                is_default: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| format!("query error: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("row error: {e}"))?;

    Ok(suites)
}

#[tauri::command]
pub async fn create_test_suite(
    name: String,
    description: Option<String>,
) -> Result<TestSuite, String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    conn.execute(
        "INSERT INTO test_suites (name, description) VALUES (?1, ?2)",
        rusqlite::params![name, description],
    )
    .map_err(|e| format!("insert error: {e}"))?;

    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, name, description, is_default, created_at, updated_at
         FROM test_suites WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            Ok(TestSuite {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                is_default: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        },
    )
    .map_err(|e| format!("fetch after insert error: {e}"))
}

#[tauri::command]
pub async fn update_test_suite(
    id: i64,
    name: String,
    description: Option<String>,
) -> Result<(), String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    conn.execute(
        "UPDATE test_suites SET name = ?1, description = ?2, updated_at = datetime('now') WHERE id = ?3",
        rusqlite::params![name, description, id],
    )
    .map_err(|e| format!("update error: {e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn delete_test_suite(id: i64) -> Result<(), String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    conn.execute("DELETE FROM test_suites WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| format!("delete error: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// CRUD commands — prompts
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_prompts(suite_id: i64) -> Result<Vec<Prompt>, String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, suite_id, category, title, text, system_prompt, ideal_answer,
                    eval_criteria, sort_order, created_at
             FROM prompts WHERE suite_id = ?1 ORDER BY sort_order ASC",
        )
        .map_err(|e| format!("query error: {e}"))?;

    let prompts = stmt
        .query_map(rusqlite::params![suite_id], |row| {
            Ok(Prompt {
                id: row.get(0)?,
                suite_id: row.get(1)?,
                category: row.get(2)?,
                title: row.get(3)?,
                text: row.get(4)?,
                system_prompt: row.get(5)?,
                ideal_answer: row.get(6)?,
                eval_criteria: row.get(7)?,
                sort_order: row.get(8)?,
                created_at: row.get(9)?,
            })
        })
        .map_err(|e| format!("query error: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("row error: {e}"))?;

    Ok(prompts)
}

#[tauri::command]
pub async fn create_prompt(
    suite_id: i64,
    title: String,
    category: String,
    text: String,
    system_prompt: Option<String>,
    ideal_answer: Option<String>,
    eval_criteria: Option<String>,
) -> Result<Prompt, String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;

    let max_order: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) FROM prompts WHERE suite_id = ?1",
            rusqlite::params![suite_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("sort_order query error: {e}"))?;

    let sort_order = max_order + 1;

    conn.execute(
        "INSERT INTO prompts (suite_id, category, title, text, system_prompt, ideal_answer, eval_criteria, sort_order)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![suite_id, category, title, text, system_prompt, ideal_answer, eval_criteria, sort_order],
    )
    .map_err(|e| format!("insert error: {e}"))?;

    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, suite_id, category, title, text, system_prompt, ideal_answer,
                eval_criteria, sort_order, created_at
         FROM prompts WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            Ok(Prompt {
                id: row.get(0)?,
                suite_id: row.get(1)?,
                category: row.get(2)?,
                title: row.get(3)?,
                text: row.get(4)?,
                system_prompt: row.get(5)?,
                ideal_answer: row.get(6)?,
                eval_criteria: row.get(7)?,
                sort_order: row.get(8)?,
                created_at: row.get(9)?,
            })
        },
    )
    .map_err(|e| format!("fetch after insert error: {e}"))
}

#[tauri::command]
pub async fn update_prompt(
    id: i64,
    title: Option<String>,
    category: Option<String>,
    text: Option<String>,
    system_prompt: Option<String>,
    ideal_answer: Option<String>,
    eval_criteria: Option<String>,
) -> Result<(), String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;

    // Fetch current values
    let (cur_title, cur_category, cur_text, cur_system, cur_ideal, cur_eval): (
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT title, category, text, system_prompt, ideal_answer, eval_criteria
             FROM prompts WHERE id = ?1",
            rusqlite::params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        )
        .map_err(|e| format!("prompt not found (id={id}): {e}"))?;

    conn.execute(
        "UPDATE prompts SET title = ?1, category = ?2, text = ?3,
         system_prompt = ?4, ideal_answer = ?5, eval_criteria = ?6
         WHERE id = ?7",
        rusqlite::params![
            title.unwrap_or(cur_title),
            category.unwrap_or(cur_category),
            text.unwrap_or(cur_text),
            system_prompt.or(cur_system),
            ideal_answer.or(cur_ideal),
            eval_criteria.or(cur_eval),
            id,
        ],
    )
    .map_err(|e| format!("update error: {e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn delete_prompt(id: i64) -> Result<(), String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    conn.execute("DELETE FROM prompts WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| format!("delete error: {e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn reorder_prompts(items: Vec<ReorderItem>) -> Result<(), String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    conn.execute_batch("BEGIN")
        .map_err(|e| format!("begin transaction: {e}"))?;

    let result = (|| -> Result<(), String> {
        for item in &items {
            conn.execute(
                "UPDATE prompts SET sort_order = ?1 WHERE id = ?2",
                rusqlite::params![item.sort_order, item.id],
            )
            .map_err(|e| format!("reorder error (id={}): {e}", item.id))?;
        }
        Ok(())
    })();

    match result {
        Ok(()) => {
            conn.execute_batch("COMMIT")
                .map_err(|e| format!("commit transaction: {e}"))?;
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            return Err(e);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Runner commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn start_benchmark(
    app: tauri::AppHandle,
    state: tauri::State<'_, ActiveBenchmarks>,
    suite_id: i64,
    model_ids: Vec<i64>,
) -> Result<i64, String> {
    // Load prompts for suite
    let prompts = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, text, system_prompt FROM prompts
                 WHERE suite_id = ?1 ORDER BY sort_order ASC",
            )
            .map_err(|e| format!("prompts query error: {e}"))?;
        let rows = stmt
            .query_map(rusqlite::params![suite_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(|e| format!("prompts query error: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("prompts row error: {e}"))?;
        rows
    };

    if prompts.is_empty() {
        return Err("Suite has no prompts".into());
    }

    // Load model names
    let models: Vec<(i64, String)> = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        model_ids
            .iter()
            .map(|&mid| {
                conn.query_row(
                    "SELECT id, name FROM models WHERE id = ?1",
                    rusqlite::params![mid],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|e| format!("model not found (id={mid}): {e}"))
            })
            .collect::<Result<Vec<_>, _>>()?
    };

    if models.is_empty() {
        return Err("No models specified".into());
    }

    // Insert benchmark_run
    let run_id = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        conn.execute(
            "INSERT INTO benchmark_runs (suite_id, status) VALUES (?1, 'running')",
            rusqlite::params![suite_id],
        )
        .map_err(|e| format!("insert run error: {e}"))?;
        conn.last_insert_rowid()
    };

    // Create cancellation token
    let cancel_token = CancellationToken::new();
    {
        let mut map = state.0.lock().map_err(|e| format!("state lock: {e}"))?;
        map.insert(run_id, cancel_token.clone());
    }

    let total = (prompts.len() * models.len()) as i32;
    let active_map = Arc::clone(&state.0);

    tokio::spawn(async move {
        let mut completed = 0i32;

        'outer: for (model_id, model_name) in &models {
            for (prompt_id, prompt_title, prompt_text, prompt_system) in &prompts {
                if cancel_token.is_cancelled() {
                    break 'outer;
                }

                let req = GenerateRequest {
                    model: model_name.clone(),
                    prompt: prompt_text.clone(),
                    system: prompt_system.clone(),
                    num_predict: None,
                    temperature: Some(0.7),
                };

                let rx = match ollama::generate_stream(req).await {
                    Ok(rx) => rx,
                    Err(e) => {
                        let _ = update_run_status(run_id, "cancelled");
                        cleanup_token(&active_map, run_id);
                        let _ = app.emit(
                            "benchmark:error",
                            BenchmarkErrorPayload {
                                run_id,
                                message: format!(
                                    "Stream error for model '{}', prompt '{}': {e}",
                                    model_name, prompt_title
                                ),
                            },
                        );
                        return;
                    }
                };

                let start = Instant::now();
                let result = benchmark_stream_and_collect(&app, run_id, rx, &cancel_token, start).await;

                match result {
                    Ok((output, metrics)) => {
                        // Save result
                        if let Err(e) = (|| -> Result<(), String> {
                            let conn =
                                db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
                            conn.execute(
                                "INSERT INTO benchmark_results
                                    (run_id, prompt_id, model_id, output,
                                     tokens_generated, time_to_first_token_ms, total_time_ms, tokens_per_second)
                                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                                rusqlite::params![
                                    run_id,
                                    prompt_id,
                                    model_id,
                                    output,
                                    metrics.tokens_generated,
                                    metrics.time_to_first_token_ms,
                                    metrics.total_time_ms,
                                    metrics.tokens_per_second,
                                ],
                            )
                            .map_err(|e| format!("insert result error: {e}"))?;
                            Ok(())
                        })() {
                            let _ = update_run_status(run_id, "cancelled");
                            cleanup_token(&active_map, run_id);
                            let _ = app.emit(
                                "benchmark:error",
                                BenchmarkErrorPayload {
                                    run_id,
                                    message: e,
                                },
                            );
                            return;
                        }

                        completed += 1;
                        let _ = app.emit(
                            "benchmark:progress",
                            BenchmarkProgressPayload {
                                run_id,
                                completed,
                                total,
                                current_model: model_name.clone(),
                                current_prompt: prompt_title.clone(),
                            },
                        );
                    }
                    Err(e) if e == "cancelled" => {
                        break 'outer;
                    }
                    Err(e) => {
                        let _ = update_run_status(run_id, "cancelled");
                        cleanup_token(&active_map, run_id);
                        let _ = app.emit(
                            "benchmark:error",
                            BenchmarkErrorPayload {
                                run_id,
                                message: format!(
                                    "Generation error for model '{}', prompt '{}': {e}",
                                    model_name, prompt_title
                                ),
                            },
                        );
                        return;
                    }
                }
            }
        }

        if cancel_token.is_cancelled() {
            let _ = update_run_status(run_id, "cancelled");
            let _ = app.emit("benchmark:error", BenchmarkErrorPayload {
                run_id,
                message: "Benchmark cancelled".into(),
            });
        } else {
            let _ = update_run_status(run_id, "completed");
            let _ = app.emit("benchmark:complete", BenchmarkCompletePayload { run_id });
        }

        cleanup_token(&active_map, run_id);
    });

    Ok(run_id)
}

#[tauri::command]
pub async fn cancel_benchmark(
    state: tauri::State<'_, ActiveBenchmarks>,
    run_id: i64,
) -> Result<(), String> {
    let found = {
        let map = state.0.lock().map_err(|e| format!("state lock: {e}"))?;
        if let Some(token) = map.get(&run_id) {
            token.cancel();
            true
        } else {
            false
        }
    };

    if !found {
        update_run_status(run_id, "cancelled")?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reorder_item_deserializes() {
        let json = r#"{"id":1,"sort_order":3}"#;
        let item: ReorderItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.id, 1);
        assert_eq!(item.sort_order, 3);
    }

    #[test]
    fn benchmark_progress_payload_serializes() {
        let p = BenchmarkProgressPayload {
            run_id: 42,
            completed: 5,
            total: 15,
            current_model: "llama3".into(),
            current_prompt: "Coding: Rust LRU Cache".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"run_id\":42"));
        assert!(json.contains("\"completed\":5"));
        assert!(json.contains("\"total\":15"));
    }

    #[test]
    fn benchmark_complete_payload_serializes() {
        let p = BenchmarkCompletePayload { run_id: 7 };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"run_id\":7"));
    }

    #[test]
    fn benchmark_error_payload_round_trips() {
        let p = BenchmarkErrorPayload {
            run_id: 99,
            message: "stream error".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: BenchmarkErrorPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.run_id, 99);
        assert_eq!(back.message, "stream error");
    }
}
