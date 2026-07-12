use crate::db;
use crate::evaluation::{
    self, build_trial_plan, digest_json, ConfidenceSummary, DisagreementSummary, EloEligibility,
    EvaluationConfig, HardwareSnapshot, ModelSnapshot, OllamaSnapshot, PositionBiasSummary,
    PromptSnapshot, RunManifest, SideOutcome, SuiteSnapshot, TrialKind,
};
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
    /// comparison_id → (result_id_a, result_id_b, a_is_left)
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
    pub repetition_index: i64,
    pub trial_key: Option<String>,
    pub generation_seed: Option<i64>,
    pub trial_status: String,
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
    pub outcome_status: String,
    pub repetitions: i64,
    pub failed_count: i64,
    pub excluded_count: i64,
    pub comparable: bool,
    pub comparability_notes: Option<String>,
    pub manifest_digest: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct CapabilityEvidence {
    pub category: String,
    pub model_id: i64,
    pub model_name: String,
    pub scoring_method: String,
    pub confidence: ConfidenceSummary,
}

#[derive(Debug, Serialize, Clone)]
pub struct CapabilityRecommendation {
    pub category: String,
    pub recommended_model: Option<String>,
    pub confidence: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct RunEvidence {
    pub run_id: i64,
    pub outcome_status: String,
    pub manifest_digest: Option<String>,
    pub comparable: bool,
    pub comparability_notes: Option<String>,
    pub planned_measured_trials: i64,
    pub completed_measured_trials: i64,
    pub failed_trials: i64,
    pub excluded_trials: i64,
    pub cancelled_trials: i64,
    pub timeout_trials: i64,
    pub hardware_dependent: bool,
    pub capability_evidence: Vec<CapabilityEvidence>,
    pub recommendations: Vec<CapabilityRecommendation>,
    pub position_bias: PositionBiasSummary,
    pub judge_disagreement: DisagreementSummary,
    pub judge_provenance: Vec<String>,
    pub elo_eligible: bool,
    pub elo_updated: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct RunComparability {
    pub comparable: bool,
    pub classification: String,
    pub quality_comparable: bool,
    pub performance_comparable: bool,
    pub reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ReproductionReceipt {
    pub version: u32,
    pub created_at_unix_ms: u64,
    pub run_a: i64,
    pub run_b: i64,
    pub run_a_manifest_digest: String,
    pub run_b_manifest_digest: String,
    pub comparability: RunComparability,
    pub run_a_manifest: RunManifest,
    pub run_b_manifest: RunManifest,
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
    pub comparison_id: i64,
    pub prompt_id: i64,
    pub repetition_index: i64,
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

#[derive(Debug, Serialize)]
struct TrialExportData {
    trial_key: String,
    prompt_id: i64,
    model_id: i64,
    repetition_index: i64,
    trial_kind: String,
    execution_order: i64,
    generation_seed: i64,
    comparison_position: Option<String>,
    status: String,
    result_id: Option<i64>,
    error_message: Option<String>,
    exclusion_reason: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct JudgeAttemptExportData {
    result_id: i64,
    judge_model_id: i64,
    judge_manifest_json: String,
    status: String,
    raw_output: Option<String>,
    error_message: Option<String>,
    started_at: String,
    completed_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct ComparisonExportData {
    comparison_id: i64,
    prompt_id: i64,
    repetition_index: i64,
    model_a_id: i64,
    model_b_id: i64,
    result_a_id: i64,
    result_b_id: i64,
    model_a_position: String,
    human_outcome: Option<String>,
    human_winner_model_id: Option<i64>,
    human_judged_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct EvaluationBundleExport {
    version: u32,
    exported_at_unix_ms: u64,
    manifest: serde_json::Value,
    evidence: RunEvidence,
    trials: Vec<TrialExportData>,
    results: Vec<BenchmarkResult>,
    judge_attempts: Vec<JudgeAttemptExportData>,
    comparisons: Vec<ComparisonExportData>,
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
    let mut pending_emit = String::new();
    let mut last_emit = Instant::now();
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
                                pending_emit.push_str(text);
                                if pending_emit.len() >= 64
                                    || last_emit.elapsed() >= std::time::Duration::from_millis(50)
                                {
                                    let _ = app.emit("benchmark:stream", BenchmarkStreamPayload {
                                        run_id,
                                        token: std::mem::take(&mut pending_emit),
                                    });
                                    last_emit = Instant::now();
                                }
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
    if !pending_emit.is_empty() {
        let _ = app.emit(
            "benchmark:stream",
            BenchmarkStreamPayload {
                run_id,
                token: pending_emit,
            },
        );
    }
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
        "UPDATE benchmark_runs
         SET status = ?1, outcome_status = ?1, comparable = 0,
             comparability_notes = 'Run did not complete all measured trials',
             completed_at = datetime('now')
         WHERE id = ?2",
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

#[derive(Debug, Clone)]
struct RunPrompt {
    id: i64,
    title: String,
    category: String,
    text: String,
    system_prompt: Option<String>,
    ideal_answer: Option<String>,
    eval_criteria: Option<String>,
    sort_order: i64,
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn capture_hardware_snapshot() -> HardwareSnapshot {
    let mut system = System::new_all();
    system.refresh_all();
    HardwareSnapshot {
        os_name: System::name(),
        os_version: System::os_version(),
        kernel_version: System::kernel_version(),
        architecture: std::env::consts::ARCH.to_string(),
        cpu_brand: system.cpus().first().map(|cpu| cpu.brand().to_string()),
        logical_cpu_count: system.cpus().len(),
        total_memory_bytes: system.total_memory(),
    }
}

fn require_ollama_available(available: bool) -> Result<(), String> {
    if available {
        Ok(())
    } else {
        Err("Ollama is unavailable. No evaluation run was created.".into())
    }
}

fn trial_key(run_key: &str, item: &evaluation::TrialPlanItem) -> String {
    format!(
        "{run_key}:{}:{}:{}:{}",
        item.kind.as_str(),
        item.prompt_id,
        item.model_id,
        item.repetition_index
    )
}

fn mark_trial_terminal(
    trial_id: i64,
    status: &str,
    error_message: Option<&str>,
    exclusion_reason: Option<&str>,
) {
    if let Ok(conn) = db::get_db().lock() {
        let _ = conn.execute(
            "UPDATE benchmark_trials
             SET status = ?1, error_message = ?2, exclusion_reason = ?3,
                 completed_at = datetime('now')
             WHERE id = ?4",
            rusqlite::params![status, error_message, exclusion_reason, trial_id],
        );
    }
}

fn create_completed_comparisons(run_id: i64, model_ids: &[i64]) -> Result<(), String> {
    if model_ids.len() != 2 {
        return Ok(());
    }
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    let mut stmt = conn
        .prepare(
            "SELECT a.prompt_id, a.repetition_index, a.result_id, b.result_id,
                    a.comparison_position
             FROM benchmark_trials a
             JOIN benchmark_trials b
               ON b.run_id = a.run_id
              AND b.prompt_id = a.prompt_id
              AND b.repetition_index = a.repetition_index
              AND b.trial_kind = a.trial_kind
             WHERE a.run_id = ?1
               AND a.model_id = ?2
               AND b.model_id = ?3
               AND a.trial_kind = 'measured'
               AND a.status = 'completed'
               AND b.status = 'completed'
               AND a.result_id IS NOT NULL
               AND b.result_id IS NOT NULL
             ORDER BY a.prompt_id, a.repetition_index",
        )
        .map_err(|e| format!("comparison preparation error: {e}"))?;
    let rows = stmt
        .query_map(
            rusqlite::params![run_id, model_ids[0], model_ids[1]],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .map_err(|e| format!("comparison preparation error: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("comparison row error: {e}"))?;
    drop(stmt);

    for (prompt_id, repetition_index, result_a_id, result_b_id, position) in rows {
        conn.execute(
            "INSERT OR IGNORE INTO benchmark_comparisons
                (run_id, prompt_id, repetition_index, model_a_id, model_b_id,
                 result_a_id, result_b_id, model_a_position)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                run_id,
                prompt_id,
                repetition_index,
                model_ids[0],
                model_ids[1],
                result_a_id,
                result_b_id,
                position,
            ],
        )
        .map_err(|e| format!("comparison insert error: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn start_benchmark(
    app: tauri::AppHandle,
    state: tauri::State<'_, ActiveBenchmarks>,
    suite_id: i64,
    model_ids: Vec<i64>,
    config: Option<EvaluationConfig>,
) -> Result<i64, String> {
    let config = config.unwrap_or_default();
    config.validate()?;

    let mut distinct_model_ids = model_ids.clone();
    distinct_model_ids.sort_unstable();
    distinct_model_ids.dedup();
    if distinct_model_ids.len() != model_ids.len() {
        return Err("Duplicate model selection is not allowed".into());
    }
    if model_ids.len() < 2 {
        return Err("Select at least two models for a comparison".into());
    }

    let (suite_name, suite_description): (String, Option<String>) = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        conn.query_row(
            "SELECT name, description FROM test_suites WHERE id = ?1",
            [suite_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("suite not found (id={suite_id}): {e}"))?
    };

    let prompts: Vec<RunPrompt> = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, category, text, system_prompt, ideal_answer,
                        eval_criteria, sort_order
                 FROM prompts
                 WHERE suite_id = ?1 ORDER BY sort_order ASC",
            )
            .map_err(|e| format!("prompts query error: {e}"))?;
        let rows = stmt
            .query_map(rusqlite::params![suite_id], |row| {
                Ok(RunPrompt {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    category: row.get(2)?,
                    text: row.get(3)?,
                    system_prompt: row.get(4)?,
                    ideal_answer: row.get(5)?,
                    eval_criteria: row.get(6)?,
                    sort_order: row.get(7)?,
                })
            })
            .map_err(|e| format!("prompts query error: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("prompts row error: {e}"))?;
        rows
    };

    if prompts.is_empty() {
        return Err("Suite has no prompts".into());
    }

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

    require_ollama_available(ollama::health_check().await?)?;
    let ollama_version = ollama::get_version().await?;
    let installed_models = ollama::list_models().await?;
    let mut model_snapshots = Vec::new();
    for (model_id, exact_tag) in &models {
        let live = installed_models
            .iter()
            .find(|candidate| candidate.name == *exact_tag)
            .ok_or_else(|| format!("Model '{exact_tag}' is not currently installed in Ollama"))?;
        if !live.capabilities.is_empty()
            && !live
                .capabilities
                .iter()
                .any(|capability| capability == "completion")
        {
            return Err(format!(
                "Model '{exact_tag}' does not support text completion"
            ));
        }
        let details = live.details.as_ref();
        model_snapshots.push(ModelSnapshot {
            database_id: *model_id,
            exact_tag: exact_tag.clone(),
            digest: live.digest.clone(),
            size_bytes: live.size,
            parameter_size: details.and_then(|value| value.parameter_size.clone()),
            quantization: details.and_then(|value| value.quantization_level.clone()),
            family: details.and_then(|value| value.family.clone()),
            modified_at: live.modified_at.clone(),
            capabilities: live.capabilities.clone(),
        });
    }

    let prompt_snapshots: Vec<PromptSnapshot> = prompts
        .iter()
        .map(|prompt| {
            let content = serde_json::json!({
                "category": prompt.category,
                "title": prompt.title,
                "text": prompt.text,
                "system_prompt": prompt.system_prompt,
                "ideal_answer": prompt.ideal_answer,
                "eval_criteria": prompt.eval_criteria,
                "sort_order": prompt.sort_order,
            });
            Ok(PromptSnapshot {
                id: prompt.id,
                category: prompt.category.clone(),
                title: prompt.title.clone(),
                text: prompt.text.clone(),
                system_prompt: prompt.system_prompt.clone(),
                ideal_answer: prompt.ideal_answer.clone(),
                eval_criteria: prompt.eval_criteria.clone(),
                sort_order: prompt.sort_order,
                digest: digest_json(&content)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let suite_content = serde_json::json!({
        "name": suite_name,
        "description": suite_description,
        "prompts": prompt_snapshots.iter().map(|prompt| &prompt.digest).collect::<Vec<_>>(),
    });
    let suite_snapshot = SuiteSnapshot {
        id: suite_id,
        name: suite_name,
        description: suite_description,
        digest: digest_json(&suite_content)?,
        prompts: prompt_snapshots,
    };

    let created_at_unix_ms = unix_time_ms();
    let random_seed = config
        .seed
        .unwrap_or(created_at_unix_ms ^ ((std::process::id() as u64) << 32))
        & i64::MAX as u64;
    let prompt_ids: Vec<i64> = prompts.iter().map(|prompt| prompt.id).collect();
    let plan = build_trial_plan(&prompt_ids, &model_ids, &config, random_seed)?;
    let measured_trial_count = plan
        .iter()
        .filter(|trial| trial.kind == TrialKind::Measured)
        .count();
    let warmup_trial_count = plan.len() - measured_trial_count;

    let generation_settings_json =
        serde_json::to_string(&config).map_err(|e| format!("settings serialize error: {e}"))?;
    let ollama_endpoint = ollama::get_base_url();
    let hardware_snapshot = capture_hardware_snapshot();
    let (run_id, run_key, manifest_digest, prepared_trials) = {
        let mut conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("begin run transaction: {e}"))?;
        tx.execute(
            "INSERT INTO benchmark_runs
                (suite_id, status, repetitions, warmup_repetitions, random_seed,
                 timeout_seconds, generation_settings_json, outcome_status)
             VALUES (?1, 'running', ?2, ?3, ?4, ?5, ?6, 'running')",
            rusqlite::params![
                suite_id,
                config.repetitions,
                config.warmup_repetitions,
                random_seed as i64,
                config.timeout_seconds as i64,
                generation_settings_json,
            ],
        )
        .map_err(|e| format!("insert run error: {e}"))?;
        let run_id = tx.last_insert_rowid();
        let run_key = format!("eval-{run_id}-{random_seed:016x}");
        let manifest = RunManifest {
            schema_version: evaluation::MANIFEST_SCHEMA_VERSION,
            run_key: run_key.clone(),
            created_at_unix_ms,
            suite: suite_snapshot,
            models: model_snapshots,
            ollama: OllamaSnapshot {
                server_version: ollama_version,
                endpoint: ollama_endpoint,
            },
            hardware: hardware_snapshot,
            generation: config.clone(),
            measured_trial_count,
            warmup_trial_count,
        };
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| format!("manifest serialize error: {e}"))?;
        let manifest_digest = evaluation::sha256_hex(manifest_json.as_bytes());
        tx.execute(
            "UPDATE benchmark_runs SET run_key = ?1, manifest_digest = ?2 WHERE id = ?3",
            rusqlite::params![run_key, manifest_digest, run_id],
        )
        .map_err(|e| format!("run manifest link error: {e}"))?;
        tx.execute(
            "INSERT INTO evaluation_run_manifests
                (run_id, schema_version, manifest_json, manifest_digest)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                run_id,
                evaluation::MANIFEST_SCHEMA_VERSION,
                manifest_json,
                manifest_digest,
            ],
        )
        .map_err(|e| format!("manifest insert error: {e}"))?;

        let mut prepared = Vec::new();
        for item in plan {
            tx.execute(
                "INSERT INTO benchmark_trials
                    (run_id, trial_key, prompt_id, model_id, repetition_index,
                     trial_kind, execution_order, generation_seed, comparison_position)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    run_id,
                    trial_key(&run_key, &item),
                    item.prompt_id,
                    item.model_id,
                    item.repetition_index,
                    item.kind.as_str(),
                    item.execution_order as i64,
                    item.generation_seed as i64,
                    item.comparison_position,
                ],
            )
            .map_err(|e| format!("trial insert error: {e}"))?;
            prepared.push((tx.last_insert_rowid(), item));
        }
        tx.commit()
            .map_err(|e| format!("commit run transaction: {e}"))?;
        (run_id, run_key, manifest_digest, prepared)
    };

    // Create cancellation token
    let cancel_token = CancellationToken::new();
    {
        let mut map = state.0.lock().map_err(|e| format!("state lock: {e}"))?;
        map.insert(run_id, cancel_token.clone());
    }

    let total = prepared_trials.len() as i32;
    let active_map = Arc::clone(&state.0);

    // Shared metrics samples buffer (filled by the metrics task, persisted after main task)
    let metrics_samples: Arc<Mutex<Vec<BenchmarkMetricsPayload>>> =
        Arc::new(Mutex::new(Vec::new()));
    let metrics_samples_metrics = Arc::clone(&metrics_samples);
    let metrics_stop = CancellationToken::new();
    let metrics_stop_task = metrics_stop.clone();
    let metrics_app = app.clone();

    // Spawn hardware metrics sampling task — runs concurrently with benchmark
    let metrics_handle = tokio::spawn(async move {
        let mut sys = System::new_all();
        loop {
            tokio::select! {
                _ = metrics_stop_task.cancelled() => break,
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
        let prompt_map: HashMap<i64, RunPrompt> = prompts
            .into_iter()
            .map(|prompt| (prompt.id, prompt))
            .collect();
        let model_map: HashMap<i64, String> = models.iter().cloned().collect();
        let mut processed = 0i32;
        let mut invalid_trials = 0usize;
        let mut fatal_failure: Option<String> = None;

        for (trial_id, item) in &prepared_trials {
            if cancel_token.is_cancelled() {
                break;
            }
            let Some(prompt) = prompt_map.get(&item.prompt_id) else {
                mark_trial_terminal(*trial_id, "failed", Some("prompt snapshot missing"), None);
                invalid_trials += 1;
                continue;
            };
            let Some(model_name) = model_map.get(&item.model_id) else {
                mark_trial_terminal(*trial_id, "failed", Some("model snapshot missing"), None);
                invalid_trials += 1;
                continue;
            };
            if let Ok(conn) = db::get_db().lock() {
                let _ = conn.execute(
                    "UPDATE benchmark_trials
                     SET status = 'running', started_at = datetime('now') WHERE id = ?1",
                    [trial_id],
                );
            }

            let num_predict = if item.kind == TrialKind::Warmup {
                config.num_predict.map(|value| value.min(64)).or(Some(64))
            } else {
                config.num_predict
            };
            let req = GenerateRequest {
                model: model_name.clone(),
                prompt: prompt.text.clone(),
                system: prompt.system_prompt.clone(),
                num_predict,
                temperature: Some(config.temperature),
                think: Some(config.think),
                seed: Some(item.generation_seed),
            };
            let start = Instant::now();
            let attempt = tokio::time::timeout(
                std::time::Duration::from_secs(config.timeout_seconds),
                async {
                    let rx = ollama::generate_stream(req).await?;
                    benchmark_stream_and_collect(&app, run_id, rx, &cancel_token, start).await
                },
            )
            .await;

            match attempt {
                Err(_) => {
                    mark_trial_terminal(
                        *trial_id,
                        "timeout",
                        Some("trial exceeded configured timeout"),
                        Some("timeout"),
                    );
                    invalid_trials += 1;
                }
                Ok(Err(error)) if error == "cancelled" => {
                    mark_trial_terminal(
                        *trial_id,
                        "cancelled",
                        Some("operator cancelled run"),
                        Some("cancelled"),
                    );
                    break;
                }
                Ok(Err(error)) => {
                    mark_trial_terminal(
                        *trial_id,
                        "failed",
                        Some(&error),
                        Some("generation failed"),
                    );
                    invalid_trials += 1;
                }
                Ok(Ok((output, metrics))) => {
                    let excluded = output.trim().is_empty();
                    let save_result = (|| -> Result<(), String> {
                        let mut conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
                        let tx = conn
                            .transaction()
                            .map_err(|e| format!("result transaction: {e}"))?;
                        tx.execute(
                            "INSERT INTO benchmark_results
                                (run_id, prompt_id, model_id, output, tokens_generated,
                                 time_to_first_token_ms, total_time_ms, tokens_per_second,
                                 trial_id, repetition_index, trial_kind, generation_seed)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                            rusqlite::params![
                                run_id,
                                item.prompt_id,
                                item.model_id,
                                output,
                                metrics.tokens_generated,
                                metrics.time_to_first_token_ms,
                                metrics.total_time_ms,
                                metrics.tokens_per_second,
                                trial_id,
                                item.repetition_index,
                                item.kind.as_str(),
                                item.generation_seed as i64,
                            ],
                        )
                        .map_err(|e| format!("insert result error: {e}"))?;
                        let result_id = tx.last_insert_rowid();
                        tx.execute(
                            "UPDATE benchmark_trials
                             SET status = ?1, result_id = ?2, exclusion_reason = ?3,
                                 completed_at = datetime('now')
                             WHERE id = ?4",
                            rusqlite::params![
                                if excluded { "excluded" } else { "completed" },
                                result_id,
                                if excluded { Some("empty output") } else { None },
                                trial_id,
                            ],
                        )
                        .map_err(|e| format!("update trial result error: {e}"))?;
                        tx.commit().map_err(|e| format!("commit result: {e}"))?;
                        Ok(())
                    })();
                    if let Err(error) = save_result {
                        mark_trial_terminal(
                            *trial_id,
                            "failed",
                            Some(&error),
                            Some("persistence failed"),
                        );
                        fatal_failure = Some(error);
                        break;
                    }
                    if excluded {
                        invalid_trials += 1;
                    }
                }
            }

            processed += 1;
            let _ = app.emit(
                "benchmark:progress",
                BenchmarkProgressPayload {
                    run_id,
                    completed: processed,
                    total,
                    current_model: model_name.clone(),
                    current_prompt: if item.kind == TrialKind::Warmup {
                        format!("Warm-up · {}", prompt.title)
                    } else {
                        format!("{} · trial {}", prompt.title, item.repetition_index + 1)
                    },
                },
            );
        }

        let was_cancelled = cancel_token.is_cancelled();
        if was_cancelled {
            if let Ok(conn) = db::get_db().lock() {
                let _ = conn.execute(
                    "UPDATE benchmark_trials
                     SET status = 'cancelled', exclusion_reason = 'run cancelled',
                         completed_at = datetime('now')
                     WHERE run_id = ?1 AND status IN ('pending', 'running')",
                    [run_id],
                );
                let _ = conn.execute(
                    "UPDATE benchmark_runs
                     SET status = 'cancelled', outcome_status = 'cancelled',
                         failure_reason = 'operator cancelled run', comparable = 0,
                         comparability_notes = 'Run was cancelled before all measured trials completed',
                         completed_at = datetime('now')
                     WHERE id = ?1",
                    [run_id],
                );
            }
            let _ = app.emit(
                "benchmark:error",
                BenchmarkErrorPayload {
                    run_id,
                    message: "Benchmark cancelled".into(),
                },
            );
        } else if let Some(error) = fatal_failure {
            if let Ok(conn) = db::get_db().lock() {
                let _ = conn.execute(
                    "UPDATE benchmark_runs
                     SET status = 'cancelled', outcome_status = 'failed', failure_reason = ?1,
                         comparable = 0, comparability_notes = 'Persistence failure invalidated the run',
                         completed_at = datetime('now')
                     WHERE id = ?2",
                    rusqlite::params![error, run_id],
                );
            }
            let _ = app.emit(
                "benchmark:error",
                BenchmarkErrorPayload {
                    run_id,
                    message: error,
                },
            );
        } else {
            if let Err(error) = create_completed_comparisons(run_id, &model_ids) {
                invalid_trials += 1;
                eprintln!("comparison preparation failed for run {run_id}: {error}");
            }
            let completed_measured = if let Ok(conn) = db::get_db().lock() {
                conn.query_row(
                    "SELECT COUNT(*) FROM benchmark_trials
                     WHERE run_id = ?1 AND trial_kind = 'measured' AND status = 'completed'",
                    [run_id],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or(0) as usize
            } else {
                0
            };
            let comparable = completed_measured == measured_trial_count && invalid_trials == 0;
            let outcome = if comparable {
                "completed"
            } else {
                "completed_with_failures"
            };
            let notes = if comparable {
                format!(
                    "All {measured_trial_count} measured trials completed under one immutable manifest"
                )
            } else {
                format!(
                    "Only {completed_measured} of {measured_trial_count} measured trials were valid; recommendation withheld"
                )
            };
            if let Ok(conn) = db::get_db().lock() {
                let _ = conn.execute(
                    "UPDATE benchmark_runs
                     SET status = 'completed', outcome_status = ?1, comparable = ?2,
                         comparability_notes = ?3, completed_at = datetime('now')
                     WHERE id = ?4",
                    rusqlite::params![outcome, i64::from(comparable), notes, run_id],
                );
            }
            let _ = app.emit("benchmark:complete", BenchmarkCompletePayload { run_id });
        }

        metrics_stop.cancel();
        let _ = metrics_handle.await;
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
        let _ = (run_key, manifest_digest);
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
                     ORDER BY bs.created_at DESC LIMIT 1),
                    br.repetition_index, bt.trial_key, br.generation_seed,
                    COALESCE(bt.status, 'legacy')
             FROM benchmark_results br
             JOIN models m ON br.model_id = m.id
             JOIN prompts p ON br.prompt_id = p.id
             LEFT JOIN benchmark_trials bt ON bt.id = br.trial_id
             WHERE br.run_id = ?1 AND br.trial_kind != 'warmup'
             ORDER BY p.category, p.sort_order, m.display_name, br.repetition_index",
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
                repetition_index: row.get(16)?,
                trial_key: row.get(17)?,
                generation_seed: row.get(18)?,
                trial_status: row.get(19)?,
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
                    br.started_at, br.completed_at, br.outcome_status, br.repetitions,
                    (SELECT COUNT(*) FROM benchmark_trials t
                     WHERE t.run_id = br.id AND t.status IN ('failed', 'timeout')),
                    (SELECT COUNT(*) FROM benchmark_trials t
                     WHERE t.run_id = br.id AND t.status = 'excluded'),
                    br.comparable, br.comparability_notes, br.manifest_digest
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
                outcome_status: row.get(10)?,
                repetitions: row.get(11)?,
                failed_count: row.get(12)?,
                excluded_count: row.get(13)?,
                comparable: row.get::<_, i64>(14)? != 0,
                comparability_notes: row.get(15)?,
                manifest_digest: row.get(16)?,
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
    let (judge_model_name, judge_digest): (String, Option<String>) = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        conn.query_row(
            "SELECT name, digest FROM models WHERE id = ?1",
            rusqlite::params![judge_model_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("judge model not found (id={judge_model_id}): {e}"))?
    };
    let judge_manifest_json = serde_json::to_string(&serde_json::json!({
        "exact_tag": judge_model_name,
        "digest": judge_digest,
        "ollama_version": ollama::get_version().await?,
        "system_prompt": "You are a strict but fair evaluator. Output only valid JSON.",
        "temperature": 0.1,
        "num_predict": 256,
        "judge_prompt_digest": evaluation::sha256_hex(build_judge_prompt("{prompt}", Some("{criteria}"), "{output}").as_bytes()),
    }))
    .map_err(|e| format!("judge manifest serialize error: {e}"))?;

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
                 JOIN benchmark_trials bt ON bt.id = br.trial_id
                 WHERE br.run_id = ?1
                   AND bt.trial_kind = 'measured'
                   AND bt.status = 'completed'
                   AND br.model_id != ?2
                   AND NOT EXISTS (
                       SELECT 1 FROM benchmark_scores bs
                       WHERE bs.result_id = br.id
                         AND bs.scoring_method = 'auto_judge'
                         AND bs.judge_model_id = ?2
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
                think: Some(false),
                seed: None,
            };

            let attempt_id = {
                let conn = match db::get_db().lock() {
                    Ok(conn) => conn,
                    Err(error) => {
                        eprintln!("auto_judge: db lock error for result {result_id}: {error}");
                        continue;
                    }
                };
                if let Err(error) = conn.execute(
                    "INSERT INTO benchmark_judge_attempts
                        (run_id, result_id, judge_model_id, judge_manifest_json)
                     VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![run_id, result_id, judge_model_id, judge_manifest_json],
                ) {
                    eprintln!("auto_judge: attempt insert error for result {result_id}: {error}");
                    continue;
                }
                conn.last_insert_rowid()
            };

            let rx = match ollama::generate_stream(req).await {
                Ok(rx) => rx,
                Err(e) => {
                    if let Ok(conn) = db::get_db().lock() {
                        let _ = conn.execute(
                            "UPDATE benchmark_judge_attempts
                             SET status = 'failed', error_message = ?1,
                                 completed_at = datetime('now') WHERE id = ?2",
                            rusqlite::params![e, attempt_id],
                        );
                    }
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
                if let Ok(conn) = db::get_db().lock() {
                    let _ = conn.execute(
                        "UPDATE benchmark_judge_attempts
                         SET status = 'cancelled', raw_output = ?1,
                             completed_at = datetime('now') WHERE id = ?2",
                        rusqlite::params![buffer, attempt_id],
                    );
                }
                break;
            }

            match parse_judge_response(&buffer) {
                Some((score, reasoning)) => {
                    let save_result = (|| -> Result<(), String> {
                        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
                        conn.execute(
                            "INSERT INTO benchmark_scores
                                (result_id, score, scoring_method, judge_model_id, notes,
                                 judge_manifest_json, raw_judge_output, status)
                             VALUES (?1, ?2, 'auto_judge', ?3, ?4, ?5, ?6, 'completed')",
                            rusqlite::params![
                                result_id,
                                score,
                                judge_model_id,
                                if reasoning.is_empty() {
                                    None
                                } else {
                                    Some(reasoning)
                                },
                                judge_manifest_json,
                                buffer,
                            ],
                        )
                        .map_err(|e| format!("insert score error: {e}"))?;
                        Ok(())
                    })();

                    if let Err(e) = save_result {
                        if let Ok(conn) = db::get_db().lock() {
                            let _ = conn.execute(
                                "UPDATE benchmark_judge_attempts
                                 SET status = 'failed', raw_output = ?1, error_message = ?2,
                                     completed_at = datetime('now') WHERE id = ?3",
                                rusqlite::params![buffer, e, attempt_id],
                            );
                        }
                        eprintln!("auto_judge: save error: {e}");
                    } else {
                        if let Ok(conn) = db::get_db().lock() {
                            let _ = conn.execute(
                                "UPDATE benchmark_judge_attempts
                                 SET status = 'completed', raw_output = ?1,
                                     completed_at = datetime('now') WHERE id = ?2",
                                rusqlite::params![buffer, attempt_id],
                            );
                        }
                        scores_added += 1;
                    }
                }
                None => {
                    if let Ok(conn) = db::get_db().lock() {
                        let _ = conn.execute(
                            "UPDATE benchmark_judge_attempts
                             SET status = 'invalid', raw_output = ?1,
                                 error_message = 'judge response could not be parsed',
                                 completed_at = datetime('now') WHERE id = ?2",
                            rusqlite::params![buffer, attempt_id],
                        );
                    }
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
    if let Ok(conn) = db::get_db().lock() {
        let _ = conn.execute(
            "UPDATE benchmark_judge_attempts
             SET status = 'cancelled', completed_at = datetime('now')
             WHERE run_id = ?1 AND status = 'running'",
            [run_id],
        );
    }
    Ok(())
}

fn recommendation_for_scored_pair(
    category: String,
    mut selected: Vec<&CapabilityEvidence>,
) -> CapabilityRecommendation {
    selected.sort_by(|a, b| {
        b.confidence
            .mean
            .partial_cmp(&a.confidence.mean)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if selected.len() < 2 {
        return CapabilityRecommendation {
            category,
            recommended_model: None,
            confidence: "insufficient evidence".into(),
            reason: "At least two models need scores from the same judge method".into(),
        };
    }
    if !selected[0].confidence.sufficient_sample || !selected[1].confidence.sufficient_sample {
        return CapabilityRecommendation {
            category,
            recommended_model: None,
            confidence: "insufficient sample".into(),
            reason: format!(
                "{} and {} do not both meet the repeated-sample threshold",
                selected[0].model_name, selected[1].model_name
            ),
        };
    }
    let top = selected[0];
    let runner_up = selected[1];
    let separated = match (top.confidence.lower_95, runner_up.confidence.upper_95) {
        (Some(top_low), Some(other_high)) => top_low > other_high,
        _ => false,
    };
    if separated {
        CapabilityRecommendation {
            category,
            recommended_model: Some(top.model_name.clone()),
            confidence: "directional".into(),
            reason: format!(
                "{} leads {} using {} and the approximate 95% intervals do not overlap",
                top.model_name, runner_up.model_name, top.scoring_method
            ),
        }
    } else {
        CapabilityRecommendation {
            category,
            recommended_model: None,
            confidence: "inconclusive".into(),
            reason: format!(
                "{} and {} have overlapping uncertainty intervals",
                top.model_name, runner_up.model_name
            ),
        }
    }
}

#[tauri::command]
pub async fn get_run_evidence(run_id: i64) -> Result<RunEvidence, String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    let (outcome_status, manifest_digest, comparable, comparability_notes): (
        String,
        Option<String>,
        i64,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT outcome_status, manifest_digest, comparable, comparability_notes
             FROM benchmark_runs WHERE id = ?1",
            [run_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|e| format!("run not found (id={run_id}): {e}"))?;

    let counts: (i64, i64, i64, i64, i64, i64) = conn
        .query_row(
            "SELECT
                SUM(CASE WHEN trial_kind = 'measured' THEN 1 ELSE 0 END),
                SUM(CASE WHEN trial_kind = 'measured' AND status = 'completed' THEN 1 ELSE 0 END),
                SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END),
                SUM(CASE WHEN status = 'excluded' THEN 1 ELSE 0 END),
                SUM(CASE WHEN status = 'cancelled' THEN 1 ELSE 0 END),
                SUM(CASE WHEN status = 'timeout' THEN 1 ELSE 0 END)
             FROM benchmark_trials WHERE run_id = ?1",
            [run_id],
            |row| {
                Ok((
                    row.get::<_, Option<i64>>(0)?.unwrap_or(0),
                    row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                    row.get::<_, Option<i64>>(2)?.unwrap_or(0),
                    row.get::<_, Option<i64>>(3)?.unwrap_or(0),
                    row.get::<_, Option<i64>>(4)?.unwrap_or(0),
                    row.get::<_, Option<i64>>(5)?.unwrap_or(0),
                ))
            },
        )
        .map_err(|e| format!("trial count error: {e}"))?;

    type ScoreRow = (String, i64, String, String, f64);
    let mut score_stmt = conn
        .prepare(
            "SELECT p.category, br.model_id, m.display_name,
                    CASE bs.scoring_method
                        WHEN 'auto_judge' THEN 'auto_judge:' || jm.name
                        ELSE 'human_score'
                    END,
                    CASE bs.scoring_method WHEN 'manual' THEN CAST(bs.score AS REAL) * 2.0
                        ELSE CAST(bs.score AS REAL) END
             FROM benchmark_results br
             JOIN benchmark_trials bt ON bt.id = br.trial_id
             JOIN prompts p ON p.id = br.prompt_id
             JOIN models m ON m.id = br.model_id
             JOIN benchmark_scores bs ON bs.result_id = br.id AND bs.status = 'completed'
                 AND bs.scoring_method IN ('auto_judge', 'manual')
             LEFT JOIN models jm ON jm.id = bs.judge_model_id
             WHERE br.run_id = ?1 AND bt.trial_kind = 'measured' AND bt.status = 'completed'",
        )
        .map_err(|e| format!("evidence query error: {e}"))?;
    let score_rows: Vec<ScoreRow> = score_stmt
        .query_map([run_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .map_err(|e| format!("evidence query error: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("evidence row error: {e}"))?;

    let mut grouped: HashMap<(String, i64, String, String), Vec<f64>> = HashMap::new();
    for (category, model_id, model_name, scoring_method, score) in score_rows {
        grouped
            .entry((category, model_id, model_name, scoring_method))
            .or_default()
            .push(score);
    }
    let mut capability_evidence: Vec<CapabilityEvidence> = grouped
        .into_iter()
        .map(
            |((category, model_id, model_name, scoring_method), values)| CapabilityEvidence {
                category,
                model_id,
                model_name,
                scoring_method,
                confidence: evaluation::mean_confidence_95(&values),
            },
        )
        .collect();
    capability_evidence.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then(a.scoring_method.cmp(&b.scoring_method))
            .then(a.model_name.cmp(&b.model_name))
    });

    let mut recommendation_groups: HashMap<String, Vec<&CapabilityEvidence>> = HashMap::new();
    for evidence in &capability_evidence {
        recommendation_groups
            .entry(evidence.category.clone())
            .or_default()
            .push(evidence);
    }
    let mut recommendations = Vec::new();
    for (category, all_evidence) in recommendation_groups {
        let human: Vec<_> = all_evidence
            .iter()
            .copied()
            .filter(|entry| entry.scoring_method == "human_score")
            .collect();
        let mut selected = human;
        selected.sort_by(|a, b| {
            b.confidence
                .mean
                .partial_cmp(&a.confidence.mean)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let recommendation = if comparable == 0 {
            CapabilityRecommendation {
                category,
                recommended_model: None,
                confidence: "withheld".into(),
                reason: "Run is incomplete or incomparable; no recommendation is allowed".into(),
            }
        } else if selected.len() >= 2 {
            recommendation_for_scored_pair(category, selected)
        } else {
            let mut by_judge: HashMap<&str, Vec<&CapabilityEvidence>> = HashMap::new();
            for entry in all_evidence
                .iter()
                .copied()
                .filter(|entry| entry.scoring_method.starts_with("auto_judge:"))
            {
                by_judge
                    .entry(entry.scoring_method.as_str())
                    .or_default()
                    .push(entry);
            }
            let judge_count = by_judge.len();
            let mut judge_winners = Vec::new();
            for entries in by_judge.into_values() {
                let candidate = recommendation_for_scored_pair(category.clone(), entries);
                judge_winners.push(candidate.recommended_model);
            }
            let consensus = judge_winners.first().cloned().flatten();
            let unanimous = consensus.is_some()
                && judge_winners
                    .iter()
                    .all(|winner| winner.as_ref() == consensus.as_ref());
            if judge_count == 0 {
                CapabilityRecommendation {
                    category,
                    recommended_model: None,
                    confidence: "insufficient evidence".into(),
                    reason: "At least two models need scores from the same judge method".into(),
                }
            } else if unanimous {
                CapabilityRecommendation {
                    category,
                    recommended_model: consensus.clone(),
                    confidence: "directional".into(),
                    reason: format!(
                        "All {judge_count} local auto-judge(s) independently support {}",
                        consensus.unwrap_or_default()
                    ),
                }
            } else if judge_count > 1 {
                CapabilityRecommendation {
                    category,
                    recommended_model: None,
                    confidence: "judge-sensitive".into(),
                    reason: format!(
                        "The {judge_count} local auto-judges do not independently support the same winner"
                    ),
                }
            } else {
                CapabilityRecommendation {
                    category,
                    recommended_model: None,
                    confidence: "inconclusive".into(),
                    reason: "The local auto-judge found overlapping uncertainty intervals".into(),
                }
            }
        };
        recommendations.push(recommendation);
    }
    recommendations.sort_by(|a, b| a.category.cmp(&b.category));

    let human_outcomes: Vec<SideOutcome> = {
        let mut stmt = conn
            .prepare(
                "SELECT human_outcome FROM benchmark_comparisons
                 WHERE run_id = ?1 AND human_outcome IN ('left', 'right', 'tie')",
            )
            .map_err(|e| format!("human vote query error: {e}"))?;
        let values = stmt
            .query_map([run_id], |row| row.get::<_, String>(0))
            .map_err(|e| format!("human vote query error: {e}"))?
            .filter_map(|value| match value.ok()?.as_str() {
                "left" => Some(SideOutcome::Left),
                "right" => Some(SideOutcome::Right),
                "tie" => Some(SideOutcome::Tie),
                _ => None,
            })
            .collect();
        values
    };
    let position_bias = evaluation::detect_position_bias(&human_outcomes);

    let disagreement_pairs: Vec<(SideOutcome, SideOutcome)> = {
        let mut stmt = conn
            .prepare(
                "SELECT bc.human_outcome, bc.model_a_position,
                    (SELECT bs.score FROM benchmark_scores bs
                     WHERE bs.result_id = bc.result_a_id AND bs.scoring_method = 'auto_judge'
                       AND bs.status = 'completed' ORDER BY bs.created_at DESC LIMIT 1),
                    (SELECT bs.score FROM benchmark_scores bs
                     WHERE bs.result_id = bc.result_b_id AND bs.scoring_method = 'auto_judge'
                       AND bs.status = 'completed' ORDER BY bs.created_at DESC LIMIT 1)
                 FROM benchmark_comparisons bc
                 WHERE bc.run_id = ?1 AND bc.human_outcome IN ('left', 'right', 'tie')",
            )
            .map_err(|e| format!("disagreement query error: {e}"))?;
        let values = stmt
            .query_map([run_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                ))
            })
            .map_err(|e| format!("disagreement query error: {e}"))?
            .filter_map(|row| {
                let (human, model_a_position, score_a, score_b) = row.ok()?;
                let human = match human.as_str() {
                    "left" => SideOutcome::Left,
                    "right" => SideOutcome::Right,
                    "tie" => SideOutcome::Tie,
                    _ => return None,
                };
                let (score_a, score_b) = (score_a?, score_b?);
                let auto = if score_a == score_b {
                    SideOutcome::Tie
                } else {
                    let a_wins = score_a > score_b;
                    let winner_is_left = (a_wins && model_a_position == "left")
                        || (!a_wins && model_a_position == "right");
                    if winner_is_left {
                        SideOutcome::Left
                    } else {
                        SideOutcome::Right
                    }
                };
                Some((human, auto))
            })
            .collect();
        values
    };
    let judge_disagreement = evaluation::judge_disagreement(&disagreement_pairs);

    let mut judge_provenance: Vec<String> = {
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT m.name, m.digest
                 FROM benchmark_judge_attempts a JOIN models m ON m.id = a.judge_model_id
                 WHERE a.run_id = ?1 AND a.status = 'completed'",
            )
            .map_err(|e| format!("judge provenance query error: {e}"))?;
        let values = stmt
            .query_map([run_id], |row| {
                let name: String = row.get(0)?;
                let digest: Option<String> = row.get(1)?;
                Ok(match digest {
                    Some(value) => format!("auto-judge {name}@{}", &value[..value.len().min(12)]),
                    None => format!("auto-judge {name} (digest unavailable)"),
                })
            })
            .map_err(|e| format!("judge provenance query error: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("judge provenance row error: {e}"))?;
        values
    };
    if capability_evidence
        .iter()
        .any(|entry| entry.scoring_method == "human_score")
    {
        judge_provenance.push("human scalar scores".into());
    }
    if !human_outcomes.is_empty() {
        judge_provenance.push("human blind comparisons".into());
    }

    let left_count = conn
        .query_row(
            "SELECT COUNT(*) FROM benchmark_comparisons
             WHERE run_id = ?1 AND model_a_position = 'left'",
            [run_id],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0);
    let right_count = conn
        .query_row(
            "SELECT COUNT(*) FROM benchmark_comparisons
             WHERE run_id = ?1 AND model_a_position = 'right'",
            [run_id],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0);
    let elo_eligible = evaluation::is_elo_eligible(EloEligibility {
        run_complete: outcome_status == "completed",
        all_trials_valid: counts.0 == counts.1,
        comparable: comparable != 0,
        positions_balanced: (left_count - right_count).abs() <= 1,
        human_judged: !human_outcomes.is_empty(),
        sample_size: human_outcomes.len(),
    });

    Ok(RunEvidence {
        run_id,
        outcome_status,
        manifest_digest,
        comparable: comparable != 0,
        comparability_notes,
        planned_measured_trials: counts.0,
        completed_measured_trials: counts.1,
        failed_trials: counts.2,
        excluded_trials: counts.3,
        cancelled_trials: counts.4,
        timeout_trials: counts.5,
        hardware_dependent: true,
        capability_evidence,
        recommendations,
        position_bias,
        judge_disagreement,
        judge_provenance,
        elo_eligible,
        elo_updated: false,
    })
}

#[tauri::command]
pub async fn get_run_comparability(run_a: i64, run_b: i64) -> Result<RunComparability, String> {
    let (valid_a, a, _, valid_b, b, _) = load_comparison_manifests(run_a, run_b)?;
    compare_manifests(valid_a, &a, valid_b, &b)
}

fn load_comparison_manifests(
    run_a: i64,
    run_b: i64,
) -> Result<(bool, RunManifest, String, bool, RunManifest, String), String> {
    let load = |run_id: i64| -> Result<(bool, RunManifest, String), String> {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        let (comparable, manifest_json, manifest_digest): (i64, String, String) = conn
            .query_row(
                "SELECT br.comparable, erm.manifest_json, erm.manifest_digest
                 FROM benchmark_runs br
                 JOIN evaluation_run_manifests erm ON erm.run_id = br.id
                 WHERE br.id = ?1",
                [run_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|e| format!("run {run_id} has no reproducibility manifest: {e}"))?;
        let manifest = serde_json::from_str(&manifest_json)
            .map_err(|e| format!("run {run_id} manifest is invalid: {e}"))?;
        Ok((comparable != 0, manifest, manifest_digest))
    };
    let (valid_a, a, digest_a) = load(run_a)?;
    let (valid_b, b, digest_b) = load(run_b)?;
    Ok((valid_a, a, digest_a, valid_b, b, digest_b))
}

fn compare_manifests(
    valid_a: bool,
    a: &RunManifest,
    valid_b: bool,
    b: &RunManifest,
) -> Result<RunComparability, String> {
    let mut reasons = Vec::new();
    let mut identity_differs = false;
    let mut runtime_differs = false;
    let mut hardware_differs = false;
    if !valid_a || !valid_b {
        reasons.push("One or both runs contain invalid or incomplete trials".into());
        identity_differs = true;
    }
    if a.suite.digest != b.suite.digest {
        reasons.push("Suite or prompt digest differs".into());
        identity_differs = true;
    }
    if a.ollama.server_version != b.ollama.server_version {
        reasons.push("Ollama server version differs".into());
        runtime_differs = true;
    }
    if digest_json(&a.hardware)? != digest_json(&b.hardware)? {
        reasons.push("Hardware/runtime snapshot differs".into());
        hardware_differs = true;
    }
    let model_keys = |manifest: &RunManifest| {
        manifest
            .models
            .iter()
            .map(|model| (model.exact_tag.clone(), model.digest.clone()))
            .collect::<Vec<_>>()
    };
    if model_keys(&a) != model_keys(&b) {
        reasons.push("Exact model tags or digests differ".into());
        identity_differs = true;
    }
    let settings_equal = a.generation.repetitions == b.generation.repetitions
        && a.generation.warmup_repetitions == b.generation.warmup_repetitions
        && a.generation.timeout_seconds == b.generation.timeout_seconds
        && a.generation.temperature == b.generation.temperature
        && a.generation.num_predict == b.generation.num_predict
        && a.generation.think == b.generation.think;
    if !settings_equal {
        reasons.push("Generation or trial settings differ".into());
        identity_differs = true;
    }
    let classification = if identity_differs {
        "incomparable"
    } else if runtime_differs {
        "runtime_variant"
    } else if hardware_differs {
        "hardware_variant"
    } else {
        "exact_reproduction"
    };
    Ok(RunComparability {
        comparable: classification == "exact_reproduction",
        classification: classification.into(),
        quality_comparable: !identity_differs && !runtime_differs,
        performance_comparable: !identity_differs && !runtime_differs && !hardware_differs,
        reasons,
    })
}

#[tauri::command]
pub async fn export_reproduction_receipt(run_a: i64, run_b: i64) -> Result<String, String> {
    let (valid_a, manifest_a, digest_a, valid_b, manifest_b, digest_b) =
        load_comparison_manifests(run_a, run_b)?;
    let comparability = compare_manifests(valid_a, &manifest_a, valid_b, &manifest_b)?;
    serde_json::to_string_pretty(&ReproductionReceipt {
        version: 1,
        created_at_unix_ms: unix_time_ms(),
        run_a,
        run_b,
        run_a_manifest_digest: digest_a,
        run_b_manifest_digest: digest_b,
        comparability,
        run_a_manifest: manifest_a,
        run_b_manifest: manifest_b,
    })
    .map_err(|e| format!("reproduction receipt serialize error: {e}"))
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
             JOIN benchmark_runs run ON run.id = br.run_id
             JOIN benchmark_trials bt ON bt.id = br.trial_id
             JOIN models m ON br.model_id = m.id
             WHERE run.comparable = 1
               AND bt.trial_kind = 'measured'
               AND bt.status = 'completed'
               AND bs.scoring_method = 'auto_judge'
               AND bs.status = 'completed'
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
             JOIN benchmark_runs run ON run.id = br.run_id
             JOIN benchmark_trials bt ON bt.id = br.trial_id
             JOIN prompts p ON br.prompt_id = p.id
             WHERE run.comparable = 1
               AND bt.trial_kind = 'measured'
               AND bt.status = 'completed'
               AND bs.scoring_method = 'auto_judge'
               AND bs.status = 'completed'
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

#[tauri::command]
pub async fn start_blind_comparison(
    state: tauri::State<'_, ActiveBlindComparisons>,
    run_id: i64,
    one_per_prompt: Option<bool>,
) -> Result<BlindComparison, String> {
    type BlindRow = (
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
        String,
        String,
        String,
        String,
        String,
    );
    let (rows, sampling_seed): (Vec<BlindRow>, i64) = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        let sampling_seed = conn
            .query_row(
                "SELECT COALESCE(random_seed, id) FROM benchmark_runs WHERE id = ?1",
                rusqlite::params![run_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("run seed query error: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT bc.id, bc.prompt_id, bc.repetition_index,
                        bc.model_a_id, bc.model_b_id, bc.result_a_id, bc.result_b_id,
                        bc.model_a_position,
                        a.output, b.output, p.title, p.category
                 FROM benchmark_comparisons bc
                 JOIN benchmark_results a ON a.id = bc.result_a_id
                 JOIN benchmark_results b ON b.id = bc.result_b_id
                 JOIN prompts p ON p.id = bc.prompt_id
                 WHERE bc.run_id = ?1 AND bc.human_outcome IS NULL
                 ORDER BY bc.prompt_id, bc.repetition_index",
            )
            .map_err(|e| format!("query error: {e}"))?;
        let values = stmt
            .query_map(rusqlite::params![run_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, String>(11)?,
                ))
            })
            .map_err(|e| format!("query error: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("row error: {e}"))?;
        (values, sampling_seed)
    };
    let rows = if one_per_prompt.unwrap_or(false) {
        select_one_comparison_per_prompt(rows, sampling_seed, |row| row.1)
    } else {
        rows
    };
    let first = rows
        .first()
        .ok_or_else(|| "No unjudged, valid comparison trials are available".to_string())?;
    let model_a_id = first.3;
    let model_b_id = first.4;
    let mut prompt_assignments: HashMap<i64, (i64, i64, bool)> = HashMap::new();
    let mut pairs: Vec<BlindPair> = Vec::new();
    for (
        comparison_id,
        prompt_id,
        repetition_index,
        _,
        _,
        result_a_id,
        result_b_id,
        model_a_position,
        output_a,
        output_b,
        title,
        category,
    ) in rows
    {
        let a_is_left = model_a_position == "left";
        let (left_result_id, left_output, right_result_id, right_output) = if a_is_left {
            (result_a_id, output_a, result_b_id, output_b)
        } else {
            (result_b_id, output_b, result_a_id, output_a)
        };
        prompt_assignments.insert(comparison_id, (result_a_id, result_b_id, a_is_left));
        pairs.push(BlindPair {
            comparison_id,
            prompt_id,
            repetition_index,
            prompt_title: title,
            prompt_category: category,
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

fn select_one_comparison_per_prompt<T, F>(rows: Vec<T>, seed: i64, prompt_id_for: F) -> Vec<T>
where
    F: Fn(&T) -> i64,
{
    let mut selected = Vec::new();
    let mut current_prompt = None;
    let mut prompt_rows: Vec<T> = Vec::new();

    let flush = |prompt_rows: &mut Vec<T>, selected: &mut Vec<T>| {
        if prompt_rows.is_empty() {
            return;
        }
        let prompt_id = prompt_id_for(&prompt_rows[0]);
        let index = (seed.wrapping_mul(31).wrapping_add(prompt_id) as u64
            % prompt_rows.len() as u64) as usize;
        selected.push(prompt_rows.swap_remove(index));
        prompt_rows.clear();
    };

    for row in rows {
        let prompt_id = prompt_id_for(&row);
        if current_prompt.is_some_and(|current| current != prompt_id) {
            flush(&mut prompt_rows, &mut selected);
        }
        current_prompt = Some(prompt_id);
        prompt_rows.push(row);
    }
    flush(&mut prompt_rows, &mut selected);
    selected
}

#[tauri::command]
pub async fn submit_blind_pick(
    state: tauri::State<'_, ActiveBlindComparisons>,
    run_id: i64,
    comparison_id: i64,
    winner: String,
) -> Result<(), String> {
    let (result_a_id, result_b_id, a_is_left, model_a_id, model_b_id) = {
        let map = state.0.lock().map_err(|e| format!("state lock: {e}"))?;
        let st = map
            .get(&run_id)
            .ok_or_else(|| format!("no active blind comparison for run {run_id}"))?;
        let assignment = st
            .prompt_assignments
            .get(&comparison_id)
            .ok_or_else(|| format!("comparison {comparison_id} not found in blind comparison"))?;
        (
            assignment.0,
            assignment.1,
            assignment.2,
            st.model_a_id,
            st.model_b_id,
        )
    };

    let winner_model_id = match winner.as_str() {
        "left" if a_is_left => Some(model_a_id),
        "left" => Some(model_b_id),
        "right" if a_is_left => Some(model_b_id),
        "right" => Some(model_a_id),
        "tie" => None,
        other => {
            return Err(format!(
                "invalid winner value: '{other}' (expected left|right|tie)"
            ))
        }
    };

    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    conn.execute(
        "UPDATE benchmark_comparisons
         SET human_outcome = ?1, human_winner_model_id = ?2,
             human_judged_at = datetime('now')
         WHERE id = ?3 AND run_id = ?4 AND human_outcome IS NULL",
        rusqlite::params![winner, winner_model_id, comparison_id, run_id],
    )
    .map_err(|e| format!("save blind judgment error: {e}"))?;

    let _ = (result_a_id, result_b_id);

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

    let judgments: Vec<(i64, String, Option<i64>, String)> = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT bc.prompt_id, p.title, bc.human_winner_model_id, bc.human_outcome
                 FROM benchmark_comparisons bc
                 JOIN prompts p ON p.id = bc.prompt_id
                 WHERE bc.run_id = ?1
                   AND bc.human_outcome IN ('left', 'right', 'tie')
                 ORDER BY bc.prompt_id, bc.repetition_index",
            )
            .map_err(|e| format!("query error: {e}"))?;
        let values = stmt
            .query_map(rusqlite::params![run_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(|e| format!("query error: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("row error: {e}"))?;
        values
    };

    let mut model_a_wins: i64 = 0;
    let mut model_b_wins: i64 = 0;
    let mut ties: i64 = 0;
    let mut entries: Vec<BlindRevealEntry> = Vec::new();

    for (prompt_id, prompt_title, winner_model_id, outcome) in judgments {
        let winner_str = if winner_model_id == Some(model_a_id) {
            model_a_wins += 1;
            model_a_name.clone()
        } else if winner_model_id == Some(model_b_id) {
            model_b_wins += 1;
            model_b_name.clone()
        } else {
            ties += 1;
            "tie".to_string()
        };

        if outcome == "tie" && winner_model_id.is_some() {
            return Err("invalid persisted blind judgment: tie has a winner".into());
        }

        entries.push(BlindRevealEntry {
            prompt_id,
            prompt_title,
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
pub async fn export_evaluation_bundle(run_id: i64) -> Result<String, String> {
    let evidence = get_run_evidence(run_id).await?;
    let results = get_benchmark_results(run_id).await?;
    let (manifest, trials, judge_attempts, comparisons) = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        let manifest_json: String = conn
            .query_row(
                "SELECT manifest_json FROM evaluation_run_manifests WHERE run_id = ?1",
                [run_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("run {run_id} has no reproducibility manifest: {e}"))?;
        let manifest = serde_json::from_str(&manifest_json)
            .map_err(|e| format!("stored manifest is invalid: {e}"))?;

        let mut trial_stmt = conn
            .prepare(
                "SELECT trial_key, prompt_id, model_id, repetition_index, trial_kind,
                        execution_order, generation_seed, comparison_position, status,
                        result_id, error_message, exclusion_reason, started_at, completed_at
                 FROM benchmark_trials WHERE run_id = ?1 ORDER BY execution_order",
            )
            .map_err(|e| format!("trial export query error: {e}"))?;
        let trials = trial_stmt
            .query_map([run_id], |row| {
                Ok(TrialExportData {
                    trial_key: row.get(0)?,
                    prompt_id: row.get(1)?,
                    model_id: row.get(2)?,
                    repetition_index: row.get(3)?,
                    trial_kind: row.get(4)?,
                    execution_order: row.get(5)?,
                    generation_seed: row.get(6)?,
                    comparison_position: row.get(7)?,
                    status: row.get(8)?,
                    result_id: row.get(9)?,
                    error_message: row.get(10)?,
                    exclusion_reason: row.get(11)?,
                    started_at: row.get(12)?,
                    completed_at: row.get(13)?,
                })
            })
            .map_err(|e| format!("trial export query error: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("trial export row error: {e}"))?;

        let mut judge_stmt = conn
            .prepare(
                "SELECT result_id, judge_model_id, judge_manifest_json, status,
                        raw_output, error_message, started_at, completed_at
                 FROM benchmark_judge_attempts WHERE run_id = ?1 ORDER BY id",
            )
            .map_err(|e| format!("judge export query error: {e}"))?;
        let judge_attempts = judge_stmt
            .query_map([run_id], |row| {
                Ok(JudgeAttemptExportData {
                    result_id: row.get(0)?,
                    judge_model_id: row.get(1)?,
                    judge_manifest_json: row.get(2)?,
                    status: row.get(3)?,
                    raw_output: row.get(4)?,
                    error_message: row.get(5)?,
                    started_at: row.get(6)?,
                    completed_at: row.get(7)?,
                })
            })
            .map_err(|e| format!("judge export query error: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("judge export row error: {e}"))?;

        let mut comparison_stmt = conn
            .prepare(
                "SELECT id, prompt_id, repetition_index, model_a_id, model_b_id,
                        result_a_id, result_b_id, model_a_position, human_outcome,
                        human_winner_model_id, human_judged_at
                 FROM benchmark_comparisons WHERE run_id = ?1
                 ORDER BY prompt_id, repetition_index",
            )
            .map_err(|e| format!("comparison export query error: {e}"))?;
        let comparisons = comparison_stmt
            .query_map([run_id], |row| {
                Ok(ComparisonExportData {
                    comparison_id: row.get(0)?,
                    prompt_id: row.get(1)?,
                    repetition_index: row.get(2)?,
                    model_a_id: row.get(3)?,
                    model_b_id: row.get(4)?,
                    result_a_id: row.get(5)?,
                    result_b_id: row.get(6)?,
                    model_a_position: row.get(7)?,
                    human_outcome: row.get(8)?,
                    human_winner_model_id: row.get(9)?,
                    human_judged_at: row.get(10)?,
                })
            })
            .map_err(|e| format!("comparison export query error: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("comparison export row error: {e}"))?;
        (manifest, trials, judge_attempts, comparisons)
    };

    serde_json::to_string_pretty(&EvaluationBundleExport {
        version: 2,
        exported_at_unix_ms: unix_time_ms(),
        manifest,
        evidence,
        trials,
        results,
        judge_attempts,
        comparisons,
    })
    .map_err(|e| format!("evaluation bundle serialize error: {e}"))
}

#[tauri::command]
pub async fn save_evaluation_bundle(run_id: i64, path: String) -> Result<(), String> {
    let destination = std::path::PathBuf::from(path);
    if destination.extension().and_then(|value| value.to_str()) != Some("json") {
        return Err("Evaluation evidence must be saved as a .json file".into());
    }
    let contents = export_evaluation_bundle(run_id).await?;
    std::fs::write(&destination, contents)
        .map_err(|e| format!("failed to save evaluation evidence: {e}"))
}

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

    let results = get_benchmark_results(run_id).await?;

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
    fn ollama_unavailable_fails_before_creating_a_run() {
        let error = require_ollama_available(false).unwrap_err();
        assert!(error.contains("No evaluation run was created"));
        assert!(require_ollama_available(true).is_ok());
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
            repetition_index: 0,
            trial_key: Some("eval-1:measured:3:4:0".into()),
            generation_seed: Some(42),
            trial_status: "completed".into(),
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
    fn per_judge_recommendation_requires_non_overlapping_intervals() {
        let strong = CapabilityEvidence {
            category: "coding".into(),
            model_id: 1,
            model_name: "Coder".into(),
            scoring_method: "auto_judge:mistral:7b".into(),
            confidence: evaluation::mean_confidence_95(&[10.0, 10.0, 10.0, 10.0, 10.0]),
        };
        let weak = CapabilityEvidence {
            category: "coding".into(),
            model_id: 2,
            model_name: "General".into(),
            scoring_method: "auto_judge:mistral:7b".into(),
            confidence: evaluation::mean_confidence_95(&[5.0, 5.0, 5.0, 5.0, 5.0]),
        };

        let recommendation = recommendation_for_scored_pair("coding".into(), vec![&weak, &strong]);

        assert_eq!(recommendation.recommended_model.as_deref(), Some("Coder"));
        assert_eq!(recommendation.confidence, "directional");
        assert!(recommendation.reason.contains("auto_judge:mistral:7b"));
    }

    #[test]
    fn per_judge_recommendation_withholds_overlapping_scores() {
        let first = CapabilityEvidence {
            category: "analysis".into(),
            model_id: 1,
            model_name: "First".into(),
            scoring_method: "auto_judge:mistral:7b".into(),
            confidence: evaluation::mean_confidence_95(&[8.0, 9.0, 8.0, 9.0, 8.0]),
        };
        let second = CapabilityEvidence {
            category: "analysis".into(),
            model_id: 2,
            model_name: "Second".into(),
            scoring_method: "auto_judge:mistral:7b".into(),
            confidence: evaluation::mean_confidence_95(&[8.0, 8.0, 9.0, 8.0, 9.0]),
        };

        let recommendation =
            recommendation_for_scored_pair("analysis".into(), vec![&first, &second]);

        assert_eq!(recommendation.recommended_model, None);
        assert_eq!(recommendation.confidence, "inconclusive");
    }

    #[test]
    fn blind_sample_selects_exactly_one_trial_per_prompt_deterministically() {
        let rows = vec![
            (10, 1, 0),
            (11, 1, 1),
            (12, 1, 2),
            (20, 2, 0),
            (21, 2, 1),
            (22, 2, 2),
        ];

        let first = select_one_comparison_per_prompt(rows.clone(), 42, |row| row.1);
        let replay = select_one_comparison_per_prompt(rows, 42, |row| row.1);

        assert_eq!(first, replay);
        assert_eq!(first.len(), 2);
        assert_eq!(first.iter().filter(|row| row.1 == 1).count(), 1);
        assert_eq!(first.iter().filter(|row| row.1 == 2).count(), 1);
    }

    #[test]
    fn blind_sample_seed_changes_selected_repetition() {
        let rows: Vec<_> = (0..5)
            .map(|repetition| (repetition, 7, repetition))
            .collect();
        let first = select_one_comparison_per_prompt(rows.clone(), 1, |row| row.1);
        let second = select_one_comparison_per_prompt(rows, 2, |row| row.1);

        assert_ne!(first[0].2, second[0].2);
    }

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

    fn comparison_manifest() -> RunManifest {
        RunManifest {
            schema_version: 1,
            run_key: "run".into(),
            created_at_unix_ms: 1,
            suite: SuiteSnapshot {
                id: 1,
                name: "suite".into(),
                description: None,
                digest: "suite-digest".into(),
                prompts: vec![],
            },
            models: vec![ModelSnapshot {
                database_id: 1,
                exact_tag: "model:tag".into(),
                digest: Some("model-digest".into()),
                size_bytes: None,
                parameter_size: None,
                quantization: None,
                family: None,
                modified_at: None,
                capabilities: vec![],
            }],
            ollama: OllamaSnapshot {
                server_version: "1.0".into(),
                endpoint: "http://localhost:11434".into(),
            },
            hardware: HardwareSnapshot {
                os_name: Some("macOS".into()),
                os_version: Some("1".into()),
                kernel_version: Some("1".into()),
                architecture: "arm64".into(),
                cpu_brand: Some("Apple".into()),
                logical_cpu_count: 8,
                total_memory_bytes: 16,
            },
            generation: EvaluationConfig::default(),
            measured_trial_count: 3,
            warmup_trial_count: 1,
        }
    }

    #[test]
    fn identical_manifests_are_exact_reproductions() {
        let manifest = comparison_manifest();
        let result = compare_manifests(true, &manifest, true, &manifest).unwrap();
        assert_eq!(result.classification, "exact_reproduction");
        assert!(result.quality_comparable);
        assert!(result.performance_comparable);
    }

    #[test]
    fn hardware_variants_allow_quality_but_not_performance_comparison() {
        let first = comparison_manifest();
        let mut second = first.clone();
        second.hardware.total_memory_bytes = 32;
        let result = compare_manifests(true, &first, true, &second).unwrap();
        assert_eq!(result.classification, "hardware_variant");
        assert!(result.quality_comparable);
        assert!(!result.performance_comparable);
    }

    #[test]
    fn runtime_variants_are_exploratory() {
        let first = comparison_manifest();
        let mut second = first.clone();
        second.ollama.server_version = "2.0".into();
        let result = compare_manifests(true, &first, true, &second).unwrap();
        assert_eq!(result.classification, "runtime_variant");
        assert!(!result.quality_comparable);
        assert!(!result.performance_comparable);
    }

    #[test]
    fn changed_model_identity_is_incomparable() {
        let first = comparison_manifest();
        let mut second = first.clone();
        second.models[0].digest = Some("different".into());
        let result = compare_manifests(true, &first, true, &second).unwrap();
        assert_eq!(result.classification, "incomparable");
        assert!(!result.quality_comparable);
        assert!(!result.performance_comparable);
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
