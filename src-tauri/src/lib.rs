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
    let mut conditions = vec![
        "d.mode = 'arena'".to_string(),
        "d.status IN ('completed', 'voting', 'abandoned')".to_string(),
    ];
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

#[tauri::command]
async fn get_user_stats() -> Result<debate::UserStats, String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    conn.query_row(
        "SELECT elo_rating, total_debates, wins, losses, draws FROM user_stats WHERE id = 1",
        [],
        |row| {
            Ok(debate::UserStats {
                elo_rating: row.get(0)?,
                total_debates: row.get(1)?,
                wins: row.get(2)?,
                losses: row.get(3)?,
                draws: row.get(4)?,
            })
        },
    )
    .map_err(|e| format!("user stats query error: {e}"))
}

#[tauri::command]
async fn export_debate_transcript(debate_id: i64) -> Result<String, String> {
    // 1. Fetch debate metadata
    struct DebateMeta {
        topic: String,
        mode: String,
        human_side: Option<String>,
        winner: Option<String>,
        model_a_id: Option<i64>,
        model_b_id: Option<i64>,
        created_at: String,
    }
    let meta: DebateMeta = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        conn.query_row(
            "SELECT topic, mode, human_side, winner, model_a_id, model_b_id, created_at
             FROM debates WHERE id = ?1",
            rusqlite::params![debate_id],
            |row| {
                Ok(DebateMeta {
                    topic: row.get(0)?,
                    mode: row.get(1)?,
                    human_side: row.get(2)?,
                    winner: row.get(3)?,
                    model_a_id: row.get(4)?,
                    model_b_id: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        )
        .map_err(|e| format!("debate not found (id={debate_id}): {e}"))?
    };
    let topic = meta.topic;
    let mode = meta.mode;
    let human_side = meta.human_side;
    let winner = meta.winner;
    let model_a_id = meta.model_a_id;
    let model_b_id = meta.model_b_id;
    let created_at = meta.created_at;

    // 2. Resolve model display names
    let resolve_model_name = |id: Option<i64>| -> String {
        let Some(mid) = id else { return "Unknown".into() };
        let Ok(conn) = db::get_db().lock() else { return "Unknown".into() };
        conn.query_row(
            "SELECT display_name FROM models WHERE id = ?1",
            rusqlite::params![mid],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "Unknown".into())
    };

    let model_a_name = resolve_model_name(model_a_id);
    let model_b_name = if mode == "arena" {
        resolve_model_name(model_b_id)
    } else {
        String::new()
    };

    let model_names = if mode == "arena" {
        format!("{model_a_name} vs {model_b_name}")
    } else {
        model_a_name.clone()
    };

    // 3. Load rounds
    let rounds: Vec<RoundTranscript> = {
        let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT round_number, speaker, phase, content FROM rounds
                 WHERE debate_id = ?1 ORDER BY round_number, id",
            )
            .map_err(|e| format!("query error: {e}"))?;
        let rows = stmt
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
        rows
    };

    // 4. Try loading scorecard for sparring debates
    let scorecard = if mode == "sparring" {
        debate::get_scorecard(debate_id).await.ok().flatten()
    } else {
        None
    };

    // 5. Build Markdown
    let mut md = String::new();

    md.push_str(&format!("# Debate: {topic}\n\n"));

    let result_str = match winner.as_deref() {
        Some("human") => "Human wins".to_string(),
        Some("model_a") => format!("{model_a_name} wins"),
        Some("model_b") => format!("{model_b_name} wins"),
        Some("draw") => "Draw".to_string(),
        _ => "In progress".to_string(),
    };

    md.push_str(&format!(
        "**Date:** {created_at} | **Mode:** {mode} | **Models:** {model_names}"
    ));
    if let Some(ref side) = human_side {
        md.push_str(&format!(" | **Your Side:** {}", side.to_uppercase()));
    }
    md.push_str(&format!(" | **Result:** {result_str}\n\n---\n\n"));

    // Group rounds by round_number to build sections
    let mut current_round = 0i32;
    for r in &rounds {
        if r.round_number != current_round {
            current_round = r.round_number;
            let phase_label = {
                let p = r.phase.as_str();
                match p {
                    "opening" => "Opening",
                    "rebuttal" => "Rebuttal",
                    "closing" => "Closing",
                    _ => "Argument",
                }
            };
            md.push_str(&format!("## Round {current_round} — {phase_label}\n\n"));
        }

        let speaker_label = match r.speaker.as_str() {
            "human" => format!("You ({})", human_side.as_deref().unwrap_or("?").to_uppercase()),
            "model_a" => {
                if mode == "arena" {
                    format!("PRO ({model_a_name})")
                } else {
                    format!("AI ({model_a_name})")
                }
            }
            "model_b" => format!("CON ({model_b_name})"),
            other => other.to_string(),
        };

        md.push_str(&format!("### {speaker_label}\n\n{}\n\n", r.content));
    }

    // 6. Append scorecard if available
    if let Some(sc) = scorecard {
        md.push_str("---\n\n## Scorecard\n\n");
        md.push_str("| Dimension | You | AI |\n");
        md.push_str("|---|---|---|\n");
        md.push_str(&format!(
            "| Persuasiveness | {} | {} |\n",
            sc.human_persuasiveness, sc.ai_persuasiveness
        ));
        md.push_str(&format!(
            "| Evidence | {} | {} |\n",
            sc.human_evidence, sc.ai_evidence
        ));
        md.push_str(&format!(
            "| Coherence | {} | {} |\n",
            sc.human_coherence, sc.ai_coherence
        ));
        md.push_str(&format!(
            "| Rebuttal | {} | {} |\n",
            sc.human_rebuttal, sc.ai_rebuttal
        ));
        md.push('\n');
        if !sc.strongest_human_point.is_empty() {
            md.push_str(&format!("**Strongest point:** {}\n\n", sc.strongest_human_point));
        }
        if !sc.weakest_human_point.is_empty() {
            md.push_str(&format!("**Weakest point:** {}\n\n", sc.weakest_human_point));
        }
        if !sc.missed_argument.is_empty() {
            md.push_str(&format!("**Missed argument:** {}\n\n", sc.missed_argument));
        }
        if !sc.improvement_tip.is_empty() {
            md.push_str(&format!("**Tip:** {}\n\n", sc.improvement_tip));
        }
    }

    Ok(md)
}

#[derive(Debug, Serialize, Clone)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[tauri::command]
async fn get_settings() -> Result<Vec<Setting>, String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    let mut stmt = conn
        .prepare("SELECT key, value FROM settings ORDER BY key")
        .map_err(|e| format!("query error: {e}"))?;
    let settings = stmt
        .query_map([], |row| {
            Ok(Setting {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        })
        .map_err(|e| format!("query error: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("row error: {e}"))?;
    Ok(settings)
}

#[tauri::command]
async fn update_setting(key: String, value: String) -> Result<(), String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = ?2",
        rusqlite::params![key, value],
    )
    .map_err(|e| format!("update setting error: {e}"))?;
    Ok(())
}

#[tauri::command]
async fn reset_elo_ratings() -> Result<(), String> {
    let conn = db::get_db().lock().map_err(|e| format!("db lock: {e}"))?;
    conn.execute_batch(
        "UPDATE models SET elo_rating = 1500.0, arena_wins = 0, arena_losses = 0, arena_draws = 0, total_debates = 0;
         DELETE FROM elo_history;
         UPDATE user_stats SET elo_rating = 1500.0, total_debates = 0, wins = 0, losses = 0, draws = 0 WHERE id = 1;",
    )
    .map_err(|e| format!("reset elo error: {e}"))?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    db::init_db().expect("failed to initialize database");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(debate::ActiveDebates(Arc::new(Mutex::new(HashMap::new()))))
        .manage(debate::ActiveSparrings(Arc::new(Mutex::new(HashMap::new()))))
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
            debate::start_sparring,
            debate::submit_human_argument,
            debate::abort_sparring,
            debate::request_scorecard,
            debate::get_scorecard,
            get_user_stats,
            export_debate_transcript,
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
            get_settings,
            update_setting,
            reset_elo_ratings,
            debate::suggest_topics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
