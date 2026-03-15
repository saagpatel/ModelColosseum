use rusqlite::{Connection, Result as SqlResult};
use std::path::PathBuf;
use std::sync::Mutex;

static DB: std::sync::OnceLock<Mutex<Connection>> = std::sync::OnceLock::new();

fn db_path() -> PathBuf {
    let home = dirs::home_dir().expect("could not resolve home directory");
    home.join(".model-colosseum")
}

pub fn init_db() -> SqlResult<()> {
    let dir = db_path();
    std::fs::create_dir_all(&dir).expect("could not create data directory");
    let path = dir.join("colosseum.db");

    let conn = Connection::open(&path)?;

    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;

    conn.execute_batch(SCHEMA)?;
    seed_defaults(&conn)?;

    DB.set(Mutex::new(conn))
        .map_err(|_| rusqlite::Error::InvalidParameterName("DB already initialized".into()))?;

    Ok(())
}

pub fn get_db() -> &'static Mutex<Connection> {
    DB.get()
        .expect("database not initialized — call init_db() first")
}

fn seed_defaults(conn: &Connection) -> SqlResult<()> {
    // Insert default settings only if table is empty
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM settings", [], |r| r.get(0))?;
    if count == 0 {
        conn.execute_batch(
            "INSERT INTO settings (key, value) VALUES
                ('default_rounds', '5'),
                ('default_word_limit', '300'),
                ('concurrent_streaming', 'true'),
                ('concurrent_max_params_billions', '40'),
                ('theme', 'dark'),
                ('elo_k_factor_initial', '40'),
                ('elo_k_factor_standard', '32'),
                ('elo_k_factor_veteran', '24'),
                ('elo_k_transition_games', '10'),
                ('elo_k_veteran_games', '30');",
        )?;
    }

    // Insert default user_stats row if missing
    conn.execute_batch(
        "INSERT OR IGNORE INTO user_stats (id, elo_rating, total_debates, wins, losses, draws)
         VALUES (1, 1500.0, 0, 0, 0, 0);",
    )?;

    Ok(())
}

const SCHEMA: &str = "
-- ============================================================
-- CORE TABLES
-- ============================================================

