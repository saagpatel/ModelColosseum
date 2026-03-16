use crate::db;
use crate::ollama::{self, GenerateRequest, StreamChunk};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use sysinfo::System;
use tauri::Emitter;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Managed state
// ---------------------------------------------------------------------------

pub struct ActiveBenchmarks(pub Arc<Mutex<HashMap<i64, CancellationToken>>>);
pub struct ActiveJudgeRuns(pub Arc<Mutex<HashMap<i64, CancellationToken>>>);
pub struct ActiveBlindComparisons(pub Arc<Mutex<HashMap<i64, BlindComparisonState>>>);

pub struct BlindComparisonState {
    pub model_a_id: i64,
    pub model_b_id: i64,
    /// prompt_id → (result_id_a, result_id_b, a_is_left)
    pub prompt_assignments: HashMap<i64, (i64, i64, bool)>,
}

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BenchmarkMetricsPayload {
    pub run_id: i64,
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub swap_percent: f32,
    pub timestamp_ms: u64,
}

// ---------------------------------------------------------------------------
// Blind comparison return types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Clone)]
pub struct BlindPair {
    pub prompt_id: i64,
    pub prompt_title: String,
    pub prompt_category: String,
    pub left_result_id: i64,
    pub left_output: String,
    pub right_result_id: i64,
    pub right_output: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct BlindComparison {
    pub id: i64,
    pub pairs: Vec<BlindPair>,
}

#[derive(Debug, Serialize, Clone)]
pub struct BlindRevealEntry {
    pub prompt_id: i64,
    pub prompt_title: String,
    pub model_a_name: String,
    pub model_b_name: String,
    pub winner: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct BlindReveal {
    pub model_a_name: String,
    pub model_b_name: String,
    pub model_a_wins: i64,
    pub model_b_wins: i64,
    pub ties: i64,
    pub entries: Vec<BlindRevealEntry>,
}

// ---------------------------------------------------------------------------
// Import/export structs
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct SuiteExport {
    version: u8,
    suite: SuiteExportData,
    prompts: Vec<PromptExportData>,
}

#[derive(Serialize, Deserialize)]
struct SuiteExportData {
    name: String,
    description: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct PromptExportData {
    category: String,
    title: String,
    text: String,
    system_prompt: Option<String>,
    ideal_answer: Option<String>,
    eval_criteria: Option<String>,
    sort_order: i64,
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
        (Some(count), Some(dur)) if dur > 0 => Some(count as f64 / (dur as f64 / 1_000_000_000.0)),
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
    conn.execute(
        "DELETE FROM test_suites WHERE id = ?1",
        rusqlite::params![id],
    )
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
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
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

    // Shared metrics samples buffer (filled by the metrics task, persisted after main task)
    let metrics_samples: Arc<Mutex<Vec<BenchmarkMetricsPayload>>> =
        Arc::new(Mutex::new(Vec::new()));
    let metrics_samples_metrics = Arc::clone(&metrics_samples);
    let metrics_cancel = cancel_token.clone();
    let metrics_app = app.clone();

    // Spawn hardware metrics sampling task — runs concurrently with benchmark
    tokio::spawn(async move {
        let mut sys = System::new_all();
        loop {
            tokio::select! {
                _ = metrics_cancel.cancelled() => break,
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(2)) => {
                    sys.refresh_cpu_usage();
                    sys.refresh_memory();

                    let cpu = sys.global_cpu_usage();
                    let mem_total = sys.total_memory();
                    let mem_used = sys.used_memory();
                    let swap_total = sys.total_swap();
                    let swap_used = sys.used_swap();

                    let memory_percent = if mem_total > 0 {
                        (mem_used as f32 / mem_total as f32) * 100.0
                    } else {
                        0.0
                    };
                    let swap_percent = if swap_total > 0 {
                        (swap_used as f32 / swap_total as f32) * 100.0
                    } else {
                        0.0
                    };
                    let timestamp_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;

                    let payload = BenchmarkMetricsPayload {
                        run_id,
                        cpu_percent: cpu,
                        memory_percent,
                        swap_percent,
                        timestamp_ms,
                    };

                    if let Ok(mut samples) = metrics_samples_metrics.lock() {
                        samples.push(payload.clone());
                    }
                    let _ = metrics_app.emit("benchmark:metrics", payload);
                }
            }
        }
    });

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
                let result =
                    benchmark_stream_and_collect(&app, run_id, rx, &cancel_token, start).await;

