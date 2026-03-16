use crate::db;
use crate::ollama::{self, GenerateRequest, StreamChunk};
use crate::prompts::{self, RoundContent};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::Emitter;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Managed state
// ---------------------------------------------------------------------------

pub struct ActiveDebates(pub Arc<Mutex<HashMap<i64, CancellationToken>>>);

pub struct ActiveSparrings(pub Arc<Mutex<HashMap<i64, SparringState>>>);

pub struct SparringState {
    pub cancel_token: CancellationToken,
    pub difficulty: String,
    pub model_name: String,
    pub topic: String,
    pub human_side: String,
    pub word_limits: [u32; 4], // [opening, rebuttal1, rebuttal2, closing]
}

// ---------------------------------------------------------------------------
// Event payloads
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DebateModePayload {
    pub debate_id: i64,
    pub mode: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StreamTokenPayload {
    pub debate_id: i64,
    pub round: i32,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoundCompletePayload {
    pub debate_id: i64,
    pub round: i32,
    pub model_a_content: String,
    pub model_b_content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DebateCompletePayload {
    pub debate_id: i64,
    pub total_rounds: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DebateErrorPayload {
    pub debate_id: i64,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DebateAbortedPayload {
    pub debate_id: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SparringStartedPayload {
    pub debate_id: i64,
    pub first_phase: String,
    pub word_limit: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SparringRoundCompletePayload {
    pub debate_id: i64,
    pub round: i32,
    pub phase: String,
    pub ai_content: String,
    pub next_phase: Option<String>,
    pub next_word_limit: Option<u32>,
    pub is_complete: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SparringErrorPayload {
    pub debate_id: i64,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

struct DebateSettings {
    default_rounds: i32,
    default_word_limit: u32,
    concurrent_max_params_billions: i64,
}

fn read_settings() -> Result<DebateSettings, String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;

    let get = |key: &str, default: &str| -> String {
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            rusqlite::params![key],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| default.to_string())
    };

    Ok(DebateSettings {
        default_rounds: get("default_rounds", "5").parse().unwrap_or(5),
        default_word_limit: get("default_word_limit", "300").parse().unwrap_or(300),
        concurrent_max_params_billions: get("concurrent_max_params_billions", "40")
            .parse()
            .unwrap_or(40),
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn round_to_phase(round: i32, total: i32) -> &'static str {
    if round == 1 {
        "opening"
    } else if round == total {
        "closing"
    } else {
        "argument"
    }
}

pub fn sparring_phase_for_round(round: i32) -> &'static str {
    match round {
        1 | 2 => "opening",
        3..=6 => "rebuttal",
        7 | 8 => "closing",
        _ => "argument",
    }
}

fn sparring_phase_index(round: i32) -> usize {
    match round {
        1 | 2 => 0,
        3 | 4 => 1,
        5 | 6 => 2,
        7 | 8 => 3,
        _ => 0,
    }
}

fn word_limit_for_round(round: i32, limits: &[u32; 4]) -> u32 {
    limits[sparring_phase_index(round)]
}

fn default_word_limits() -> [u32; 4] {
    [200, 300, 300, 150]
}

fn cleanup_sparring(map: &Arc<Mutex<HashMap<i64, SparringState>>>, debate_id: i64) {
    if let Ok(mut m) = map.lock() {
        m.remove(&debate_id);
    }
}

struct RoundMetrics {
    tokens_generated: Option<u64>,
    time_to_first_token_ms: Option<u64>,
    generation_time_ms: u64,
    tokens_per_second: Option<f64>,
}

async fn stream_and_collect(
    app: &tauri::AppHandle,
    event_name: &str,
    debate_id: i64,
    round: i32,
    mut rx: tokio::sync::mpsc::Receiver<Result<StreamChunk, String>>,
    cancel_token: &CancellationToken,
    start: Instant,
) -> Result<(String, RoundMetrics), String> {
    let mut buffer = String::new();
    let mut first_token_time: Option<u64> = None;
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
                                    first_token_time = Some(start.elapsed().as_millis() as u64);
                                }
                                buffer.push_str(text);
                                let _ = app.emit(event_name, StreamTokenPayload {
                                    debate_id,
                                    round,
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

    let generation_time_ms = start.elapsed().as_millis() as u64;
    let tps = match (eval_count, eval_duration) {
        (Some(count), Some(dur)) if dur > 0 => {
            Some(count as f64 / (dur as f64 / 1_000_000_000.0))
        }
        _ => None,
    };

    Ok((
        buffer,
        RoundMetrics {
            tokens_generated: eval_count,
            time_to_first_token_ms: first_token_time,
            generation_time_ms,
            tokens_per_second: tps,
        },
    ))
}

fn save_round(
    debate_id: i64,
    round_number: i32,
    speaker: &str,
    phase: &str,
    content: &str,
    metrics: &RoundMetrics,
) -> Result<(), String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    conn.execute(
        "INSERT INTO rounds (debate_id, round_number, speaker, phase, content,
            tokens_generated, time_to_first_token_ms, generation_time_ms, tokens_per_second)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            debate_id,
            round_number,
            speaker,
            phase,
            content,
            metrics.tokens_generated.map(|v| v as i64),
            metrics.time_to_first_token_ms.map(|v| v as i64),
            metrics.generation_time_ms as i64,
            metrics.tokens_per_second,
        ],
    )
    .map_err(|e| format!("save round error: {e}"))?;
    Ok(())
}

fn abort_debate_in_db(debate_id: i64) -> Result<(), String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    conn.execute(
        "UPDATE debates SET status = 'abandoned', completed_at = datetime('now') WHERE id = ?1",
        rusqlite::params![debate_id],
    )
    .map_err(|e| format!("abort db error: {e}"))?;
    Ok(())
}

fn cleanup_token(map: &Arc<Mutex<HashMap<i64, CancellationToken>>>, debate_id: i64) {
    if let Ok(mut m) = map.lock() {
        m.remove(&debate_id);
    }
}

fn load_history(debate_id: i64) -> Result<Vec<RoundContent>, String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    let mut stmt = conn
        .prepare(
            "SELECT speaker, content, round_number FROM rounds
             WHERE debate_id = ?1 ORDER BY round_number, id",
        )
        .map_err(|e| format!("history query error: {e}"))?;

    let rows = stmt
        .query_map(rusqlite::params![debate_id], |row| {
            Ok(RoundContent {
                speaker: row.get(0)?,
                content: row.get(1)?,
                round_number: row.get(2)?,
            })
        })
        .map_err(|e| format!("history query error: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("history row error: {e}"))?;

    Ok(rows)
}

// ---------------------------------------------------------------------------
// Debate loop (spawned task)
// ---------------------------------------------------------------------------

macro_rules! or_abort {
    ($app:expr, $map:expr, $debate_id:expr, $expr:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => {
                let msg = e.to_string();
                let _ = abort_debate_in_db($debate_id);
                cleanup_token(&$map, $debate_id);
                let _ = $app.emit(
                    "debate:error",
                    DebateErrorPayload {
                        debate_id: $debate_id,
                        message: msg,
                    },
                );
                return;
            }
        }
    };
}

#[allow(clippy::too_many_arguments)]
async fn run_debate_loop(
    app: tauri::AppHandle,
    active_map: Arc<Mutex<HashMap<i64, CancellationToken>>>,
    token: CancellationToken,
    debate_id: i64,
    model_a_name: String,
    model_b_name: String,
    total_rounds: i32,
    word_limit: u32,
    topic: String,
    concurrent: bool,
) {
    let num_predict = word_limit * 2;

    for round in 1..=total_rounds {
        if token.is_cancelled() {
            let _ = abort_debate_in_db(debate_id);
            cleanup_token(&active_map, debate_id);
            let _ = app.emit("debate:aborted", DebateAbortedPayload { debate_id });
            return;
        }

        let history = or_abort!(app, active_map, debate_id, load_history(debate_id));

        let system_a = prompts::build_arena_system_prompt(
            "pro", &topic, round, word_limit, &history, "model_a",
        );
        let system_b = prompts::build_arena_system_prompt(
            "con", &topic, round, word_limit, &history, "model_b",
        );

        let user_prompt = if round == 1 {
            topic.clone()
        } else {
            "Continue the debate.".into()
        };

        let req_a = GenerateRequest {
            model: model_a_name.clone(),
            prompt: user_prompt.clone(),
            system: Some(system_a),
            num_predict: Some(num_predict),
            temperature: Some(0.7),
        };
        let req_b = GenerateRequest {
            model: model_b_name.clone(),
            prompt: user_prompt,
            system: Some(system_b),
            num_predict: Some(num_predict),
            temperature: Some(0.7),
        };

        let phase = round_to_phase(round, total_rounds);

        if concurrent {
            let rx_a = or_abort!(app, active_map, debate_id, ollama::generate_stream(req_a).await);
            let rx_b = or_abort!(app, active_map, debate_id, ollama::generate_stream(req_b).await);

            let start_a = Instant::now();
            let start_b = Instant::now();

            let (result_a, result_b) = tokio::join!(
                stream_and_collect(&app, "debate:stream:a", debate_id, round, rx_a, &token, start_a),
                stream_and_collect(&app, "debate:stream:b", debate_id, round, rx_b, &token, start_b),
            );

            let (content_a, metrics_a) = or_abort!(app, active_map, debate_id, result_a);
            let (content_b, metrics_b) = or_abort!(app, active_map, debate_id, result_b);

            or_abort!(app, active_map, debate_id,
                save_round(debate_id, round, "model_a", phase, &content_a, &metrics_a));
            or_abort!(app, active_map, debate_id,
                save_round(debate_id, round, "model_b", phase, &content_b, &metrics_b));

            let _ = app.emit(
                "debate:round_complete",
                RoundCompletePayload {
                    debate_id,
                    round,
                    model_a_content: content_a,
                    model_b_content: content_b,
                },
            );
        } else {
            // Sequential: stream A fully, then B
            let start_a = Instant::now();
            let rx_a = or_abort!(app, active_map, debate_id, ollama::generate_stream(req_a).await);
            let (content_a, metrics_a) = or_abort!(app, active_map, debate_id,
                stream_and_collect(&app, "debate:stream:a", debate_id, round, rx_a, &token, start_a).await);

            or_abort!(app, active_map, debate_id,
                save_round(debate_id, round, "model_a", phase, &content_a, &metrics_a));

            let start_b = Instant::now();
            let rx_b = or_abort!(app, active_map, debate_id, ollama::generate_stream(req_b).await);
            let (content_b, metrics_b) = or_abort!(app, active_map, debate_id,
                stream_and_collect(&app, "debate:stream:b", debate_id, round, rx_b, &token, start_b).await);

            or_abort!(app, active_map, debate_id,
                save_round(debate_id, round, "model_b", phase, &content_b, &metrics_b));

            let _ = app.emit(
                "debate:round_complete",
                RoundCompletePayload {
                    debate_id,
                    round,
                    model_a_content: content_a,
                    model_b_content: content_b,
                },
            );
        }
    }

    // Debate finished — update status
    {
        let conn = match db::get_db().lock() {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit(
                    "debate:error",
                    DebateErrorPayload {
                        debate_id,
                        message: format!("db lock: {e}"),
                    },
                );
                cleanup_token(&active_map, debate_id);
                return;
            }
        };
        let _ = conn.execute(
            "UPDATE debates SET status = 'voting', completed_at = datetime('now') WHERE id = ?1",
            rusqlite::params![debate_id],
        );
    }

    cleanup_token(&active_map, debate_id);
    let _ = app.emit(
        "debate:complete",
        DebateCompletePayload {
            debate_id,
            total_rounds,
        },
    );
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn start_debate(
    app: tauri::AppHandle,
    state: tauri::State<'_, ActiveDebates>,
    topic: String,
    model_a_id: i64,
    model_b_id: i64,
    rounds: Option<i32>,
    format: Option<String>,
) -> Result<i64, String> {
    let settings = read_settings()?;
    let total_rounds = rounds.unwrap_or(settings.default_rounds);
    let word_limit = settings.default_word_limit;

    // Fetch model info
    let (model_a_name, param_a, model_b_name, param_b) = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        let fetch = |id: i64| -> Result<(String, Option<i64>), String> {
            conn.query_row(
                "SELECT name, parameter_count FROM models WHERE id = ?1",
                rusqlite::params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| format!("model not found (id={id}): {e}"))
        };
        let (na, pa) = fetch(model_a_id)?;
        let (nb, pb) = fetch(model_b_id)?;
        (na, pa, nb, pb)
    };

    let combined_params = param_a.unwrap_or(0) + param_b.unwrap_or(0);
    let concurrent = combined_params <= settings.concurrent_max_params_billions;

    let debate_format = format.unwrap_or_else(|| "freestyle".into());

    // Insert debate record
    let debate_id = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        conn.execute(
            "INSERT INTO debates (topic, mode, debate_format, model_a_id, model_b_id, total_rounds, status)
             VALUES (?1, 'arena', ?2, ?3, ?4, ?5, 'in_progress')",
            rusqlite::params![topic, debate_format, model_a_id, model_b_id, total_rounds],
        )
        .map_err(|e| format!("insert debate error: {e}"))?;
        conn.last_insert_rowid()
    };

    // Create cancellation token
    let cancel_token = CancellationToken::new();
    {
        let mut map = state
            .0
            .lock()
            .map_err(|e| format!("state lock: {e}"))?;
        map.insert(debate_id, cancel_token.clone());
    }

    let mode_str = if concurrent {
        "concurrent"
    } else {
        "sequential"
    };
    let _ = app.emit(
        "debate:mode",
        DebateModePayload {
            debate_id,
            mode: mode_str.into(),
        },
    );

    let active_map = Arc::clone(&state.0);
    tokio::spawn(run_debate_loop(
        app,
        active_map,
        cancel_token,
        debate_id,
        model_a_name,
        model_b_name,
        total_rounds,
        word_limit,
        topic,
        concurrent,
    ));

    Ok(debate_id)
}

#[tauri::command]
pub async fn abort_debate(
    state: tauri::State<'_, ActiveDebates>,
    debate_id: i64,
) -> Result<(), String> {
    let found = {
        let map = state.0.lock().map_err(|e| format!("state lock: {e}"))?;
        if let Some(token) = map.get(&debate_id) {
            token.cancel();
            true
        } else {
            false
        }
    };

    if !found {
        // Debate already finished or unknown — mark abandoned as safety net
        abort_debate_in_db(debate_id)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Vote command
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Clone)]
pub struct VoteResult {
    pub debate_id: i64,
    pub rating_a_before: f64,
    pub rating_a_after: f64,
    pub rating_b_before: f64,
    pub rating_b_after: f64,
}

#[tauri::command]
pub async fn vote_debate(debate_id: i64, winner: String) -> Result<VoteResult, String> {
    let valid = ["model_a", "model_b", "draw"];
    if !valid.contains(&winner.as_str()) {
        return Err(format!(
            "Invalid winner '{}': must be 'model_a', 'model_b', or 'draw'",
            winner
        ));
    }

    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;

    // Read debate
    let (model_a_id, model_b_id, status): (i64, i64, String) = conn
        .query_row(
            "SELECT model_a_id, model_b_id, status FROM debates WHERE id = ?1",
            rusqlite::params![debate_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| format!("debate not found (id={debate_id}): {e}"))?;

    if status != "voting" {
        return Err(format!(
            "Debate is not in voting state (current status: '{status}')"
        ));
    }

    // Read model A
    let (rating_a, total_a): (f64, i64) = conn
        .query_row(
            "SELECT elo_rating, total_debates FROM models WHERE id = ?1",
            rusqlite::params![model_a_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("model_a not found (id={model_a_id}): {e}"))?;

    // Read model B
    let (rating_b, total_b): (f64, i64) = conn
        .query_row(
            "SELECT elo_rating, total_debates FROM models WHERE id = ?1",
            rusqlite::params![model_b_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("model_b not found (id={model_b_id}): {e}"))?;

    // Map winner to Outcome from model_a perspective
    let outcome = match winner.as_str() {
        "model_a" => crate::elo::Outcome::Win,
        "model_b" => crate::elo::Outcome::Loss,
        _ => crate::elo::Outcome::Draw,
    };

    let (new_a, new_b, k_a, k_b) = crate::elo::update_ratings(
        rating_a,
        rating_b,
        outcome,
        total_a as u32,
        total_b as u32,
    );

    // Win/loss/draw increments for each model
    let (a_wins, a_losses, a_draws, b_wins, b_losses, b_draws) = match winner.as_str() {
        "model_a" => (1i64, 0i64, 0i64, 0i64, 1i64, 0i64),
        "model_b" => (0i64, 1i64, 0i64, 1i64, 0i64, 0i64),
        _ => (0i64, 0i64, 1i64, 0i64, 0i64, 1i64),
    };

    // Transaction: update all records
    conn.execute_batch("BEGIN")
        .map_err(|e| format!("begin transaction: {e}"))?;

    let result = (|| -> Result<(), String> {
        // Update debate status and winner
        conn.execute(
            "UPDATE debates SET winner = ?1, status = 'completed' WHERE id = ?2",
            rusqlite::params![winner, debate_id],
        )
        .map_err(|e| format!("update debate: {e}"))?;

        // Insert elo_history for model_a
        conn.execute(
            "INSERT INTO elo_history (model_id, debate_id, rating_before, rating_after, k_factor)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![model_a_id, debate_id, rating_a, new_a, k_a],
        )
        .map_err(|e| format!("insert elo_history model_a: {e}"))?;

        // Insert elo_history for model_b
        conn.execute(
            "INSERT INTO elo_history (model_id, debate_id, rating_before, rating_after, k_factor)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![model_b_id, debate_id, rating_b, new_b, k_b],
        )
        .map_err(|e| format!("insert elo_history model_b: {e}"))?;

        // Update model_a stats
        conn.execute(
            "UPDATE models SET
                elo_rating = ?1,
                arena_wins = arena_wins + ?2,
                arena_losses = arena_losses + ?3,
                arena_draws = arena_draws + ?4,
                total_debates = total_debates + 1,
                last_used_at = datetime('now')
             WHERE id = ?5",
            rusqlite::params![new_a, a_wins, a_losses, a_draws, model_a_id],
        )
        .map_err(|e| format!("update model_a: {e}"))?;

        // Update model_b stats
        conn.execute(
            "UPDATE models SET
                elo_rating = ?1,
                arena_wins = arena_wins + ?2,
                arena_losses = arena_losses + ?3,
                arena_draws = arena_draws + ?4,
                total_debates = total_debates + 1,
                last_used_at = datetime('now')
             WHERE id = ?5",
            rusqlite::params![new_b, b_wins, b_losses, b_draws, model_b_id],
        )
        .map_err(|e| format!("update model_b: {e}"))?;

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

    Ok(VoteResult {
        debate_id,
        rating_a_before: rating_a,
        rating_a_after: new_a,
        rating_b_before: rating_b,
        rating_b_after: new_b,
    })
}

// ---------------------------------------------------------------------------
// Sparring commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn start_sparring(
    app: tauri::AppHandle,
    state: tauri::State<'_, ActiveSparrings>,
    topic: String,
    model_id: i64,
    human_side: String,
    difficulty: String,
) -> Result<i64, String> {
    // Validate inputs
    if !["pro", "con"].contains(&human_side.as_str()) {
        return Err(format!("Invalid human_side '{}': must be 'pro' or 'con'", human_side));
    }
    if !["casual", "competitive", "expert"].contains(&difficulty.as_str()) {
        return Err(format!("Invalid difficulty '{}': must be 'casual', 'competitive', or 'expert'", difficulty));
    }

    // Fetch model name from DB
    let model_name = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        conn.query_row(
            "SELECT name FROM models WHERE id = ?1",
            rusqlite::params![model_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|e| format!("model not found (id={model_id}): {e}"))?
    };

    // Insert debate record
    let debate_id = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        conn.execute(
            "INSERT INTO debates (topic, mode, debate_format, model_a_id, model_b_id, human_side, total_rounds, status)
             VALUES (?1, 'sparring', 'freestyle', ?2, NULL, ?3, 8, 'in_progress')",
            rusqlite::params![topic, model_id, human_side],
        )
        .map_err(|e| format!("insert debate error: {e}"))?;
        conn.last_insert_rowid()
    };

    // Create state
    let cancel_token = CancellationToken::new();
    let word_limits = default_word_limits();
    {
        let mut map = state.0.lock().map_err(|e| format!("state lock: {e}"))?;
        map.insert(debate_id, SparringState {
            cancel_token: cancel_token.clone(),
            difficulty: difficulty.clone(),
            model_name,
            topic: topic.clone(),
            human_side: human_side.clone(),
            word_limits,
        });
    }

    // Emit started event
    let _ = app.emit("sparring:started", SparringStartedPayload {
        debate_id,
        first_phase: "opening".into(),
        word_limit: word_limits[0],
    });

    Ok(debate_id)
}

#[tauri::command]
pub async fn submit_human_argument(
    app: tauri::AppHandle,
    state: tauri::State<'_, ActiveSparrings>,
    debate_id: i64,
    content: String,
) -> Result<(), String> {
    // Read state
    let (cancel_token, difficulty, model_name, topic, human_side, word_limits) = {
        let map = state.0.lock().map_err(|e| format!("state lock: {e}"))?;
        let s = map.get(&debate_id).ok_or_else(|| format!("No active sparring session for debate {debate_id}"))?;
        (
            s.cancel_token.clone(),
            s.difficulty.clone(),
            s.model_name.clone(),
            s.topic.clone(),
            s.human_side.clone(),
            s.word_limits,
        )
    };

    // Count existing rounds
    let existing_rounds = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        conn.query_row(
            "SELECT COUNT(*) FROM rounds WHERE debate_id = ?1",
            rusqlite::params![debate_id],
            |row| row.get::<_, i32>(0),
        )
        .map_err(|e| format!("count rounds error: {e}"))?
    };

    let human_round = existing_rounds + 1;
    let ai_round = existing_rounds + 2;

    // Validate it's human's turn (odd rounds)
    if human_round % 2 != 1 {
        return Err(format!("Not human's turn (round {human_round})"));
    }

    if human_round > 8 {
        return Err("Sparring session is already complete (8 rounds done)".into());
    }

    let phase = sparring_phase_for_round(human_round);

    // Save human round with zero metrics
    save_round(debate_id, human_round, "human", phase, &content, &RoundMetrics {
        tokens_generated: None,
        time_to_first_token_ms: None,
        generation_time_ms: 0,
        tokens_per_second: None,
    })?;

    // Spawn AI response
    let active_map = Arc::clone(&state.0);
    tokio::spawn(async move {
        let ai_phase = sparring_phase_for_round(ai_round);
        let ai_word_limit = word_limit_for_round(ai_round, &word_limits);

        // Load history
        let history = match load_history(debate_id) {
            Ok(h) => h,
            Err(e) => {
                let _ = app.emit("sparring:error", SparringErrorPayload {
                    debate_id,
                    message: e,
                });
                return;
            }
        };

        // Determine AI side (opposite of human)
        let ai_side = if human_side == "pro" { "con" } else { "pro" };

        // Build prompt
        let system_prompt = prompts::build_sparring_system_prompt(
            &difficulty, ai_side, &topic, ai_phase, ai_word_limit, &history,
        );

        let req = ollama::GenerateRequest {
            model: model_name,
            prompt: "Continue the debate.".into(),
            system: Some(system_prompt),
            num_predict: Some(ai_word_limit * 2),
            temperature: Some(0.7),
        };

        // Start stream
        let rx = match ollama::generate_stream(req).await {
            Ok(rx) => rx,
            Err(e) => {
                let _ = app.emit("sparring:error", SparringErrorPayload {
                    debate_id,
                    message: e,
                });
                return;
            }
        };

        let start = Instant::now();
        let result = stream_and_collect(
            &app, "sparring:stream", debate_id, ai_round, rx, &cancel_token, start,
        ).await;

        match result {
            Ok((ai_content, metrics)) => {
                if let Err(e) = save_round(debate_id, ai_round, "model_a", ai_phase, &ai_content, &metrics) {
                    let _ = app.emit("sparring:error", SparringErrorPayload {
                        debate_id,
                        message: e,
                    });
                    return;
                }

                let is_complete = ai_round >= 8;

                // Determine next phase info
                let (next_phase, next_word_limit) = if is_complete {
                    (None, None)
                } else {
                    let next_round = ai_round + 1;
                    (
                        Some(sparring_phase_for_round(next_round).to_string()),
                        Some(word_limit_for_round(next_round, &word_limits)),
                    )
                };

                let _ = app.emit("sparring:round_complete", SparringRoundCompletePayload {
                    debate_id,
                    round: ai_round,
                    phase: ai_phase.to_string(),
                    ai_content,
                    next_phase,
                    next_word_limit,
                    is_complete,
                });

                if is_complete {
                    // Update DB status
                    {
                        let conn = match db::get_db().lock() {
                            Ok(c) => c,
                            Err(e) => {
                                let _ = app.emit("sparring:error", SparringErrorPayload {
                                    debate_id,
                                    message: format!("db lock: {e}"),
                                });
                                return;
                            }
                        };
                        let _ = conn.execute(
                            "UPDATE debates SET status = 'completed', completed_at = datetime('now') WHERE id = ?1",
                            rusqlite::params![debate_id],
                        );
                    }
                    cleanup_sparring(&active_map, debate_id);
                    let _ = app.emit("sparring:complete", DebateCompletePayload {
                        debate_id,
                        total_rounds: 8,
                    });
                }
            }
            Err(e) if e == "cancelled" => {
                let _ = abort_debate_in_db(debate_id);
                cleanup_sparring(&active_map, debate_id);
                let _ = app.emit("sparring:aborted", DebateAbortedPayload { debate_id });
            }
            Err(e) => {
                let _ = abort_debate_in_db(debate_id);
                cleanup_sparring(&active_map, debate_id);
                let _ = app.emit("sparring:error", SparringErrorPayload {
                    debate_id,
                    message: e,
                });
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn abort_sparring(
    state: tauri::State<'_, ActiveSparrings>,
    debate_id: i64,
) -> Result<(), String> {
    let found = {
        let map = state.0.lock().map_err(|e| format!("state lock: {e}"))?;
        if let Some(s) = map.get(&debate_id) {
            s.cancel_token.cancel();
            true
        } else {
            false
        }
    };

    if !found {
        abort_debate_in_db(debate_id)?;
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
    fn round_to_phase_mapping() {
        assert_eq!(round_to_phase(1, 5), "opening");
        assert_eq!(round_to_phase(2, 5), "argument");
        assert_eq!(round_to_phase(3, 5), "argument");
        assert_eq!(round_to_phase(4, 5), "argument");
        assert_eq!(round_to_phase(5, 5), "closing");
    }

    #[test]
    fn round_to_phase_single_round() {
        // Single round debate: round 1 is opening (not closing)
        assert_eq!(round_to_phase(1, 1), "opening");
    }

    #[test]
    fn round_to_phase_two_rounds() {
        assert_eq!(round_to_phase(1, 2), "opening");
        assert_eq!(round_to_phase(2, 2), "closing");
    }

    #[test]
    fn vote_debate_validates_winner() {
        let valid = ["model_a", "model_b", "draw"];
        assert!(valid.contains(&"model_a"));
        assert!(valid.contains(&"model_b"));
        assert!(valid.contains(&"draw"));
        assert!(!valid.contains(&"invalid"));
    }

    #[test]
    fn sparring_phase_mapping() {
        assert_eq!(sparring_phase_for_round(1), "opening");
        assert_eq!(sparring_phase_for_round(2), "opening");
        assert_eq!(sparring_phase_for_round(3), "rebuttal");
        assert_eq!(sparring_phase_for_round(4), "rebuttal");
        assert_eq!(sparring_phase_for_round(5), "rebuttal");
        assert_eq!(sparring_phase_for_round(6), "rebuttal");
        assert_eq!(sparring_phase_for_round(7), "closing");
        assert_eq!(sparring_phase_for_round(8), "closing");
    }

    #[test]
    fn sparring_phase_index_mapping() {
        assert_eq!(sparring_phase_index(1), 0);
        assert_eq!(sparring_phase_index(2), 0);
        assert_eq!(sparring_phase_index(3), 1);
        assert_eq!(sparring_phase_index(4), 1);
        assert_eq!(sparring_phase_index(5), 2);
        assert_eq!(sparring_phase_index(6), 2);
        assert_eq!(sparring_phase_index(7), 3);
        assert_eq!(sparring_phase_index(8), 3);
    }

    #[test]
    fn word_limits_per_round() {
        let limits = default_word_limits();
        assert_eq!(word_limit_for_round(1, &limits), 200);
        assert_eq!(word_limit_for_round(2, &limits), 200);
        assert_eq!(word_limit_for_round(3, &limits), 300);
        assert_eq!(word_limit_for_round(5, &limits), 300);
        assert_eq!(word_limit_for_round(7, &limits), 150);
        assert_eq!(word_limit_for_round(8, &limits), 150);
    }

    #[test]
    fn sparring_out_of_range_round() {
        // Out of range defaults to "argument" / index 0
        assert_eq!(sparring_phase_for_round(9), "argument");
        assert_eq!(sparring_phase_for_round(0), "argument");
        assert_eq!(sparring_phase_index(10), 0);
    }
}