CREATE TABLE IF NOT EXISTS models (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    parameter_count INTEGER,
    quantization TEXT,
    family TEXT,
    elo_rating REAL NOT NULL DEFAULT 1500.0,
    arena_wins INTEGER NOT NULL DEFAULT 0,
    arena_losses INTEGER NOT NULL DEFAULT 0,
    arena_draws INTEGER NOT NULL DEFAULT 0,
    total_debates INTEGER NOT NULL DEFAULT 0,
    first_seen_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_used_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_models_elo ON models(elo_rating DESC);

-- ============================================================
-- ARENA + SPARRING TABLES
-- ============================================================

CREATE TABLE IF NOT EXISTS debates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    topic TEXT NOT NULL,
    mode TEXT NOT NULL CHECK(mode IN ('arena', 'sparring')),
    debate_format TEXT NOT NULL DEFAULT 'freestyle'
        CHECK(debate_format IN ('freestyle', 'formal', 'socratic')),
    model_a_id INTEGER REFERENCES models(id),
    model_b_id INTEGER REFERENCES models(id),
    human_side TEXT CHECK(human_side IN ('pro', 'con')),
    total_rounds INTEGER NOT NULL DEFAULT 5,
    winner TEXT CHECK(winner IN ('model_a', 'model_b', 'human', 'draw')),
    judge_model_id INTEGER REFERENCES models(id),
    status TEXT NOT NULL DEFAULT 'in_progress'
        CHECK(status IN ('in_progress', 'voting', 'completed', 'abandoned')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_debates_status ON debates(status);
CREATE INDEX IF NOT EXISTS idx_debates_created ON debates(created_at DESC);

CREATE TABLE IF NOT EXISTS rounds (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    debate_id INTEGER NOT NULL REFERENCES debates(id) ON DELETE CASCADE,
    round_number INTEGER NOT NULL,
    speaker TEXT NOT NULL CHECK(speaker IN ('model_a', 'model_b', 'human')),
    phase TEXT NOT NULL DEFAULT 'argument'
        CHECK(phase IN ('opening', 'argument', 'rebuttal', 'cross_exam', 'closing')),
    content TEXT NOT NULL,
    tokens_generated INTEGER,
    time_to_first_token_ms INTEGER,
    generation_time_ms INTEGER,
    tokens_per_second REAL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_rounds_debate ON rounds(debate_id, round_number);

CREATE TABLE IF NOT EXISTS elo_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    model_id INTEGER NOT NULL REFERENCES models(id),
    debate_id INTEGER NOT NULL REFERENCES debates(id),
    rating_before REAL NOT NULL,
    rating_after REAL NOT NULL,
    k_factor REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_elo_history_model ON elo_history(model_id, created_at DESC);

CREATE TABLE IF NOT EXISTS debate_tags (
    debate_id INTEGER NOT NULL REFERENCES debates(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    PRIMARY KEY (debate_id, tag)
);

-- ============================================================
-- BENCHMARK TABLES
-- ============================================================

CREATE TABLE IF NOT EXISTS test_suites (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS prompts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    suite_id INTEGER NOT NULL REFERENCES test_suites(id) ON DELETE CASCADE,
    category TEXT NOT NULL
        CHECK(category IN ('coding', 'creative', 'analysis', 'summarization', 'conversation', 'instruction', 'reasoning')),
    title TEXT NOT NULL,
    text TEXT NOT NULL,
    system_prompt TEXT,
    ideal_answer TEXT,
    eval_criteria TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_prompts_suite ON prompts(suite_id, category, sort_order);

CREATE TABLE IF NOT EXISTS benchmark_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    suite_id INTEGER NOT NULL REFERENCES test_suites(id),
    status TEXT NOT NULL DEFAULT 'running'
        CHECK(status IN ('running', 'completed', 'cancelled')),
    notes TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

CREATE TABLE IF NOT EXISTS benchmark_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id INTEGER NOT NULL REFERENCES benchmark_runs(id) ON DELETE CASCADE,
    prompt_id INTEGER NOT NULL REFERENCES prompts(id),
    model_id INTEGER NOT NULL REFERENCES models(id),
    output TEXT NOT NULL,
    tokens_generated INTEGER NOT NULL,
    time_to_first_token_ms INTEGER,
    total_time_ms INTEGER NOT NULL,
    tokens_per_second REAL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_results_run ON benchmark_results(run_id, model_id);
CREATE INDEX IF NOT EXISTS idx_results_prompt ON benchmark_results(prompt_id, model_id);

CREATE TABLE IF NOT EXISTS benchmark_scores (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    result_id INTEGER NOT NULL REFERENCES benchmark_results(id) ON DELETE CASCADE,
    score INTEGER NOT NULL CHECK(score BETWEEN 1 AND 10),
    scoring_method TEXT NOT NULL CHECK(scoring_method IN ('manual', 'auto_judge', 'head_to_head')),
    judge_model_id INTEGER REFERENCES models(id),
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_scores_result ON benchmark_scores(result_id);

-- ============================================================
-- SPARRING-SPECIFIC TABLES
-- ============================================================

CREATE TABLE IF NOT EXISTS sparring_scorecards (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    debate_id INTEGER NOT NULL UNIQUE REFERENCES debates(id) ON DELETE CASCADE,
    human_persuasiveness INTEGER CHECK(human_persuasiveness BETWEEN 1 AND 10),
    human_evidence INTEGER CHECK(human_evidence BETWEEN 1 AND 10),
    human_coherence INTEGER CHECK(human_coherence BETWEEN 1 AND 10),
    human_rebuttal INTEGER CHECK(human_rebuttal BETWEEN 1 AND 10),
    ai_persuasiveness INTEGER CHECK(ai_persuasiveness BETWEEN 1 AND 10),
    ai_evidence INTEGER CHECK(ai_evidence BETWEEN 1 AND 10),
    ai_coherence INTEGER CHECK(ai_coherence BETWEEN 1 AND 10),
    ai_rebuttal INTEGER CHECK(ai_rebuttal BETWEEN 1 AND 10),
    strongest_human_point TEXT,
    weakest_human_point TEXT,
    missed_argument TEXT,
    improvement_tip TEXT,
    raw_judge_output TEXT
);

CREATE TABLE IF NOT EXISTS user_stats (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    elo_rating REAL NOT NULL DEFAULT 1500.0,
    total_debates INTEGER NOT NULL DEFAULT 0,
    wins INTEGER NOT NULL DEFAULT 0,
    losses INTEGER NOT NULL DEFAULT 0,
    draws INTEGER NOT NULL DEFAULT 0
);

-- ============================================================
-- SETTINGS
-- ============================================================

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
";
