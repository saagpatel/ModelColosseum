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
pub struct ActiveJudgeRuns(pub Arc<Mutex<HashMap<i64, CancellationToken>>>);

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

#[derive(Debug, Serialize, Clone)]
pub struct BenchmarkResult {
    pub id: i64,
    pub run_id: i64,
    pub prompt_id: i64,
    pub model_id: i64,
    pub model_name: String,
    pub prompt_title: String,
    pub prompt_category: String,
    pub output: String,
    pub tokens_generated: i64,
    pub time_to_first_token_ms: Option<i64>,
    pub total_time_ms: i64,
    pub tokens_per_second: Option<f64>,
    pub manual_score: Option<i64>,
    pub auto_judge_score: Option<i64>,
    pub auto_judge_notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct BenchmarkRunSummary {
    pub id: i64,
    pub suite_id: i64,
    pub suite_name: String,
    pub status: String,
    pub model_count: i64,
    pub prompt_count: i64,
    pub scored_count: i64,
    pub total_results: i64,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct BenchmarkLeaderboardEntry {
    pub model_id: i64,
    pub model_name: String,
    pub display_name: String,
    pub avg_score: Option<f64>,
    pub category_scores: HashMap<String, f64>,
    pub avg_tps: Option<f64>,
    pub avg_ttft_ms: Option<f64>,
    pub total_prompts_scored: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct RunComparisonEntry {
    pub prompt_id: i64,
    pub prompt_title: String,
    pub prompt_category: String,
    pub model_id: i64,
    pub model_name: String,
    pub run_a_score: Option<f64>,
    pub run_b_score: Option<f64>,
    pub score_delta: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AutoJudgeProgressPayload {
    pub run_id: i64,
    pub completed: i32,
    pub total: i32,
    pub current_model: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AutoJudgeCompletePayload {
    pub run_id: i64,
    pub scores_added: i32,
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
// Results + scoring commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_benchmark_results(run_id: i64) -> Result<Vec<BenchmarkResult>, String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    let mut stmt = conn
        .prepare(
            "SELECT br.id, br.run_id, br.prompt_id, br.model_id,
                    m.display_name, p.title, p.category,
                    br.output, br.tokens_generated, br.time_to_first_token_ms,
                    br.total_time_ms, br.tokens_per_second, br.created_at,
                    (SELECT bs.score FROM benchmark_scores bs
                     WHERE bs.result_id = br.id AND bs.scoring_method = 'manual'
                     ORDER BY bs.created_at DESC LIMIT 1),
                    (SELECT bs.score FROM benchmark_scores bs
                     WHERE bs.result_id = br.id AND bs.scoring_method = 'auto_judge'
                     ORDER BY bs.created_at DESC LIMIT 1),
                    (SELECT bs.notes FROM benchmark_scores bs
                     WHERE bs.result_id = br.id AND bs.scoring_method = 'auto_judge'
                     ORDER BY bs.created_at DESC LIMIT 1)
             FROM benchmark_results br
             JOIN models m ON br.model_id = m.id
             JOIN prompts p ON br.prompt_id = p.id
             WHERE br.run_id = ?1
             ORDER BY p.category, p.sort_order, m.display_name",
        )
        .map_err(|e| format!("query error: {e}"))?;

    let results = stmt
        .query_map(rusqlite::params![run_id], |row| {
            Ok(BenchmarkResult {
                id: row.get(0)?,
                run_id: row.get(1)?,
                prompt_id: row.get(2)?,
                model_id: row.get(3)?,
                model_name: row.get(4)?,
                prompt_title: row.get(5)?,
                prompt_category: row.get(6)?,
                output: row.get(7)?,
                tokens_generated: row.get(8)?,
                time_to_first_token_ms: row.get(9)?,
                total_time_ms: row.get(10)?,
                tokens_per_second: row.get(11)?,
                created_at: row.get(12)?,
                manual_score: row.get(13)?,
                auto_judge_score: row.get(14)?,
                auto_judge_notes: row.get(15)?,
            })
        })
        .map_err(|e| format!("query error: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("row error: {e}"))?;

    Ok(results)
}

#[tauri::command]
pub async fn score_result(
    result_id: i64,
    score: i64,
    notes: Option<String>,
) -> Result<(), String> {
    if !(1..=5).contains(&score) {
        return Err(format!("Score must be between 1 and 5, got {score}"));
    }

    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    conn.execute(
        "INSERT INTO benchmark_scores (result_id, score, scoring_method, notes)
         VALUES (?1, ?2, 'manual', ?3)",
        rusqlite::params![result_id, score, notes],
    )
    .map_err(|e| format!("insert score error: {e}"))?;

    Ok(())
}

#[tauri::command]
pub async fn list_benchmark_runs(suite_id: Option<i64>) -> Result<Vec<BenchmarkRunSummary>, String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    let mut stmt = conn
        .prepare(
            "SELECT br.id, br.suite_id, ts.name, br.status,
                    (SELECT COUNT(DISTINCT res.model_id) FROM benchmark_results res WHERE res.run_id = br.id),
                    (SELECT COUNT(DISTINCT p.id) FROM prompts p WHERE p.suite_id = br.suite_id),
                    (SELECT COUNT(DISTINCT res2.id) FROM benchmark_results res2
                     JOIN benchmark_scores bs2 ON bs2.result_id = res2.id
                     WHERE res2.run_id = br.id),
                    (SELECT COUNT(*) FROM benchmark_results res3 WHERE res3.run_id = br.id),
                    br.started_at, br.completed_at
             FROM benchmark_runs br
             JOIN test_suites ts ON br.suite_id = ts.id
             WHERE (?1 IS NULL OR br.suite_id = ?1)
             ORDER BY br.id DESC",
        )
        .map_err(|e| format!("query error: {e}"))?;

    let runs = stmt
        .query_map(rusqlite::params![suite_id], |row| {
            Ok(BenchmarkRunSummary {
                id: row.get(0)?,
                suite_id: row.get(1)?,
                suite_name: row.get(2)?,
                status: row.get(3)?,
                model_count: row.get(4)?,
                prompt_count: row.get(5)?,
                scored_count: row.get(6)?,
                total_results: row.get(7)?,
                started_at: row.get(8)?,
                completed_at: row.get(9)?,
            })
        })
        .map_err(|e| format!("query error: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("row error: {e}"))?;

    Ok(runs)
}

// ---------------------------------------------------------------------------
// Auto-judge helpers
// ---------------------------------------------------------------------------

fn build_judge_prompt(prompt_text: &str, eval_criteria: Option<&str>, output: &str) -> String {
    let criteria = eval_criteria
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("Assess accuracy, completeness, clarity, and helpfulness.");

    format!(
        "You are an expert judge evaluating AI model outputs. Score the following output on a scale of 1-10.\n\n\
         ## Original Prompt\n{prompt_text}\n\n\
         ## Evaluation Criteria\n{criteria}\n\n\
         ## Model Output to Judge\n{output}\n\n\
         Respond ONLY with valid JSON: {{\"score\": <1-10>, \"reasoning\": \"<brief explanation>\"}}",
    )
}

fn parse_judge_response(response: &str) -> Option<(i64, String)> {
    // Try serde_json first
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(response.trim()) {
        if let (Some(score), reasoning) = (
            v.get("score").and_then(|s| s.as_i64()),
            v.get("reasoning")
                .and_then(|r| r.as_str())
                .unwrap_or("")
                .to_string(),
        ) {
            if (1..=10).contains(&score) {
                return Some((score, reasoning));
            }
        }
    }

    // Fallback: regex-style scan for "score": N
    let score = response
        .find("\"score\"")
        .and_then(|idx| {
            let after = &response[idx + 7..];
            let after = after.trim_start_matches([':', ' ']);
            after
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<i64>()
                .ok()
        })
        .filter(|&s| (1..=10).contains(&s));

    score.map(|s| (s, String::new()))
}

// ---------------------------------------------------------------------------
// Auto-judge command
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn auto_judge_benchmark(
    app: tauri::AppHandle,
    state: tauri::State<'_, ActiveJudgeRuns>,
    run_id: i64,
    judge_model_id: i64,
) -> Result<(), String> {
    // Get judge model name
    let judge_model_name: String = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        conn.query_row(
            "SELECT name FROM models WHERE id = ?1",
            rusqlite::params![judge_model_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("judge model not found (id={judge_model_id}): {e}"))?
    };

    // Collect results that need judging: no auto_judge score yet, not self-judging
    // (result_id, model_name, prompt_text, eval_criteria, output)
    let tasks: Vec<(i64, String, String, Option<String>, String)> = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT br.id, m.name, p.text, p.eval_criteria, br.output
                 FROM benchmark_results br
                 JOIN models m ON br.model_id = m.id
                 JOIN prompts p ON br.prompt_id = p.id
                 WHERE br.run_id = ?1
                   AND br.model_id != ?2
                   AND NOT EXISTS (
                       SELECT 1 FROM benchmark_scores bs
                       WHERE bs.result_id = br.id
                         AND bs.scoring_method = 'auto_judge'
                   )
                 ORDER BY br.id",
            )
            .map_err(|e| format!("query error: {e}"))?;

        let rows = stmt
            .query_map(rusqlite::params![run_id, judge_model_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(|e| format!("query error: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("row error: {e}"))?;
        rows
    };

    if tasks.is_empty() {
        let _ = app.emit(
            "autojudge:complete",
            AutoJudgeCompletePayload {
                run_id,
                scores_added: 0,
            },
        );
        return Ok(());
    }

    let total = tasks.len() as i32;
    let cancel_token = CancellationToken::new();
    {
        let mut map = state.0.lock().map_err(|e| format!("state lock: {e}"))?;
        map.insert(run_id, cancel_token.clone());
    }

    let active_map = Arc::clone(&state.0);

    tokio::spawn(async move {
        let mut scores_added = 0i32;

        for (i, (result_id, model_name, prompt_text, eval_criteria, output)) in
            tasks.iter().enumerate()
        {
            if cancel_token.is_cancelled() {
                break;
            }

            let judge_prompt = build_judge_prompt(
                prompt_text,
                eval_criteria.as_deref(),
                output,
            );

            let req = GenerateRequest {
                model: judge_model_name.clone(),
                prompt: judge_prompt,
                system: Some(
                    "You are a strict but fair evaluator. Output only valid JSON.".into(),
                ),
                num_predict: Some(256),
                temperature: Some(0.1),
            };

            let rx = match ollama::generate_stream(req).await {
                Ok(rx) => rx,
                Err(e) => {
                    eprintln!("auto_judge: stream error for result {result_id}: {e}");
                    continue;
                }
            };

            // Collect full response without streaming to UI
            let mut buffer = String::new();
            let mut rx = rx;
            loop {
                tokio::select! {
                    _ = cancel_token.cancelled() => break,
                    chunk = rx.recv() => {
                        match chunk {
                            Some(Ok(c)) => {
                                if let Some(ref text) = c.response {
                                    buffer.push_str(text);
                                }
                                if c.done { break; }
                            }
                            Some(Err(e)) => {
                                eprintln!("auto_judge: chunk error: {e}");
                                break;
                            }
                            None => break,
                        }
                    }
                }
            }

            if cancel_token.is_cancelled() {
                break;
            }

            match parse_judge_response(&buffer) {
                Some((score, reasoning)) => {
                    let save_result = (|| -> Result<(), String> {
                        let conn =
                            db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
                        conn.execute(
                            "INSERT INTO benchmark_scores
                                (result_id, score, scoring_method, judge_model_id, notes)
                             VALUES (?1, ?2, 'auto_judge', ?3, ?4)",
                            rusqlite::params![
                                result_id,
                                score,
                                judge_model_id,
                                if reasoning.is_empty() {
                                    None
                                } else {
                                    Some(reasoning)
                                },
                            ],
                        )
                        .map_err(|e| format!("insert score error: {e}"))?;
                        Ok(())
                    })();

                    if let Err(e) = save_result {
                        eprintln!("auto_judge: save error: {e}");
                    } else {
                        scores_added += 1;
                    }
                }
                None => {
                    eprintln!(
                        "auto_judge: failed to parse response for result {result_id}: {:?}",
                        &buffer[..buffer.len().min(200)]
                    );
                }
            }

            let _ = app.emit(
                "autojudge:progress",
                AutoJudgeProgressPayload {
                    run_id,
                    completed: i as i32 + 1,
                    total,
                    current_model: model_name.clone(),
                },
            );
        }

        let _ = app.emit(
            "autojudge:complete",
            AutoJudgeCompletePayload {
                run_id,
                scores_added,
            },
        );

        cleanup_token(&active_map, run_id);
    });

    Ok(())
}

#[tauri::command]
pub async fn cancel_auto_judge(
    state: tauri::State<'_, ActiveJudgeRuns>,
    run_id: i64,
) -> Result<(), String> {
    let map = state.0.lock().map_err(|e| format!("state lock: {e}"))?;
    if let Some(token) = map.get(&run_id) {
        token.cancel();
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Leaderboard command
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_benchmark_leaderboard() -> Result<Vec<BenchmarkLeaderboardEntry>, String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;

    // Pass 1: overall avg normalized score + perf metrics per model
    #[derive(Debug)]
    struct OverallRow {
        model_id: i64,
        model_name: String,
        display_name: String,
        avg_score: Option<f64>,
        total_prompts_scored: i64,
        avg_tps: Option<f64>,
        avg_ttft_ms: Option<f64>,
    }

    let mut overall_stmt = conn
        .prepare(
            "SELECT m.id, m.name, m.display_name,
                    AVG(CASE bs.scoring_method
                            WHEN 'manual' THEN bs.score * 2.0
                            ELSE CAST(bs.score AS REAL)
                        END),
                    COUNT(DISTINCT br.id),
                    AVG(br.tokens_per_second),
                    AVG(br.time_to_first_token_ms)
             FROM benchmark_results br
             JOIN benchmark_scores bs ON bs.result_id = br.id
             JOIN models m ON br.model_id = m.id
             GROUP BY m.id
             ORDER BY AVG(CASE bs.scoring_method
                              WHEN 'manual' THEN bs.score * 2.0
                              ELSE CAST(bs.score AS REAL)
                          END) DESC NULLS LAST",
        )
        .map_err(|e| format!("leaderboard query error: {e}"))?;

    let overall_rows: Vec<OverallRow> = overall_stmt
        .query_map([], |row| {
            Ok(OverallRow {
                model_id: row.get(0)?,
                model_name: row.get(1)?,
                display_name: row.get(2)?,
                avg_score: row.get(3)?,
                total_prompts_scored: row.get(4)?,
                avg_tps: row.get(5)?,
                avg_ttft_ms: row.get(6)?,
            })
        })
        .map_err(|e| format!("leaderboard query error: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("leaderboard row error: {e}"))?;

    // Pass 2: per-category avg normalized score per model
    let mut cat_stmt = conn
        .prepare(
            "SELECT br.model_id, p.category,
                    AVG(CASE bs.scoring_method
                            WHEN 'manual' THEN bs.score * 2.0
                            ELSE CAST(bs.score AS REAL)
                        END)
             FROM benchmark_results br
             JOIN benchmark_scores bs ON bs.result_id = br.id
             JOIN prompts p ON br.prompt_id = p.id
             GROUP BY br.model_id, p.category",
        )
        .map_err(|e| format!("category query error: {e}"))?;

    let cat_rows: Vec<(i64, String, f64)> = cat_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| format!("category query error: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("category row error: {e}"))?;

    // Build category_scores map per model_id
    let mut cat_map: HashMap<i64, HashMap<String, f64>> = HashMap::new();
    for (model_id, category, avg) in cat_rows {
        cat_map
            .entry(model_id)
            .or_default()
            .insert(category, avg);
    }

    let entries = overall_rows
        .into_iter()
        .map(|row| BenchmarkLeaderboardEntry {
            model_id: row.model_id,
            model_name: row.model_name,
            display_name: row.display_name,
            avg_score: row.avg_score,
            category_scores: cat_map.remove(&row.model_id).unwrap_or_default(),
            avg_tps: row.avg_tps,
            avg_ttft_ms: row.avg_ttft_ms,
            total_prompts_scored: row.total_prompts_scored,
        })
        .collect();

    Ok(entries)
}

// ---------------------------------------------------------------------------
// Run comparison command
// ---------------------------------------------------------------------------

// (prompt_id, model_id) → (prompt_title, category, model_name, run_a_score, run_b_score)
type ComparisonMap = HashMap<(i64, i64), (String, String, String, Option<f64>, Option<f64>)>;
// (run_id, prompt_id, model_id, title, category, model_name, avg_score)
type ComparisonRow = (i64, i64, i64, String, String, String, Option<f64>);

#[tauri::command]
pub async fn get_run_comparison(
    run_a: i64,
    run_b: i64,
) -> Result<Vec<RunComparisonEntry>, String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;

    // Avg normalized score per (run, prompt, model)
    let mut stmt = conn
        .prepare(
            "SELECT br.run_id, br.prompt_id, br.model_id,
                    p.title, p.category, m.display_name,
                    AVG(CASE bs.scoring_method
                            WHEN 'manual' THEN bs.score * 2.0
                            ELSE CAST(bs.score AS REAL)
                        END)
             FROM benchmark_results br
             JOIN benchmark_scores bs ON bs.result_id = br.id
             JOIN prompts p ON br.prompt_id = p.id
             JOIN models m ON br.model_id = m.id
             WHERE br.run_id IN (?1, ?2)
             GROUP BY br.run_id, br.prompt_id, br.model_id",
        )
        .map_err(|e| format!("comparison query error: {e}"))?;

    // Key: (prompt_id, model_id) → (title, category, model_name, run_a_score, run_b_score)
    let mut map: ComparisonMap = HashMap::new();

    let rows: Vec<ComparisonRow> = stmt
        .query_map(rusqlite::params![run_a, run_b], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        })
        .map_err(|e| format!("comparison query error: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("comparison row error: {e}"))?;

    for (run_id, prompt_id, model_id, title, category, model_name, score) in rows {
        let entry = map
            .entry((prompt_id, model_id))
            .or_insert_with(|| (title, category, model_name, None, None));
        if run_id == run_a {
            entry.3 = score;
        } else {
            entry.4 = score;
        }
    }

    let mut entries: Vec<RunComparisonEntry> = map
        .into_iter()
        .map(
            |((prompt_id, model_id), (title, category, model_name, a_score, b_score))| {
                let delta = match (a_score, b_score) {
                    (Some(a), Some(b)) => Some(b - a),
                    _ => None,
                };
                RunComparisonEntry {
                    prompt_id,
                    prompt_title: title,
                    prompt_category: category,
                    model_id,
                    model_name,
                    run_a_score: a_score,
                    run_b_score: b_score,
                    score_delta: delta,
                }
            },
        )
        .collect();

    entries.sort_by(|a, b| {
        a.prompt_category
            .cmp(&b.prompt_category)
            .then(a.prompt_id.cmp(&b.prompt_id))
            .then(a.model_name.cmp(&b.model_name))
    });

    Ok(entries)
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

    #[test]
    fn benchmark_result_serializes() {
        let r = BenchmarkResult {
            id: 1,
            run_id: 2,
            prompt_id: 3,
            model_id: 4,
            model_name: "Llama 3".into(),
            prompt_title: "Coding challenge".into(),
            prompt_category: "Coding".into(),
            output: "Here is the solution...".into(),
            tokens_generated: 128,
            time_to_first_token_ms: Some(250),
            total_time_ms: 3200,
            tokens_per_second: Some(40.0),
            manual_score: Some(4),
            auto_judge_score: Some(8),
            auto_judge_notes: Some("Good reasoning".into()),
            created_at: "2026-01-01T00:00:00".into(),
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"model_name\":\"Llama 3\""));
        assert!(json.contains("\"manual_score\":4"));
        assert!(json.contains("\"auto_judge_score\":8"));
    }

    #[test]
    fn auto_judge_progress_payload_serializes() {
        let p = AutoJudgeProgressPayload {
            run_id: 5,
            completed: 3,
            total: 10,
            current_model: "mistral".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"run_id\":5"));
        assert!(json.contains("\"completed\":3"));
        assert!(json.contains("\"total\":10"));
    }

    #[test]
    fn build_judge_prompt_uses_criteria() {
        let prompt = build_judge_prompt(
            "Write a poem about Rust",
            Some("Check creativity and technical accuracy"),
            "Rust is memory safe...",
        );
        assert!(prompt.contains("Write a poem about Rust"));
        assert!(prompt.contains("Check creativity and technical accuracy"));
        assert!(prompt.contains("Rust is memory safe..."));
        assert!(prompt.contains("{\"score\": <1-10>"));
    }

    #[test]
    fn build_judge_prompt_falls_back_default_criteria() {
        let prompt = build_judge_prompt("A question", None, "An answer");
        assert!(prompt.contains("accuracy, completeness, clarity"));
    }

    #[test]
    fn build_judge_prompt_ignores_empty_criteria() {
        let prompt = build_judge_prompt("A question", Some("   "), "An answer");
        assert!(prompt.contains("accuracy, completeness, clarity"));
    }

    #[test]
    fn parse_judge_response_valid_json() {
        let resp = r#"{"score": 7, "reasoning": "Good answer"}"#;
        let result = parse_judge_response(resp);
        assert_eq!(result, Some((7, "Good answer".into())));
    }

    #[test]
    fn parse_judge_response_rejects_out_of_range() {
        let resp = r#"{"score": 11, "reasoning": "Too high"}"#;
        let result = parse_judge_response(resp);
        assert!(result.is_none());
    }

    #[test]
    fn parse_judge_response_fallback_regex() {
        let resp = r#"Here is my evaluation: "score": 6 because it was decent."#;
        let result = parse_judge_response(resp);
        assert_eq!(result.map(|(s, _)| s), Some(6));
    }

    #[test]
    fn parse_judge_response_invalid_returns_none() {
        let resp = "I cannot score this response.";
        let result = parse_judge_response(resp);
        assert!(result.is_none());
    }

    #[test]
    fn run_comparison_entry_serializes() {
        let e = RunComparisonEntry {
            prompt_id: 1,
            prompt_title: "Reasoning".into(),
            prompt_category: "Logic".into(),
            model_id: 2,
            model_name: "gemma3".into(),
            run_a_score: Some(6.0),
            run_b_score: Some(8.5),
            score_delta: Some(2.5),
        };
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"score_delta\":2.5"));
        assert!(json.contains("\"run_a_score\":6.0"));
    }

    #[test]
    fn benchmark_leaderboard_entry_serializes_with_categories() {
        let mut cats = HashMap::new();
        cats.insert("Coding".into(), 8.2);
        cats.insert("Logic".into(), 6.5);
        let e = BenchmarkLeaderboardEntry {
            model_id: 1,
            model_name: "llama3".into(),
            display_name: "Llama 3".into(),
            avg_score: Some(7.35),
            category_scores: cats,
            avg_tps: Some(42.1),
            avg_ttft_ms: Some(310.0),
            total_prompts_scored: 20,
        };
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"display_name\":\"Llama 3\""));
        assert!(json.contains("\"total_prompts_scored\":20"));
        assert!(json.contains("Coding"));
    }
}