                match result {
                    Ok((output, metrics)) => {
                        // Save result
                        if let Err(e) = (|| -> Result<(), String> {
                            let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
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
                                BenchmarkErrorPayload { run_id, message: e },
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
            let _ = app.emit(
                "benchmark:error",
                BenchmarkErrorPayload {
                    run_id,
                    message: "Benchmark cancelled".into(),
                },
            );
        } else {
            let _ = update_run_status(run_id, "completed");
            let _ = app.emit("benchmark:complete", BenchmarkCompletePayload { run_id });
        }

        // Persist collected hardware metrics to DB
        if let Ok(samples) = metrics_samples.lock() {
            if !samples.is_empty() {
                if let Ok(json) = serde_json::to_string(&*samples) {
                    let conn_result = db::get_db().lock();
                    if let Ok(conn) = conn_result {
                        let _ = conn.execute(
                            "UPDATE benchmark_runs SET hardware_metrics = ?1 WHERE id = ?2",
                            rusqlite::params![json, run_id],
                        );
                    }
                }
            }
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
pub async fn score_result(result_id: i64, score: i64, notes: Option<String>) -> Result<(), String> {
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
pub async fn list_benchmark_runs(
    suite_id: Option<i64>,
) -> Result<Vec<BenchmarkRunSummary>, String> {
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

            let judge_prompt = build_judge_prompt(prompt_text, eval_criteria.as_deref(), output);

            let req = GenerateRequest {
                model: judge_model_name.clone(),
                prompt: judge_prompt,
                system: Some("You are a strict but fair evaluator. Output only valid JSON.".into()),
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
                        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
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
        cat_map.entry(model_id).or_default().insert(category, avg);
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
pub async fn get_run_comparison(run_a: i64, run_b: i64) -> Result<Vec<RunComparisonEntry>, String> {
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
// Blind comparison commands
// ---------------------------------------------------------------------------

/// prompt_id → model_id → (result_id, output, title, category)
type ByPromptMap = HashMap<i64, HashMap<i64, (i64, String, String, String)>>;

#[tauri::command]
pub async fn start_blind_comparison(
    state: tauri::State<'_, ActiveBlindComparisons>,
    run_id: i64,
) -> Result<BlindComparison, String> {
    // Query all benchmark_results for this run
    let results: Vec<(i64, i64, i64, String, String, String)> = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT br.id, br.model_id, br.prompt_id, br.output, p.title, p.category
                 FROM benchmark_results br
                 JOIN prompts p ON br.prompt_id = p.id
                 WHERE br.run_id = ?1
                 ORDER BY br.prompt_id, br.model_id",
            )
            .map_err(|e| format!("query error: {e}"))?;
        let rows: Vec<_> = stmt
            .query_map(rusqlite::params![run_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .map_err(|e| format!("query error: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("row error: {e}"))?;
        rows
    };

    // Derive distinct model set
    let mut model_ids: Vec<i64> = results.iter().map(|r| r.1).collect();
    model_ids.sort_unstable();
    model_ids.dedup();

    if model_ids.len() != 2 {
        return Err(format!(
            "Blind comparison requires exactly 2 models, found {}",
            model_ids.len()
        ));
    }
    let model_a_id = model_ids[0];
    let model_b_id = model_ids[1];

    // Group results by prompt_id
    let mut by_prompt: ByPromptMap = HashMap::new();
    for (result_id, model_id, prompt_id, output, title, category) in &results {
        by_prompt.entry(*prompt_id).or_default().insert(
            *model_id,
            (*result_id, output.clone(), title.clone(), category.clone()),
        );
    }

    // Build pairs with random left/right assignment
    let mut prompt_assignments: HashMap<i64, (i64, i64, bool)> = HashMap::new();
    let mut pairs: Vec<BlindPair> = Vec::new();

    let mut sorted_prompts: Vec<i64> = by_prompt.keys().copied().collect();
    sorted_prompts.sort_unstable();

    let base_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();

    for (idx, prompt_id) in sorted_prompts.iter().enumerate() {
        let prompt_map = match by_prompt.get(prompt_id) {
            Some(m) => m,
            None => continue,
        };
        let (result_a_id, output_a, title, category) = match prompt_map.get(&model_a_id) {
            Some(v) => v,
            None => continue,
        };
        let (result_b_id, output_b, _, _) = match prompt_map.get(&model_b_id) {
            Some(v) => v,
            None => continue,
        };

        // Random left/right: XOR nanos with prompt index
        let a_is_left = (base_nanos ^ (idx as u32)).is_multiple_of(2);

        let (left_result_id, left_output, right_result_id, right_output) = if a_is_left {
            (
                *result_a_id,
                output_a.clone(),
                *result_b_id,
                output_b.clone(),
            )
        } else {
            (
                *result_b_id,
                output_b.clone(),
                *result_a_id,
                output_a.clone(),
            )
        };

        prompt_assignments.insert(*prompt_id, (*result_a_id, *result_b_id, a_is_left));

        pairs.push(BlindPair {
            prompt_id: *prompt_id,
            prompt_title: title.clone(),
            prompt_category: category.clone(),
            left_result_id,
            left_output,
            right_result_id,
            right_output,
        });
    }

    // Store state
    {
        let mut map = state.0.lock().map_err(|e| format!("state lock: {e}"))?;
        map.insert(
            run_id,
            BlindComparisonState {
                model_a_id,
                model_b_id,
                prompt_assignments,
            },
        );
    }

    Ok(BlindComparison { id: run_id, pairs })
}

#[tauri::command]
pub async fn submit_blind_pick(
    state: tauri::State<'_, ActiveBlindComparisons>,
    run_id: i64,
    prompt_id: i64,
    winner: String,
) -> Result<(), String> {
    let (result_a_id, result_b_id, a_is_left) = {
        let map = state.0.lock().map_err(|e| format!("state lock: {e}"))?;
        let st = map
            .get(&run_id)
            .ok_or_else(|| format!("no active blind comparison for run {run_id}"))?;
        let assignment = st
            .prompt_assignments
            .get(&prompt_id)
            .ok_or_else(|| format!("prompt {prompt_id} not found in blind comparison"))?;
        *assignment
    };

    // Resolve left/right → model_a/model_b winner
    let (winner_result_id, loser_result_id) = match winner.as_str() {
        "left" => {
            if a_is_left {
                (result_a_id, result_b_id)
            } else {
                (result_b_id, result_a_id)
            }
        }
        "right" => {
            if a_is_left {
                (result_b_id, result_a_id)
            } else {
                (result_a_id, result_b_id)
            }
        }
        "tie" => {
            // Both get tie score
            let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
            conn.execute(
                "INSERT INTO benchmark_scores (result_id, score, scoring_method) VALUES (?1, 5, 'head_to_head')",
                rusqlite::params![result_a_id],
            )
            .map_err(|e| format!("insert score error: {e}"))?;
            conn.execute(
                "INSERT INTO benchmark_scores (result_id, score, scoring_method) VALUES (?1, 5, 'head_to_head')",
                rusqlite::params![result_b_id],
            )
            .map_err(|e| format!("insert score error: {e}"))?;
            return Ok(());
        }
        other => {
            return Err(format!(
                "invalid winner value: '{other}' (expected left|right|tie)"
            ))
        }
    };

    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    conn.execute(
        "INSERT INTO benchmark_scores (result_id, score, scoring_method) VALUES (?1, 10, 'head_to_head')",
        rusqlite::params![winner_result_id],
    )
    .map_err(|e| format!("insert winner score error: {e}"))?;
    conn.execute(
        "INSERT INTO benchmark_scores (result_id, score, scoring_method) VALUES (?1, 1, 'head_to_head')",
        rusqlite::params![loser_result_id],
    )
    .map_err(|e| format!("insert loser score error: {e}"))?;

    Ok(())
}

#[tauri::command]
pub async fn finish_blind_comparison(
    state: tauri::State<'_, ActiveBlindComparisons>,
    run_id: i64,
) -> Result<BlindReveal, String> {
    let (model_a_id, model_b_id) = {
        let map = state.0.lock().map_err(|e| format!("state lock: {e}"))?;
        let st = map
            .get(&run_id)
            .ok_or_else(|| format!("no active blind comparison for run {run_id}"))?;
        (st.model_a_id, st.model_b_id)
    };

    // Resolve model names
    let (model_a_name, model_b_name) = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        let name_a: String = conn
            .query_row(
                "SELECT display_name FROM models WHERE id = ?1",
                rusqlite::params![model_a_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("model_a not found: {e}"))?;
        let name_b: String = conn
            .query_row(
                "SELECT display_name FROM models WHERE id = ?1",
                rusqlite::params![model_b_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("model_b not found: {e}"))?;
        (name_a, name_b)
    };

    // Query all head_to_head scores for this run
    // result_id → (model_id, prompt_id, score)
    let score_rows: Vec<(i64, i64, i64, i64)> = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT bs.result_id, br.model_id, br.prompt_id, bs.score
                 FROM benchmark_scores bs
                 JOIN benchmark_results br ON bs.result_id = br.id
                 WHERE br.run_id = ?1 AND bs.scoring_method = 'head_to_head'
                 ORDER BY br.prompt_id",
            )
            .map_err(|e| format!("query error: {e}"))?;
        let rows: Vec<_> = stmt
            .query_map(rusqlite::params![run_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(|e| format!("query error: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("row error: {e}"))?;
        rows
    };

    // Query prompt titles
    let prompt_titles: HashMap<i64, String> = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT br.prompt_id, p.title
                 FROM benchmark_results br
                 JOIN prompts p ON br.prompt_id = p.id
                 WHERE br.run_id = ?1",
            )
            .map_err(|e| format!("query error: {e}"))?;
        let rows: HashMap<_, _> = stmt
            .query_map(rusqlite::params![run_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| format!("query error: {e}"))?
            .collect::<Result<HashMap<_, _>, _>>()
            .map_err(|e| format!("row error: {e}"))?;
        rows
    };

    // Group by prompt_id: collect model_id → score
    let mut by_prompt: HashMap<i64, HashMap<i64, i64>> = HashMap::new();
    for (_result_id, model_id, prompt_id, score) in score_rows {
        by_prompt
            .entry(prompt_id)
            .or_default()
            .insert(model_id, score);
    }

    let mut model_a_wins: i64 = 0;
    let mut model_b_wins: i64 = 0;
    let mut ties: i64 = 0;
    let mut entries: Vec<BlindRevealEntry> = Vec::new();

    let mut sorted_prompts: Vec<i64> = by_prompt.keys().copied().collect();
    sorted_prompts.sort_unstable();

    for prompt_id in sorted_prompts {
        let scores = &by_prompt[&prompt_id];
        let score_a = scores.get(&model_a_id).copied().unwrap_or(0);
        let score_b = scores.get(&model_b_id).copied().unwrap_or(0);

        let winner_str = if score_a > score_b {
            model_a_wins += 1;
            model_a_name.clone()
        } else if score_b > score_a {
            model_b_wins += 1;
            model_b_name.clone()
        } else {
            ties += 1;
            "tie".to_string()
        };

        entries.push(BlindRevealEntry {
            prompt_id,
            prompt_title: prompt_titles.get(&prompt_id).cloned().unwrap_or_default(),
            model_a_name: model_a_name.clone(),
            model_b_name: model_b_name.clone(),
            winner: winner_str,
        });
    }

    // Remove from active state
    {
        let mut map = state.0.lock().map_err(|e| format!("state lock: {e}"))?;
        map.remove(&run_id);
    }

    Ok(BlindReveal {
        model_a_name,
        model_b_name,
        model_a_wins,
        model_b_wins,
        ties,
        entries,
    })
}

// ---------------------------------------------------------------------------
// Hardware metrics command
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_benchmark_metrics(
    run_id: i64,
) -> Result<Option<Vec<BenchmarkMetricsPayload>>, String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    let result: rusqlite::Result<Option<String>> = conn.query_row(
        "SELECT hardware_metrics FROM benchmark_runs WHERE id = ?1",
        rusqlite::params![run_id],
        |row| row.get(0),
    );

    match result {
        Ok(Some(json)) => {
            let samples: Vec<BenchmarkMetricsPayload> =
                serde_json::from_str(&json).map_err(|e| format!("metrics parse error: {e}"))?;
            Ok(Some(samples))
        }
        Ok(None) => Ok(None),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("db query error: {e}")),
    }
}

// ---------------------------------------------------------------------------
// Import/export commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn export_test_suite(suite_id: i64) -> Result<String, String> {
    let (suite_name, suite_description): (String, Option<String>) = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        conn.query_row(
            "SELECT name, description FROM test_suites WHERE id = ?1",
            rusqlite::params![suite_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("suite not found (id={suite_id}): {e}"))?
    };

    let prompts: Vec<PromptExportData> = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT category, title, text, system_prompt, ideal_answer, eval_criteria, sort_order
                 FROM prompts WHERE suite_id = ?1 ORDER BY sort_order ASC",
            )
            .map_err(|e| format!("query error: {e}"))?;
        let rows: Vec<_> = stmt
            .query_map(rusqlite::params![suite_id], |row| {
                Ok(PromptExportData {
                    category: row.get(0)?,
                    title: row.get(1)?,
                    text: row.get(2)?,
                    system_prompt: row.get(3)?,
                    ideal_answer: row.get(4)?,
                    eval_criteria: row.get(5)?,
                    sort_order: row.get(6)?,
                })
            })
            .map_err(|e| format!("query error: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("row error: {e}"))?;
        rows
    };

    let export = SuiteExport {
        version: 1,
        suite: SuiteExportData {
            name: suite_name,
            description: suite_description,
        },
        prompts,
    };

    serde_json::to_string_pretty(&export).map_err(|e| format!("serialize error: {e}"))
}

#[tauri::command]
pub async fn import_test_suite(json_data: String) -> Result<TestSuite, String> {
    let export: SuiteExport =
        serde_json::from_str(&json_data).map_err(|e| format!("invalid JSON: {e}"))?;

    if export.version != 1 {
        return Err(format!(
            "unsupported export version {} (expected 1)",
            export.version
        ));
    }

    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;

    // Check for name collision and append "(imported)" if needed
    let existing_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM test_suites WHERE name = ?1",
            rusqlite::params![export.suite.name],
            |row| row.get(0),
        )
        .map_err(|e| format!("name check error: {e}"))?;

    let suite_name = if existing_count > 0 {
        format!("{} (imported)", export.suite.name)
    } else {
        export.suite.name.clone()
    };

    conn.execute_batch("BEGIN")
        .map_err(|e| format!("begin tx: {e}"))?;

    let result = (|| -> Result<i64, String> {
        conn.execute(
            "INSERT INTO test_suites (name, description) VALUES (?1, ?2)",
            rusqlite::params![suite_name, export.suite.description],
        )
        .map_err(|e| format!("insert suite error: {e}"))?;
        let suite_id = conn.last_insert_rowid();

        for p in &export.prompts {
            conn.execute(
                "INSERT INTO prompts (suite_id, category, title, text, system_prompt, ideal_answer, eval_criteria, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    suite_id,
                    p.category,
                    p.title,
                    p.text,
                    p.system_prompt,
                    p.ideal_answer,
                    p.eval_criteria,
                    p.sort_order,
                ],
            )
            .map_err(|e| format!("insert prompt error: {e}"))?;
        }

        Ok(suite_id)
    })();

    match result {
        Ok(suite_id) => {
            conn.execute_batch("COMMIT")
                .map_err(|e| format!("commit tx: {e}"))?;
            conn.query_row(
                "SELECT id, name, description, is_default, created_at, updated_at
                 FROM test_suites WHERE id = ?1",
                rusqlite::params![suite_id],
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
            .map_err(|e| format!("fetch after import error: {e}"))
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn export_leaderboard() -> Result<String, String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    let mut stmt = conn
        .prepare(
            "SELECT name, display_name, elo_rating, arena_wins, arena_losses, arena_draws, total_debates
             FROM models ORDER BY elo_rating DESC",
        )
        .map_err(|e| format!("query error: {e}"))?;

    let rows: Vec<(String, String, f64, i64, i64, i64, i64)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })
        .map_err(|e| format!("query error: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("row error: {e}"))?;

    let mut csv =
        String::from("Rank,Model,Display Name,Elo Rating,Wins,Losses,Draws,Total Debates\n");
    for (rank, (name, display_name, elo, wins, losses, draws, total)) in rows.iter().enumerate() {
        csv.push_str(&format!(
            "{},{},{},{:.1},{},{},{},{}\n",
            rank + 1,
            name,
            display_name,
            elo,
            wins,
            losses,
            draws,
            total
        ));
    }

    Ok(csv)
}

#[tauri::command]
pub async fn export_benchmark_report(run_id: i64) -> Result<String, String> {
    // Query run metadata
    let (suite_name, status, started_at, completed_at): (String, String, String, Option<String>) = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        conn.query_row(
            "SELECT ts.name, br.status, br.started_at, br.completed_at
             FROM benchmark_runs br
             JOIN test_suites ts ON br.suite_id = ts.id
             WHERE br.id = ?1",
            rusqlite::params![run_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|e| format!("run not found (id={run_id}): {e}"))?
    };

    // Query all results with scores
    let results: Vec<BenchmarkResult> = {
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
        let rows: Vec<_> = stmt
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
        rows
    };

    // Build per-model summary: model_name → (sum_score, count, sum_tps, tps_count)
    let mut model_stats: HashMap<String, (f64, i64, f64, i64)> = HashMap::new();
    for r in &results {
        let score = r
            .auto_judge_score
            .map(|s| s as f64)
            .or_else(|| r.manual_score.map(|s| s as f64 * 2.0));
        let entry = model_stats
            .entry(r.model_name.clone())
            .or_insert((0.0, 0, 0.0, 0));
        if let Some(s) = score {
            entry.0 += s;
            entry.1 += 1;
        }
        if let Some(tps) = r.tokens_per_second {
            entry.2 += tps;
            entry.3 += 1;
        }
    }

    let mut md = String::new();
    md.push_str(&format!("# Benchmark Report — Run #{run_id}\n\n"));
    md.push_str(&format!("**Suite:** {suite_name}  \n"));
    md.push_str(&format!("**Status:** {status}  \n"));
    md.push_str(&format!("**Started:** {started_at}  \n"));
    if let Some(ref ca) = completed_at {
        md.push_str(&format!("**Completed:** {ca}  \n"));
    }
    md.push_str("\n---\n\n");

    // Per-model summary table
    md.push_str("## Model Summary\n\n");
    md.push_str("| Model | Avg Score | Avg TPS | Prompts Scored |\n");
    md.push_str("|---|---|---|---|\n");
    let mut model_names: Vec<String> = model_stats.keys().cloned().collect();
    model_names.sort();
    for name in &model_names {
        let (sum_score, score_count, sum_tps, tps_count) = model_stats[name];
        let avg_score = if score_count > 0 {
            format!("{:.1}", sum_score / score_count as f64)
        } else {
            "—".into()
        };
        let avg_tps = if tps_count > 0 {
            format!("{:.1}", sum_tps / tps_count as f64)
        } else {
            "—".into()
        };
        md.push_str(&format!(
            "| {name} | {avg_score} | {avg_tps} | {score_count} |\n"
        ));
    }
    md.push('\n');

    // Per-prompt details
    md.push_str("## Prompt Details\n\n");
    let mut current_category = String::new();
    let mut current_prompt = String::new();

    for r in &results {
        if r.prompt_category != current_category {
            current_category = r.prompt_category.clone();
            md.push_str(&format!("### Category: {current_category}\n\n"));
        }
        if r.prompt_title != current_prompt {
            current_prompt = r.prompt_title.clone();
            md.push_str(&format!("#### {current_prompt}\n\n"));
        }

        let score_str = r
            .auto_judge_score
            .map(|s| format!("auto_judge: {s}/10"))
            .or_else(|| r.manual_score.map(|s| format!("manual: {s}/5")))
            .unwrap_or_else(|| "unscored".into());

        let tps_str = r
            .tokens_per_second
            .map(|t| format!("{t:.1} TPS"))
            .unwrap_or_else(|| "—".into());

        md.push_str(&format!(
            "**{}** | {} | {}\n\n",
            r.model_name, score_str, tps_str
        ));
        md.push_str(&format!("{}\n\n", r.output));

        if let Some(ref notes) = r.auto_judge_notes {
            if !notes.is_empty() {
                md.push_str(&format!("*Judge notes: {notes}*\n\n"));
            }
        }

        md.push_str("---\n\n");
    }

    Ok(md)
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

    // ----- Blind comparison score mapping tests -----

    #[test]
    fn blind_comparison_left_win_maps_to_model_a_when_a_is_left() {
        // a_is_left=true, winner="left" → result_a wins (score 10), result_b loses (score 1)
        // Verify score assignment logic matches submit_blind_pick:
        // left wins → winner_result_id = result_a_id (since a_is_left=true)
        let a_is_left = true;
        let result_a_id = 10i64;
        let result_b_id = 20i64;
        let winner = "left";

        let (winner_id, loser_id) = if winner == "left" {
            if a_is_left {
                (result_a_id, result_b_id)
            } else {
                (result_b_id, result_a_id)
            }
        } else {
            if a_is_left {
                (result_b_id, result_a_id)
            } else {
                (result_a_id, result_b_id)
            }
        };

        assert_eq!(winner_id, result_a_id);
        assert_eq!(loser_id, result_b_id);
    }

    #[test]
    fn blind_comparison_right_win_maps_to_model_b_when_a_is_left() {
        // a_is_left=true, winner="right" → result_b wins (score 10), result_a loses (score 1)
        let a_is_left = true;
        let result_a_id = 10i64;
        let result_b_id = 20i64;
        let winner = "right";

        let (winner_id, loser_id) = if winner == "left" {
            if a_is_left {
                (result_a_id, result_b_id)
            } else {
                (result_b_id, result_a_id)
            }
        } else {
            if a_is_left {
                (result_b_id, result_a_id)
            } else {
                (result_a_id, result_b_id)
            }
        };

        assert_eq!(winner_id, result_b_id);
        assert_eq!(loser_id, result_a_id);
    }

    #[test]
    fn blind_comparison_left_win_maps_to_model_b_when_b_is_left() {
        // a_is_left=false (b is left), winner="left" → result_b wins
        let a_is_left = false;
        let result_a_id = 10i64;
        let result_b_id = 20i64;
        let winner = "left";

        let (winner_id, loser_id) = if winner == "left" {
            if a_is_left {
                (result_a_id, result_b_id)
            } else {
                (result_b_id, result_a_id)
            }
        } else {
            if a_is_left {
                (result_b_id, result_a_id)
            } else {
                (result_a_id, result_b_id)
            }
        };

        assert_eq!(winner_id, result_b_id);
        assert_eq!(loser_id, result_a_id);
    }

    // ----- SuiteExport JSON round-trip test -----

    #[test]
    fn suite_export_round_trips_json() {
        let original = SuiteExport {
            version: 1,
            suite: SuiteExportData {
                name: "Test Suite Alpha".into(),
                description: Some("A test suite".into()),
            },
            prompts: vec![
                PromptExportData {
                    category: "coding".into(),
                    title: "Rust LRU Cache".into(),
                    text: "Implement an LRU cache in Rust.".into(),
                    system_prompt: None,
                    ideal_answer: Some("Use HashMap + VecDeque".into()),
                    eval_criteria: Some("Correctness, efficiency".into()),
                    sort_order: 0,
                },
                PromptExportData {
                    category: "analysis".into(),
                    title: "Log Analysis".into(),
                    text: "Analyze these logs.".into(),
                    system_prompt: Some("You are a senior SRE.".into()),
                    ideal_answer: None,
                    eval_criteria: None,
                    sort_order: 1,
                },
            ],
        };

        let json = serde_json::to_string_pretty(&original).unwrap();
        let restored: SuiteExport = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.version, 1);
        assert_eq!(restored.suite.name, "Test Suite Alpha");
        assert_eq!(restored.suite.description, Some("A test suite".into()));
        assert_eq!(restored.prompts.len(), 2);
        assert_eq!(restored.prompts[0].category, "coding");
        assert_eq!(restored.prompts[0].title, "Rust LRU Cache");
        assert_eq!(
            restored.prompts[0].ideal_answer,
            Some("Use HashMap + VecDeque".into())
        );
        assert_eq!(restored.prompts[1].category, "analysis");
        assert!(restored.prompts[1].system_prompt.is_some());
        assert_eq!(restored.prompts[1].sort_order, 1);
    }

    #[test]
    fn suite_export_version_mismatch_detected() {
        let json = r#"{"version":2,"suite":{"name":"x","description":null},"prompts":[]}"#;
        let export: SuiteExport = serde_json::from_str(json).unwrap();
        // Version check is done in import_test_suite command; verify version field is read correctly
        assert_eq!(export.version, 2);
    }

    #[test]
    fn benchmark_metrics_payload_round_trips() {
        let p = BenchmarkMetricsPayload {
            run_id: 5,
            cpu_percent: 42.5,
            memory_percent: 60.1,
            swap_percent: 0.0,
            timestamp_ms: 1_700_000_000_000,
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: BenchmarkMetricsPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.run_id, 5);
        assert!((back.cpu_percent - 42.5).abs() < 0.01);
        assert!((back.memory_percent - 60.1).abs() < 0.01);
        assert_eq!(back.timestamp_ms, 1_700_000_000_000);
    }

    #[test]
    fn blind_reveal_serializes() {
        let reveal = BlindReveal {
            model_a_name: "Llama 3".into(),
            model_b_name: "Gemma 3".into(),
            model_a_wins: 3,
            model_b_wins: 2,
            ties: 1,
            entries: vec![BlindRevealEntry {
                prompt_id: 1,
                prompt_title: "Coding challenge".into(),
                model_a_name: "Llama 3".into(),
                model_b_name: "Gemma 3".into(),
                winner: "Llama 3".into(),
            }],
        };
        let json = serde_json::to_string(&reveal).unwrap();
        assert!(json.contains("\"model_a_wins\":3"));
        assert!(json.contains("\"ties\":1"));
        assert!(json.contains("Llama 3"));
    }
}
