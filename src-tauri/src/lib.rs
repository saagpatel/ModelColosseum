mod benchmark;
mod db;
mod debate;
mod elo;
mod ollama;
mod prompts;

use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Clone)]
pub struct Model {
    pub id: i64,
    pub name: String,
    pub display_name: String,
    pub parameter_count: Option<i64>,
    pub quantization: Option<String>,
    pub family: Option<String>,
    pub elo_rating: f64,
    pub arena_wins: i64,
    pub arena_losses: i64,
    pub arena_draws: i64,
    pub total_debates: i64,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EloHistoryPoint {
    pub rating: f64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct DebateSummary {
    pub id: i64,
    pub topic: String,
    pub model_a_name: String,
    pub model_b_name: String,
    pub winner: Option<String>,
    pub status: String,
    pub total_rounds: i32,
    pub created_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct RoundTranscript {
    pub round_number: i32,
    pub speaker: String,
    pub phase: String,
    pub content: String,
}

fn make_display_name(name: &str) -> String {
    // "qwen3:32b-q4_k_m" → "Qwen3 32B"
    let base = name.split(':').next().unwrap_or(name);
    let tag = name.split(':').nth(1).unwrap_or("");

    let mut display = String::new();
    let mut prev_is_lower = false;
    for ch in base.chars() {
        if ch.is_uppercase() && prev_is_lower {
            display.push(' ');
        }
        display.push(ch);
        prev_is_lower = ch.is_lowercase();
    }

    // Extract size from tag if present (e.g., "32b", "7b")
    let size_part: String = tag
        .split('-')
        .find(|s| s.ends_with('b') && s[..s.len() - 1].parse::<f64>().is_ok())
        .unwrap_or("")
        .to_uppercase();

    if !size_part.is_empty() && !display.to_lowercase().contains(&size_part.to_lowercase()) {
        display.push(' ');
        display.push_str(&size_part);
    }

    // Capitalize first letter
    let mut chars = display.chars();
    match chars.next() {
        None => display,
        Some(first) => first.to_uppercase().to_string() + chars.as_str(),
    }
}

fn parse_parameter_count(size_str: &str) -> Option<i64> {
    // "32B" → 32, "7.5B" → 8, "671M" → 0
    let s = size_str.trim().to_uppercase();
    if s.ends_with('B') {
        s[..s.len() - 1].parse::<f64>().ok().map(|v| v.round() as i64)
    } else if s.ends_with('M') {
        // Millions — store as 0 for billion-scale comparison
        Some(0)
    } else {
        None
    }
}

#[tauri::command]
async fn health_check() -> Result<bool, String> {
    ollama::health_check().await
}

#[tauri::command]
async fn list_models() -> Result<Vec<Model>, String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, display_name, parameter_count, quantization, family,
                    elo_rating, arena_wins, arena_losses, arena_draws, total_debates,
                    last_used_at
             FROM models ORDER BY elo_rating DESC",
        )
        .map_err(|e| format!("query error: {e}"))?;

    let models = stmt
        .query_map([], |row| {
            Ok(Model {
                id: row.get(0)?,
                name: row.get(1)?,
                display_name: row.get(2)?,
                parameter_count: row.get(3)?,
                quantization: row.get(4)?,
                family: row.get(5)?,
                elo_rating: row.get(6)?,
                arena_wins: row.get(7)?,
                arena_losses: row.get(8)?,
                arena_draws: row.get(9)?,
                total_debates: row.get(10)?,
                last_used_at: row.get(11)?,
            })
        })
        .map_err(|e| format!("query error: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("row error: {e}"))?;

    Ok(models)
}

#[tauri::command]
async fn get_leaderboard() -> Result<Vec<Model>, String> {
    list_models().await
}

#[tauri::command]
async fn get_model_elo_history(model_id: i64, limit: Option<i64>) -> Result<Vec<EloHistoryPoint>, String> {
    let effective_limit = limit.unwrap_or(20);
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    let mut stmt = conn
        .prepare(
            "SELECT rating_after, created_at FROM elo_history
             WHERE model_id = ?1 ORDER BY created_at ASC LIMIT ?2",
        )
        .map_err(|e| format!("query error: {e}"))?;

    let points = stmt
        .query_map(rusqlite::params![model_id, effective_limit], |row| {
            Ok(EloHistoryPoint {
                rating: row.get(0)?,
                created_at: row.get(1)?,
            })
        })
        .map_err(|e| format!("query error: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("row error: {e}"))?;

    Ok(points)
}

#[tauri::command]
async fn get_debates(
    cursor: Option<i64>,
    limit: Option<i64>,
    search: Option<String>,
    model_id: Option<i64>,
) -> Result<Vec<DebateSummary>, String> {
    let effective_limit = limit.unwrap_or(20);
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;

    // Build dynamic query
    let mut conditions = vec!["d.status IN ('completed', 'voting', 'abandoned')".to_string()];
    let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(c) = cursor {
        conditions.push(format!("d.id < ?{}", param_values.len() + 1));
        param_values.push(Box::new(c));
    }
    if let Some(ref s) = search {
        conditions.push(format!("d.topic LIKE ?{}", param_values.len() + 1));
        param_values.push(Box::new(format!("%{s}%")));
    }
    if let Some(mid) = model_id {
        conditions.push(format!(
            "(d.model_a_id = ?{0} OR d.model_b_id = ?{0})",
            param_values.len() + 1
        ));
        param_values.push(Box::new(mid));
    }

    let where_clause = conditions.join(" AND ");
    let limit_param_idx = param_values.len() + 1;
    param_values.push(Box::new(effective_limit));

    let sql = format!(
        "SELECT d.id, d.topic, ma.display_name, mb.display_name, d.winner, d.status,
                d.total_rounds, d.created_at
         FROM debates d
         JOIN models ma ON d.model_a_id = ma.id
         JOIN models mb ON d.model_b_id = mb.id
         WHERE {where_clause}
         ORDER BY d.id DESC
         LIMIT ?{limit_param_idx}"
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| format!("query error: {e}"))?;

    let params_refs: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

    let debates = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(DebateSummary {
                id: row.get(0)?,
                topic: row.get(1)?,
                model_a_name: row.get(2)?,
                model_b_name: row.get(3)?,
                winner: row.get(4)?,
                status: row.get(5)?,
                total_rounds: row.get(6)?,
                created_at: row.get(7)?,
            })
        })
        .map_err(|e| format!("query error: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("row error: {e}"))?;

    Ok(debates)
}

#[tauri::command]
async fn get_debate_transcript(debate_id: i64) -> Result<Vec<RoundTranscript>, String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    let mut stmt = conn
        .prepare(
            "SELECT round_number, speaker, phase, content FROM rounds
             WHERE debate_id = ?1 ORDER BY round_number, id",
        )
        .map_err(|e| format!("query error: {e}"))?;

    let rounds = stmt
        .query_map(rusqlite::params![debate_id], |row| {
            Ok(RoundTranscript {
                round_number: row.get(0)?,
                speaker: row.get(1)?,
                phase: row.get(2)?,
                content: row.get(3)?,
            })
        })
        .map_err(|e| format!("query error: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("row error: {e}"))?;

    Ok(rounds)
}

#[tauri::command]
async fn refresh_models() -> Result<Vec<Model>, String> {
    let ollama_models = ollama::list_models().await?;

    // Fetch detailed metadata for each model and upsert into DB
    for m in &ollama_models {
        let (param_count, quant, family) = match &m.details {
            Some(d) => (
                d.parameter_size
                    .as_deref()
                    .and_then(parse_parameter_count),
                d.quantization_level.clone(),
                d.family.clone(),
            ),
            None => {
                // Try /api/show for more details
                match ollama::show_model(&m.name).await {
                    Ok(show) => {
                        let details = show.details.as_ref();
                        (
                            details
                                .and_then(|d| d.parameter_size.as_deref())
                                .and_then(parse_parameter_count),
                            details.and_then(|d| d.quantization_level.clone()),
                            details.and_then(|d| d.family.clone()),
                        )
                    }
                    Err(_) => (None, None, None),
                }
            }
        };

        let display = make_display_name(&m.name);

        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        conn.execute(
            "INSERT INTO models (name, display_name, parameter_count, quantization, family)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(name) DO UPDATE SET
                display_name = ?2,
                parameter_count = COALESCE(?3, models.parameter_count),
                quantization = COALESCE(?4, models.quantization),
                family = COALESCE(?5, models.family)",
            rusqlite::params![m.name, display, param_count, quant, family],
        )
        .map_err(|e| format!("upsert error: {e}"))?;
    }

    list_models().await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    db::init_db().expect("failed to initialize database");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(debate::ActiveDebates(Arc::new(Mutex::new(HashMap::new()))))
        .manage(benchmark::ActiveBenchmarks(Arc::new(Mutex::new(HashMap::new()))))
        .manage(benchmark::ActiveJudgeRuns(Arc::new(Mutex::new(HashMap::new()))))
        .invoke_handler(tauri::generate_handler![
            health_check,
            list_models,
            refresh_models,
            get_leaderboard,
            get_model_elo_history,
            get_debates,
            get_debate_transcript,
            debate::start_debate,
            debate::abort_debate,
            debate::vote_debate,
            benchmark::list_test_suites,
            benchmark::create_test_suite,
            benchmark::update_test_suite,
            benchmark::delete_test_suite,
            benchmark::list_prompts,
            benchmark::create_prompt,
            benchmark::update_prompt,
            benchmark::delete_prompt,
            benchmark::reorder_prompts,
            benchmark::start_benchmark,
            benchmark::cancel_benchmark,
            benchmark::get_benchmark_results,
            benchmark::score_result,
            benchmark::list_benchmark_runs,
            benchmark::auto_judge_benchmark,
            benchmark::cancel_auto_judge,
            benchmark::get_benchmark_leaderboard,
            benchmark::get_run_comparison,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
