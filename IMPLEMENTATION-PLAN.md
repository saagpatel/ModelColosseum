# Model Colosseum — Implementation Plan

## 1. EXEC SUMMARY

### What We're Building
A local-first Tauri 2.0 desktop app that provides three modes for evaluating Ollama models: **Arena** (two models debate a topic while you judge), **Benchmark** (run custom test suites across all models with automated + manual scoring), and **Sparring Ring** (you debate against a model in structured rounds). All modes feed into a unified Elo leaderboard backed by SQLite, giving you definitive answers about which model performs best for which task type — on YOUR hardware, against YOUR use cases.

### Riskiest Parts & De-risking Strategy

**1. MEDIUM — Concurrent Ollama Streaming (Two Models Simultaneously)**
- **Why risky:** Ollama serves one model at a time by default. Running two concurrent `/api/generate` calls may queue the second request, cause memory pressure with two 32B models loaded, or produce interleaved token delivery that's hard to render smoothly.
- **Mitigation:** Test Ollama's `OLLAMA_NUM_PARALLEL` environment variable (set to 2). If two large models exceed VRAM, implement automatic fallback: detect model sizes via `/api/show`, and if combined parameter count > 40B, switch to sequential mode with a "sequential fallback" toast notification.
- **Fallback:** Sequential generation with a visual "waiting for opponent" indicator. Still dramatic — Model B's panel shows a thinking animation while Model A streams.

**2. MEDIUM — LLM Debate Quality (Models Agreeing Instead of Arguing)**
- **Why risky:** LLMs are RLHF-trained to be agreeable. They may refuse to argue, concede all points, or produce generic rebuttals that don't engage with the opponent's specific claims.
- **Mitigation:** Aggressive system prompts with explicit instructions: "You MUST disagree. Quote your opponent's exact words and explain why they're wrong. Never concede a point without immediately pivoting to a stronger counterargument." Include few-shot examples in the system prompt showing what a good rebuttal looks like vs. a bad one.
- **Fallback:** Add a "debate quality" check after round 1 — if both models are agreeing, inject a stronger adversarial system prompt mid-debate: "The judges have noted you're being too agreeable. Your opponent is winning. Fight harder."

**3. LOW — Elo Rating Stability with Small Sample Sizes**
- **Why risky:** After 5 debates, Elo ratings are statistically noisy. Users may draw wrong conclusions from early ratings.
- **Mitigation:** Display "Provisional" badge until a model has 10+ rated debates. Show confidence intervals. Use K-factor decay: K=40 for first 10 debates, K=32 for 10-30, K=24 for 30+.
- **Fallback:** N/A — this is a display/UX concern, not a technical blocker.

**4. LOW — Tauri 2.0 + SQLite Plugin Maturity**
- **Why risky:** Tauri 2.0 is relatively new. The `tauri-plugin-sql` may have edge cases with concurrent writes or migration handling.
- **Mitigation:** Use WAL mode for SQLite. Wrap all writes in explicit transactions via Rust commands. Test concurrent reads during streaming.
- **Fallback:** If the SQL plugin is flaky, use a direct `rusqlite` integration via Tauri commands instead of the plugin abstraction.

**5. LOW — Benchmark Suite Run Times**
- **Why risky:** 10 prompts × 6 models × 30s avg = 30 min per full benchmark run. User may lose patience or the app may appear frozen.
- **Mitigation:** Real-time progress bar with streaming output per prompt. Allow partial runs (select specific categories or models). Show estimated time remaining based on tokens/second from completed runs.
- **Fallback:** Add "Quick Benchmark" preset that runs 3 prompts per category instead of full suite.

### Shortest Path to Daily Use
Ship **Phase 1 (Arena MVP)** by end of Week 2. This gives you model-vs-model debates with streaming, voting, and an Elo leaderboard — solves ~50% of the "which model is better?" question through adversarial comparison. Add **Phase 2 (Benchmark)** by Week 4 for systematic evaluation — that's 80% of the value. **Phase 3 (Sparring Ring)** by Week 5-6 completes the app.

---

## 2. SPEC LOCK

### Goal
A single desktop app where you can definitively rank your local Ollama models across debate quality, task performance, and adversarial reasoning — replacing manual A/B testing with structured, persistent, quantified evaluation.

### Success Metrics
1. Arena debate completes 5 rounds between two models in < 3 minutes (sequential) or < 2 minutes (concurrent) for 14B parameter models
2. Benchmark suite of 15 prompts × 6 models completes in < 20 minutes with real-time progress
3. Elo ratings stabilize (< 50 point variance between runs) after 15+ debates per model
4. Cold app launch to first interaction < 2 seconds
5. SQLite database handles 1,000+ debate transcripts without query degradation (< 100ms for leaderboard)

### Hard Constraints
- Local-only. No cloud APIs. No data leaves the machine.
- Ollama REST API (localhost:11434) is the only LLM interface
- macOS-only (M4 Pro + M3 targets). No Windows/Linux for MVP.
- All data persisted in SQLite. No external databases.
- Tauri 2.0 (not 1.x). React 19. TypeScript strict mode.
- Dark theme only for MVP.

### Locked Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Concurrent vs Sequential streaming | Concurrent with automatic sequential fallback | User explicitly chose concurrent. Fallback prevents OOM with large models. |
| Round default | 5 rounds | Good balance of depth vs time. Configurable 3/5/7. |
| Elo starting rating | 1500, K-factor 32 with decay | Standard chess Elo. K-decay prevents volatile late-game swings. |
| Arena debate format | Free-form by default, structured modes as Phase 3 feature | Free-form is simpler to implement and more entertaining. Structured (opening/rebuttal/closing) adds rigidity without proportional value in MVP. |
| Benchmark scoring scale | 1-5 manual, 1-10 auto-judge (normalized to 1-5) | 1-5 is fast for manual review. Auto-judge gets more granularity internally. |
| Judge model selection | User picks judge model. Cannot judge own output. | Auto-selecting "best" model is opinionated. Let user decide. Enforce no self-judging. |
| Sparring Ring deployment | Tauri desktop (not web) | User chose Tauri-only. Shareability via transcript export, not live URLs. |
| Database | SQLite via rusqlite in Tauri Rust backend, NOT tauri-plugin-sql | Direct rusqlite gives more control over migrations, WAL mode, and concurrent access. Exposed via Tauri commands. |
| System prompt storage | Hardcoded defaults with user-editable overrides stored in SQLite settings table | System prompts are critical to debate quality. Defaults must be carefully tuned. User overrides for power users. |
| Model metadata | Fetched from Ollama `/api/show` on first use, cached in SQLite | Parameter count, quantization level, family — needed for leaderboard display and concurrent streaming decisions. |

---

## 3. ARCHITECTURE

### System Diagram

```
┌─────────────────────────────────────────────────────────┐
│                    React 19 Frontend                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐ │
│  │  Arena    │  │Benchmark │  │ Sparring  │  │Leaderbd │ │
│  │  View     │  │  View    │  │  Ring     │  │  View   │ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬────┘ │
│       │              │              │              │      │
│  ┌────┴──────────────┴──────────────┴──────────────┴───┐ │
│  │              Shared Services Layer                   │ │
│  │  OllamaService │ DebateEngine │ EloService │ Store  │ │
│  └───────────────────────┬─────────────────────────────┘ │
└──────────────────────────┼───────────────────────────────┘
                           │ Tauri IPC (invoke)
┌──────────────────────────┼───────────────────────────────┐
│                   Tauri Rust Backend                      │
│  ┌───────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │ ollama_client  │  │  db (rusqlite)│  │  system_info  │  │
│  │ - list_models  │  │  - migrations │  │  - cpu/mem    │  │
│  │ - generate     │  │  - queries    │  │  - gpu usage  │  │
│  │ - show_model   │  │  - WAL mode   │  │               │  │
│  │ - stream       │  │              │  │               │  │
│  └───────┬───────┘  └──────┬───────┘  └───────────────┘  │
└──────────┼──────────────────┼────────────────────────────┘
           │                  │
    ┌──────┴──────┐    ┌──────┴──────┐
    │   Ollama    │    │   SQLite    │
    │ :11434 REST │    │  colosseum  │
    │             │    │    .db      │
    └─────────────┘    └─────────────┘
```

### Data Flow — Arena Debate

```
User sets topic + picks Model A & Model B
  → Frontend dispatches "start_debate" Tauri command
  → Rust backend creates debate record in SQLite
  → Round loop (1..N):
      ├─ Spawn two concurrent tasks:
      │   ├─ Task 1: POST /api/generate (Model A, full history) → stream tokens via Tauri event "debate:stream:a"
      │   └─ Task 2: POST /api/generate (Model B, full history) → stream tokens via Tauri event "debate:stream:b"
      ├─ Frontend renders both streams in split-pane simultaneously
      ├─ Both tasks complete → save round content to SQLite
      └─ Emit "debate:round_complete" event
  → All rounds done → show vote UI
  → User votes → Rust command calculates Elo → updates SQLite
  → Frontend refreshes leaderboard
```

### Data Flow — Benchmark Run

```
User selects test suite + models
  → Frontend dispatches "start_benchmark" Tauri command
  → Rust backend creates benchmark_run record
  → For each model (sequential — one model at a time to avoid memory contention):
      For each prompt in suite:
        ├─ POST /api/generate with prompt + system_prompt
        ├─ Measure: time_to_first_token, total_time, tokens_generated
        ├─ Stream tokens via Tauri event "benchmark:stream:{model}:{prompt_id}"
        ├─ Save result to SQLite
        └─ Emit "benchmark:progress" with completion percentage
  → All models done → present results grid for scoring
  → User scores (or auto-judge scores) → save to SQLite
  → Aggregate → update leaderboard
```

### Data Flow — Sparring Ring

```
User enters topic + picks side (FOR/AGAINST) + selects opponent model
  → Rust backend creates debate record (model_b = null, is_sparring = true)
  → Phase loop (Opening → Rebuttal 1 → Rebuttal 2 → Closing):
      ├─ User types their argument → save to rounds table
      ├─ Send full transcript + AI system prompt to opponent model
      ├─ Stream AI response via Tauri event "sparring:stream"
      ├─ Save AI response to rounds table
      └─ Advance to next phase
  → All phases complete → Switch to Judge persona
  → Send full transcript to judge model → stream scorecard
  → Parse scores → save to SQLite
  → Update user's personal Elo
```

### Tech Stack

| Component | Technology | Version | Justification |
|-----------|-----------|---------|---------------|
| App Shell | Tauri | 2.x (latest stable) | Local-first desktop, Rust backend for performance, small binary |
| Frontend Framework | React | 19.x | Hooks-based, concurrent features for streaming UI |
| Language | TypeScript | 5.x | Strict mode. No `any`. |
| Build Tool | Vite | 6.x | Fast HMR, native ESM, Tauri integration via `@tauri-apps/vite-plugin` |
| Styling | Tailwind CSS | 4.x | Utility-first, dark theme, rapid prototyping |
| Charts | Recharts | 2.x | React-native charts for radar, bar, sparklines. Familiar API. |
| Database | SQLite via rusqlite | 0.31+ | Direct Rust integration, WAL mode, full SQL control |
| LLM Interface | Ollama REST API | v1 | `/api/generate` (streaming), `/api/tags` (model list), `/api/show` (metadata) |
| HTTP Client (Rust) | reqwest | 0.12+ | Async streaming support for Ollama API |
| System Info | sysinfo crate | 0.31+ | CPU/memory monitoring during benchmarks |
| Serialization | serde + serde_json | 1.x | Rust ↔ Frontend data exchange |
| Async Runtime | tokio | 1.x | Required by reqwest, powers concurrent streaming |
| State Management | Zustand | 5.x | Minimal, TypeScript-first, no boilerplate |
| Routing | React Router | 7.x | Tab-based navigation between modes |

### Data Model

```sql
-- ============================================================
-- CORE TABLES
-- ============================================================

-- Cached model metadata from Ollama
CREATE TABLE models (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,           -- e.g., "qwen3:32b"
    display_name TEXT NOT NULL,          -- e.g., "Qwen3 32B"
    parameter_count INTEGER,            -- billions, from /api/show
    quantization TEXT,                  -- e.g., "Q4_K_M"
    family TEXT,                        -- e.g., "qwen3"
    elo_rating REAL NOT NULL DEFAULT 1500.0,
    arena_wins INTEGER NOT NULL DEFAULT 0,
    arena_losses INTEGER NOT NULL DEFAULT 0,
    arena_draws INTEGER NOT NULL DEFAULT 0,
    total_debates INTEGER NOT NULL DEFAULT 0,
    first_seen_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_used_at TEXT,
    UNIQUE(name)
);
CREATE INDEX idx_models_elo ON models(elo_rating DESC);

-- ============================================================
-- ARENA + SPARRING TABLES
-- ============================================================

-- A debate session (Arena: model vs model, Sparring: human vs model)
CREATE TABLE debates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    topic TEXT NOT NULL,
    mode TEXT NOT NULL CHECK(mode IN ('arena', 'sparring')),
    debate_format TEXT NOT NULL DEFAULT 'freestyle'
        CHECK(debate_format IN ('freestyle', 'formal', 'socratic')),
    model_a_id INTEGER REFERENCES models(id),      -- In sparring: the AI opponent
    model_b_id INTEGER REFERENCES models(id),      -- In sparring: NULL
    human_side TEXT CHECK(human_side IN ('pro', 'con')), -- Sparring only
    total_rounds INTEGER NOT NULL DEFAULT 5,
    winner TEXT CHECK(winner IN ('model_a', 'model_b', 'human', 'draw')),
    judge_model_id INTEGER REFERENCES models(id),   -- If auto-judged
    status TEXT NOT NULL DEFAULT 'in_progress'
        CHECK(status IN ('in_progress', 'voting', 'completed', 'abandoned')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);
CREATE INDEX idx_debates_status ON debates(status);
CREATE INDEX idx_debates_created ON debates(created_at DESC);

-- Individual rounds within a debate
CREATE TABLE rounds (
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
CREATE INDEX idx_rounds_debate ON rounds(debate_id, round_number);

-- Elo history for tracking rating changes over time
CREATE TABLE elo_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    model_id INTEGER NOT NULL REFERENCES models(id),
    debate_id INTEGER NOT NULL REFERENCES debates(id),
    rating_before REAL NOT NULL,
    rating_after REAL NOT NULL,
    k_factor REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_elo_history_model ON elo_history(model_id, created_at DESC);

-- Tags for categorizing debates
CREATE TABLE debate_tags (
    debate_id INTEGER NOT NULL REFERENCES debates(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    PRIMARY KEY (debate_id, tag)
);

-- ============================================================
-- BENCHMARK TABLES
-- ============================================================

-- A collection of test prompts
CREATE TABLE test_suites (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Individual test prompts within a suite
CREATE TABLE prompts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    suite_id INTEGER NOT NULL REFERENCES test_suites(id) ON DELETE CASCADE,
    category TEXT NOT NULL
        CHECK(category IN ('coding', 'creative', 'analysis', 'summarization', 'conversation', 'instruction', 'reasoning')),
    title TEXT NOT NULL,                 -- Short label for display
    text TEXT NOT NULL,                  -- The actual prompt
    system_prompt TEXT,                  -- Optional system prompt override
    ideal_answer TEXT,                   -- Optional reference answer
    eval_criteria TEXT,                  -- What to look for when scoring
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_prompts_suite ON prompts(suite_id, category, sort_order);

-- A single benchmark execution
CREATE TABLE benchmark_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    suite_id INTEGER NOT NULL REFERENCES test_suites(id),
    status TEXT NOT NULL DEFAULT 'running'
        CHECK(status IN ('running', 'completed', 'cancelled')),
    notes TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

-- Per-model, per-prompt results
CREATE TABLE benchmark_results (
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
CREATE INDEX idx_results_run ON benchmark_results(run_id, model_id);
CREATE INDEX idx_results_prompt ON benchmark_results(prompt_id, model_id);

-- Scores for benchmark results
CREATE TABLE benchmark_scores (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    result_id INTEGER NOT NULL REFERENCES benchmark_results(id) ON DELETE CASCADE,
    score INTEGER NOT NULL CHECK(score BETWEEN 1 AND 10),
    scoring_method TEXT NOT NULL CHECK(scoring_method IN ('manual', 'auto_judge', 'head_to_head')),
    judge_model_id INTEGER REFERENCES models(id),
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_scores_result ON benchmark_scores(result_id);

-- ============================================================
-- SPARRING-SPECIFIC TABLES
-- ============================================================

-- Post-debate scorecards from judge
CREATE TABLE sparring_scorecards (
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
    raw_judge_output TEXT              -- Full judge response for reference
);

-- User's personal debate Elo (Sparring Ring)
CREATE TABLE user_stats (
    id INTEGER PRIMARY KEY CHECK(id = 1),  -- Single row
    elo_rating REAL NOT NULL DEFAULT 1500.0,
    total_debates INTEGER NOT NULL DEFAULT 0,
    wins INTEGER NOT NULL DEFAULT 0,
    losses INTEGER NOT NULL DEFAULT 0,
    draws INTEGER NOT NULL DEFAULT 0
);

-- ============================================================
-- SETTINGS
-- ============================================================

CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Default settings inserted on first run
INSERT INTO settings (key, value) VALUES
    ('default_rounds', '5'),
    ('default_word_limit', '300'),
    ('concurrent_streaming', 'true'),
    ('concurrent_max_params_billions', '40'),
    ('theme', 'dark'),
    ('elo_k_factor_initial', '40'),
    ('elo_k_factor_standard', '32'),
    ('elo_k_factor_veteran', '24'),
    ('elo_k_transition_games', '10'),
    ('elo_k_veteran_games', '30');
```

### API Contracts — Ollama REST API

**List Models**
```
GET http://localhost:11434/api/tags
Response: { "models": [{ "name": "qwen3:32b", "size": 19876543210, "details": { "parameter_size": "32B", "quantization_level": "Q4_K_M", "family": "qwen3" } }] }
Rate limit: None (local)
```

**Show Model Metadata**
```
POST http://localhost:11434/api/show
Body: { "name": "qwen3:32b" }
Response: { "modelfile": "...", "parameters": "...", "template": "...", "details": { "parameter_size": "32B", "quantization_level": "Q4_K_M", "family": "qwen3", "families": ["qwen3"] } }
Rate limit: None (local)
```

**Generate (Streaming)**
```
POST http://localhost:11434/api/generate
Body: {
  "model": "qwen3:32b",
  "prompt": "...",
  "system": "...",
  "stream": true,
  "options": { "num_predict": 1024, "temperature": 0.7 }
}
Response: NDJSON stream, each line: { "model": "...", "response": "token", "done": false }
Final line: { "model": "...", "response": "", "done": true, "total_duration": 12345678, "eval_count": 150, "eval_duration": 10000000 }
Rate limit: None, but concurrent requests may queue depending on OLLAMA_NUM_PARALLEL
```

**Check Ollama Health**
```
GET http://localhost:11434/
Response: "Ollama is running" (200 OK)
```

### Internal Tauri Command API

All frontend ↔ backend communication via `@tauri-apps/api/core invoke()`.

```typescript
// Model management
invoke('list_models') → Model[]
invoke('refresh_models') → Model[]  // Re-fetch from Ollama, update cache
invoke('get_model', { id: number }) → Model

// Arena
invoke('start_debate', { topic: string, modelAId: number, modelBId: number, rounds: number, format: string }) → Debate
invoke('vote_debate', { debateId: number, winner: 'model_a' | 'model_b' | 'draw' }) → { debate: Debate, eloChanges: EloChange[] }
invoke('auto_judge_debate', { debateId: number, judgeModelId: number }) → JudgeResult

// Benchmark
invoke('list_test_suites') → TestSuite[]
invoke('create_test_suite', { name: string, description: string }) → TestSuite
invoke('add_prompt', { suiteId: number, prompt: PromptInput }) → Prompt
invoke('start_benchmark', { suiteId: number, modelIds: number[] }) → BenchmarkRun
invoke('cancel_benchmark', { runId: number }) → void
invoke('score_result', { resultId: number, score: number, method: string }) → void
invoke('auto_judge_benchmark', { runId: number, judgeModelId: number }) → void

// Sparring
invoke('start_sparring', { topic: string, humanSide: 'pro' | 'con', opponentModelId: number, judgeModelId: number }) → Debate
invoke('submit_human_argument', { debateId: number, content: string }) → void
invoke('request_scorecard', { debateId: number }) → SparringScorecard

// Leaderboard
invoke('get_leaderboard', { mode?: string, category?: string }) → LeaderboardEntry[]
invoke('get_model_history', { modelId: number }) → EloHistory[]
invoke('get_user_stats') → UserStats

// Settings
invoke('get_settings') → Settings
invoke('update_setting', { key: string, value: string }) → void

// Streaming events (Tauri events, not commands)
listen('debate:stream:a', (event: { token: string, done: boolean }) => void)
listen('debate:stream:b', (event: { token: string, done: boolean }) => void)
listen('sparring:stream', (event: { token: string, done: boolean }) => void)
listen('benchmark:stream', (event: { modelId: number, promptId: number, token: string, done: boolean }) => void)
listen('benchmark:progress', (event: { completed: number, total: number, currentModel: string, currentPrompt: string }) => void)
```

---

## 4. PHASED IMPLEMENTATION

### Phase 0: Foundation (Week 1)
**Objectives:**
- Tauri 2.0 project scaffold with React 19 + TypeScript + Vite
- SQLite database with full schema and migrations
- Ollama client in Rust with streaming support
- Model list fetching and caching

**Week 1 Tasks:**

1. **Scaffold Tauri 2.0 project** — `npm create tauri-app@latest model-colosseum -- --template react-ts`. Configure Vite, install Tailwind 4, Zustand, React Router, Recharts. Verify `cargo tauri dev` launches.
   - AC: App window opens with "Hello World" React page. Tailwind classes render. Hot reload works.

2. **Set up SQLite with rusqlite** — Add `rusqlite` with `bundled` feature to Cargo.toml. Create `src-tauri/src/db.rs` module. Implement `init_db()` that creates the database file at `~/.model-colosseum/colosseum.db`, enables WAL mode, and runs all CREATE TABLE statements from the data model above. Implement `get_db()` singleton pattern using `std::sync::Mutex<Connection>`.
   - AC: On app launch, database file exists at expected path. All tables visible via `sqlite3` CLI. WAL mode confirmed with `PRAGMA journal_mode`.

3. **Build Ollama REST client** — Create `src-tauri/src/ollama.rs`. Implement functions: `list_models()` (GET /api/tags), `show_model()` (POST /api/show), `health_check()` (GET /), and `generate_stream()` (POST /api/generate with streaming response). Use `reqwest` with async streaming. `generate_stream()` should return a `tokio::sync::mpsc::Receiver<String>` of tokens.
   - AC: `list_models` Tauri command returns array of installed models. `health_check` returns true when Ollama is running. `generate_stream` successfully streams tokens from a test prompt.

4. **Wire model list to frontend** — Create `list_models` and `refresh_models` Tauri commands. On app launch, fetch models from Ollama, upsert into `models` table (preserving existing Elo ratings), return to frontend. Build a minimal `<ModelSelector>` component that displays models in a dropdown.
   - AC: Dropdown shows all installed Ollama models with parameter count and quantization. New models appear after `refresh`. Previously seen models retain their Elo.

5. **Implement Elo calculation module** — Create `src-tauri/src/elo.rs`. Implement standard Elo: `expected_score(ra, rb) → f64`, `update_ratings(ra, rb, outcome, k) → (new_ra, new_rb)`. K-factor selection based on total debates (K=40 < 10 games, K=32 10-30 games, K=24 > 30 games). Add unit tests.
   - AC: Unit tests pass for all Elo edge cases: equal ratings draw, strong vs weak win, K-factor transitions.

**Success Criteria:**
- [ ] `cargo tauri dev` launches app with React frontend
- [ ] SQLite database created with all tables on first launch
- [ ] Ollama models listed in UI dropdown
- [ ] Streaming tokens received from Ollama in Rust backend
- [ ] Elo calculation unit tests pass

**Risks:**
- Risk: Ollama not running when app launches
  - Mitigation: Health check on startup, show "Ollama not detected" banner with install instructions
  - Fallback: Retry button, link to ollama.com

---

### Phase 1: Arena Mode (Weeks 2-3)
**Objectives:**
- Full debate engine with concurrent streaming
- Split-pane debate viewer with live token rendering
- Vote system with Elo updates
- Basic leaderboard view
- Debate history browser

**Week 2 Tasks:**

1. **Build the Debate Engine in Rust** — Create `src-tauri/src/debate.rs`. Implement `start_debate()` command that: creates debate record, then for each round spawns two `tokio::spawn` tasks that concurrently call `generate_stream()` for Model A and Model B. Each task emits Tauri events (`debate:stream:a` and `debate:stream:b`) per token. After both tasks complete, save round content to `rounds` table. Build the system prompt template with role assignment, debate rules, and full history injection.
   - AC: Start a debate between two models. Both stream simultaneously. Full history is passed to each model per round. All rounds saved to DB.

2. **Implement concurrent streaming with fallback** — Before starting a debate, check both models' parameter counts from the `models` table. If combined > setting `concurrent_max_params_billions` (default 40B), switch to sequential mode: Model A generates fully, then Model B. Emit a `debate:mode` event so the frontend can adapt UI. In sequential mode, Model B's panel shows a "thinking..." animation until Model A completes.
   - AC: Two 14B models stream concurrently. Qwen3 32B + Llama 3.3 70B triggers sequential fallback. UI correctly adapts.

3. **Build Arena UI — Split Pane View** — Create `src/pages/Arena.tsx`. Layout: topic input at top, two side-by-side panels (Model A left, Model B right). Model selectors above each panel. "Start Debate" button. During debate: round counter at top center, streaming text in each panel with typing animation (cursor blink), model name and avatar/icon on each side. VS badge between panels. Dark theme with gold/amber accents for the arena aesthetic.
   - AC: Both panels stream tokens simultaneously with visible typing effect. Round counter advances. Clear visual distinction between sides.

4. **Build system prompt templates** — Create `src-tauri/src/prompts.rs` with `build_debate_system_prompt(role, topic, round, history)`. The prompt must: assign a clear adversarial role, instruct the model to quote and counter the opponent's specific points, set a word limit (default 300), and evolve per round (first round: establish position, later rounds: rebut specific points). Store default prompts in SQLite `settings` with keys like `prompt_arena_pro`, `prompt_arena_con`.
   - AC: Models produce substantive arguments that reference each other's specific points by round 2+. Word limits approximately respected.

**Week 3 Tasks:**

5. **Implement vote system** — After final round, show vote UI: "Left Wins", "Right Wins", "Draw" buttons. `vote_debate` Tauri command updates debate record, calculates Elo changes for both models, writes to `elo_history`, updates `models` table. Return Elo changes to frontend for display ("+15 / -15" animation).
   - AC: Vote recorded. Elo ratings update correctly. Elo history tracked. Win/loss counts increment.

6. **Build Leaderboard view** — Create `src/pages/Leaderboard.tsx`. Table with columns: Rank, Model, Elo, W/L/D, Total Debates, Avg TPS, Last Active. Sortable columns. "Provisional" badge for models with < 10 debates. Elo trend sparkline per model (last 20 ratings from `elo_history`). Filter by mode (Arena only for now, Benchmark and Sparring added later).
   - AC: Leaderboard shows all models sorted by Elo. Sparklines render. Provisional badge appears for < 10 debates.

7. **Build Debate History browser** — Create `src/pages/History.tsx`. List of past debates: topic, models, outcome, date. Click to expand full transcript with round-by-round view. Search by topic. Filter by model, outcome, date range.
   - AC: All past debates browsable. Full transcript viewable. Search and filters work.

8. **Navigation and layout shell** — Create app shell with sidebar or top tab navigation: Arena, Benchmark (disabled, coming soon), Sparring (disabled, coming soon), Leaderboard, History, Settings. Dark theme with colosseum/arena aesthetic: dark gray/slate background, gold/amber accent color, subtle gradients.
   - AC: All tabs navigate correctly. Disabled tabs show "Coming Soon" state. Consistent dark theme throughout.

**Success Criteria:**
- [ ] Two models debate 5 rounds with concurrent streaming
- [ ] Sequential fallback triggers for oversized model pairs
- [ ] Models reference each other's specific points by round 2
- [ ] Vote updates Elo correctly
- [ ] Leaderboard ranks all models with sparklines
- [ ] Debate history searchable and browsable
- [ ] Dark arena theme applied consistently

**Risks:**
- Risk: Concurrent streaming causes token interleaving or dropped events
  - Mitigation: Each stream uses its own Tauri event channel. Frontend buffers tokens per-side independently. Add sequence numbers to events if ordering issues arise.
  - Fallback: Default to sequential if concurrent produces rendering bugs

---

### Phase 2: Benchmark Mode (Weeks 4-5)
**Objectives:**
- Test suite CRUD with category management
- Batch benchmark runner with progress UI
- Manual scoring interface (1-5 per output)
- Auto-judge mode with configurable judge model
- Category-specific leaderboard with radar chart
- Performance metrics table (TPS, TTFT)

**Week 4 Tasks:**

1. **Test suite CRUD** — Create `src/pages/Benchmark.tsx` with sub-views. Build test suite editor: create/rename/delete suites, add/edit/remove prompts with category picker, optional system prompt, ideal answer, and eval criteria fields. Drag-to-reorder prompts. Ship a default "Starter Suite" with 15 prompts across 5 categories tailored to your use cases (see Default Test Suite below).
   - AC: Can create suites, add prompts to all 7 categories, edit/delete. Default suite installed on first launch with 15 prompts.

2. **Batch benchmark runner** — Create `src-tauri/src/benchmark.rs`. `start_benchmark` command iterates models sequentially (to avoid memory contention), prompts within each model. For each: call `generate_stream`, measure TTFT and total time, emit progress events. Save all results to `benchmark_results`. Show real-time progress: overall bar, current model name, current prompt, streaming output preview, ETA based on average completion time.
   - AC: 15 prompts × 3 models completes successfully. Progress bar accurate. ETA within 20% of actual. All results persisted.

3. **Results grid view** — After benchmark completes, show a grid: rows = prompts, columns = models. Each cell shows truncated output with expand button, tokens/second, total time. Highlight fastest model per prompt in green. Click any cell to see full output.
   - AC: Grid renders all results. Fastest model highlighted. Full output viewable on click.

**Week 5 Tasks:**

4. **Manual scoring interface** — In the results grid, add a 1-5 star rating widget per cell. Score persists immediately on click. Show running averages per model in the column header. "Score All" mode: present outputs one-by-one for focused review (blind mode optional — hide model names).
   - AC: Can score every result 1-5. Running averages update in real time. Blind mode hides model names.

5. **Auto-judge mode** — `auto_judge_benchmark` command: for each unscored result, send a judging prompt to the judge model containing the original prompt, the model's output, and eval criteria. Parse a 1-10 score from the response (normalize to 1-5 for display). Never let a model judge its own output. Show auto-judge scores alongside manual scores with clear labeling.
   - AC: Auto-judge scores all results. Scores appear with "Auto" badge. Self-judging prevented. Judge scores visible next to manual scores.

6. **Benchmark leaderboard integration** — Extend leaderboard to include benchmark data: category-specific average scores, radar chart (one polygon per model, axes = categories), performance metrics table (avg TPS, avg TTFT per model). Add "Benchmark" filter to leaderboard tab.
   - AC: Radar chart renders with proper scaling. Category breakdown accurate. Performance table sortable.

7. **Run history and comparison** — List past benchmark runs with date, suite name, models. Click to view results. "Compare Runs" mode: select two runs and see score differences (useful when re-testing after model updates or quantization changes).
   - AC: Past runs browsable. Run comparison shows score deltas highlighted green/red.

**Success Criteria:**
- [ ] Default test suite ships with 15 prompts across categories
- [ ] Full benchmark run completes with progress tracking
- [ ] Manual 1-5 scoring works in grid and focused modes
- [ ] Auto-judge produces reasonable scores, no self-judging
- [ ] Radar chart visualizes category strengths per model
- [ ] Run comparison highlights improvements/regressions

**Risks:**
- Risk: Auto-judge unreliable (verbose bias, inflated scores)
  - Mitigation: Always show manual scores as primary, auto-judge as secondary. Add calibration prompt: "A score of 5 is near-perfect. Most outputs should score 2-4."
  - Fallback: Auto-judge is advisory only, manual scoring is source of truth

---

### Phase 3: Sparring Ring (Weeks 5-6)
**Objectives:**
- Structured debate format (Opening → Rebuttal × 2 → Closing)
- Word limit enforcement with live counter
- AI opponent that engages with user's specific points
- Post-debate scorecard from judge model
- User Elo tracking

**Week 5 Tasks (overlap with Benchmark polish):**

1. **Sparring debate engine** — Extend `debate.rs` to handle sparring mode. Phase progression: Opening Statement → Rebuttal 1 → Rebuttal 2 → Closing Statement. After user submits their argument, send full transcript to opponent model with sparring-specific system prompt. Track phases in the `rounds.phase` column.
   - AC: Full 4-phase debate completes. AI responds to user's specific points. Phase labels correct in transcript.

2. **Sparring Ring UI** — Create `src/pages/SparringRing.tsx`. Topic input + side selector (FOR/AGAINST) + model picker + judge model picker. During debate: left panel = user's text input with word count and limit indicator, right panel = AI streaming response. Phase indicator at top with progress dots. "Submit" button disabled until user types content. Phase name and word limit displayed prominently.
   - AC: User can type arguments, see live word count, submit, and watch AI respond. Phase progression is clear.

**Week 6 Tasks:**

3. **System prompts for sparring** — Build specialized prompts: Opponent persona (adversarial, quotes user's words, exploits weak points), Judge persona (impartial, scores on 4 dimensions, provides specific feedback). Add difficulty levels: Casual (AI concedes minor points, generous scoring), Competitive (AI argues aggressively, fair scoring), Expert (AI brings domain knowledge, strict scoring).
   - AC: Casual AI is noticeably easier than Expert. AI quotes user's specific words at Competitive+ levels.

4. **Post-debate scorecard** — After closing statements, automatically send full transcript to judge model with scorecard prompt. Parse structured scores (Persuasiveness, Evidence, Coherence, Rebuttal — each 1-10 for both sides). Display as a report card with specific feedback text. Save to `sparring_scorecards`.
   - AC: Scorecard displays all 8 scores + 4 text feedback items. Scores saved to DB.

5. **User stats and Elo** — Track user's personal Elo in `user_stats`. Update after each judged sparring match (based on scorecard total). Show user's Elo, W/L/D record, and trend on the Sparring Ring home screen. Add to leaderboard as a special "You" entry.
   - AC: User Elo updates after each sparring session. Trend line visible. "You" appears on leaderboard.

6. **Transcript export** — Export any debate (Arena or Sparring) as Markdown. Format: metadata header, then round-by-round with clear speaker labels and phase indicators. Include scorecard at the end for sparring matches.
   - AC: Exported .md file is clean, readable, and includes all debate content + scores.

**Success Criteria:**
- [ ] 4-phase structured debate completes
- [ ] Word limits enforced with live counter
- [ ] AI quotes and counters user's specific arguments
- [ ] Scorecard parses correctly with all 4 dimensions
- [ ] User Elo tracks across sessions
- [ ] Markdown export works for all debate types

**Risks:**
- Risk: Score parsing fails (model returns unstructured text instead of parseable scores)
  - Mitigation: Use structured output prompt: "Respond ONLY in this exact JSON format: {...}". Regex fallback parser for common failure modes. If parsing fails, show raw judge output and let user manually score.
  - Fallback: Manual score override always available

---

### Phase 4: Polish & Advanced Features (Weeks 7-8)
**Objectives:**
- Arena debate modes (Formal, Socratic)
- Topic suggestions
- Head-to-head blind test for Benchmark
- Hardware metrics during runs
- Settings page
- Visual polish and animations

**Week 7 Tasks:**

1. **Debate modes** — Add Formal Debate (3 phases: Opening Statement, Structured Rebuttal with direct point-by-point response, Closing Summary) and Socratic mode (one model asks probing questions, other defends a position — alternates questioner each round). Mode selector in Arena UI.
   - AC: Both new modes produce qualitatively different debates than Freestyle.

2. **Topic suggestions** — "Suggest a Topic" button that calls a lightweight model (smallest installed) with a meta-prompt: "Generate 5 interesting debate topics across technology, philosophy, geopolitics, sports, and culture. Be specific and debatable, not generic." Parse and display as clickable chips.
   - AC: 5 diverse, specific topics generated. One click loads topic into input.

3. **Head-to-head blind test** — In Benchmark results, add "Blind Compare" mode: for each prompt, show two outputs side-by-side with model names hidden (randomized left/right). User picks the better one. Convert picks to scores. Show results with model reveals.
   - AC: Blind comparison hides model identity. Picks tracked. Reveal shows actual models.

4. **Hardware metrics** — Use `sysinfo` crate to sample CPU%, memory pressure, and swap usage every 2 seconds during benchmark runs. Correlate with generation performance. Show as a live mini-chart during runs and in benchmark results.
   - AC: CPU/memory graph visible during benchmark. Data persisted with run results.

**Week 8 Tasks:**

5. **Settings page** — Create `src/pages/Settings.tsx`. Configurable: default round count, word limits, concurrent streaming toggle, concurrent parameter threshold, system prompt editor (view/edit all default prompts), Ollama URL (default localhost:11434), theme colors. Reset to defaults button.
   - AC: All settings persist across app restarts. System prompt editing works with preview.

6. **Visual polish** — Animate Elo changes (count-up animation). Add VS badge animation on debate start. Winner celebration effect (subtle confetti or glow). Smooth tab transitions. Loading states for all async operations. Empty states for first-launch experience.
   - AC: Animations are smooth (60fps). No jarring transitions. Empty states guide new users.

7. **Import/Export** — Export test suites as JSON. Import test suites from JSON file. Export full leaderboard as CSV. Export individual benchmark runs as Markdown report.
   - AC: Round-trip: export a suite → import in fresh install → all prompts intact.

**Success Criteria:**
- [ ] Formal and Socratic debate modes work
- [ ] Topic suggestions generate varied, specific topics
- [ ] Blind compare eliminates model-name bias
- [ ] Hardware metrics tracked during benchmarks
- [ ] All settings persist and apply correctly
- [ ] Animations smooth, empty states helpful
- [ ] Import/export works for suites and leaderboards

---

## DEFAULT TEST SUITE

Ship with these 15 prompts on first launch. Tailored to YOUR use cases:

### Coding (3 prompts)
1. "Refactor this TypeScript function to be more readable and type-safe: `function processData(d: any) { const r = []; for (let i = 0; i < d.length; i++) { if (d[i].status === 'active' && d[i].score > 50) { r.push({name: d[i].name, score: d[i].score * 1.1}); } } return r.sort((a,b) => b.score - a.score); }`"
2. "Write a Rust function that implements an LRU cache with O(1) get and put operations. Include proper error handling and documentation comments."
3. "Debug this SQL query that's supposed to find users who have placed orders in the last 30 days but haven't logged in: `SELECT u.* FROM users u LEFT JOIN orders o ON u.id = o.user_id WHERE o.created_at > NOW() - INTERVAL 30 DAY AND u.last_login < NOW() - INTERVAL 30 DAY`"

### Creative Writing (3 prompts)
4. "Write a 200-word product description for a hypothetical AI-powered coffee machine that learns your preferences. The tone should be premium but approachable — think Apple's marketing style."
5. "Generate a D&D encounter for 4 level-5 players in a corrupted forest. Include NPC dialogue, environmental hazards, a moral dilemma, and at least one combat option that isn't 'fight the monster.'"
6. "Write opening and closing paragraphs for a blog post titled 'Why Every IT Team Needs an Internal Developer Platform.' The audience is CTOs at mid-size companies."

### Analysis (3 prompts)
7. "A company's monthly active users dropped 23% quarter-over-quarter, but revenue increased 8%. Propose 3 plausible explanations for this divergence and rank them by likelihood."
8. "Compare the security implications of storing API tokens in macOS Keychain vs. environment variables vs. a .env file vs. an encrypted SQLite database. Rank by security, developer experience, and portability."
9. "Analyze this error pattern from a hypothetical log: 500 errors spike every Monday at 9am PST, lasting exactly 15 minutes, affecting only the /api/auth endpoint. What are the most likely root causes?"

### Summarization (3 prompts)
10. "Summarize the following Jira ticket for a weekly stakeholder report (2-3 sentences): 'User reports that SSO login fails intermittently on Chrome 120+ when using Okta FastPass. Reproducible on macOS Sonoma only. CrowdStrike Falcon logs show no blocks. Okta system log shows successful auth but redirect fails. Workaround: clear browser cookies and retry. Affects ~30 users in Engineering division. P2 priority.'"
11. "Distill the key architectural decisions from this paragraph into a bullet list of no more than 5 items: [include a 300-word technical architecture description about microservices migration]"
12. "Write a 3-sentence executive summary of the following incident: A Kubernetes cluster ran out of disk space on 3 nodes simultaneously at 2am, causing 47 pod evictions across 12 services. Root cause was log rotation misconfiguration deployed in the previous release. Resolution took 4 hours and required manual node cleanup."

### Reasoning (3 prompts)
13. "You're an IT support engineer deciding between two MDM solutions for a 500-person company: Kandji (Apple-focused, $8/device/month, 100+ pre-built automations) vs. Jamf Pro (broader ecosystem, $12/device/month, deeper API). The company is 90% Mac, 10% Windows. Build a decision matrix and make a recommendation."
14. "A user reports their laptop is 'slow.' Walk through a systematic diagnostic process — what questions do you ask, in what order, and what does each answer tell you about the root cause?"
15. "Should a solo developer use a monorepo or polyrepo for 6 Tauri desktop apps that share 60% of their backend code? Argue both sides, then make a definitive recommendation."

---

## 5. SECURITY & CREDENTIALS

- **No credentials to store.** Ollama runs locally on localhost:11434 with no authentication. No API keys, no tokens.
- **Data stays local.** SQLite database at `~/.model-colosseum/colosseum.db`. No network calls except to localhost Ollama.
- **No sensitive data.** Debate transcripts are user-generated topics + model outputs. No PII processing.
- **File permissions:** Database directory created with 700 permissions (owner-only).
- **If Ollama URL is ever made configurable** (e.g., remote Ollama server): validate URL scheme (http/https only), warn user that transcripts will be sent over the network, and recommend HTTPS.

---

## 6. TESTING STRATEGY

### Phase 0 (Foundation)
- **Manual:** Verify Ollama model list matches `ollama list` CLI output. Verify DB schema with `sqlite3` CLI.
- **Automated:** Unit tests for Elo calculation (Rust tests). Unit test for system prompt builder (verify history injection, word limit insertion).
- **Verification:** `SELECT * FROM models` returns correct data after Ollama sync.

### Phase 1 (Arena)
- **Manual:** Run 3 debates. Verify: both models stream, rounds save correctly, Elo updates match manual calculation, leaderboard sorts correctly, debate history shows full transcripts.
- **Automated:** Integration test for debate flow (mock Ollama responses). Unit tests for Elo update with K-factor decay.
- **Verification:** After 5 debates, verify `SUM(arena_wins + arena_losses + arena_draws) = total_debates` for all models.

### Phase 2 (Benchmark)
- **Manual:** Run default test suite with 3 models. Score all results manually. Run auto-judge. Compare auto vs manual scores (should correlate > 0.6).
- **Automated:** Unit tests for score aggregation. Test that self-judging is prevented.
- **Verification:** `COUNT(benchmark_results) = COUNT(prompts) × COUNT(selected_models)` per run.

### Phase 3 (Sparring)
- **Manual:** Complete 2 sparring sessions at Casual and Competitive difficulty. Verify scorecard parses correctly. Check Elo updates.
- **Automated:** Unit test for scorecard JSON parsing with edge cases (malformed output, missing fields).
- **Verification:** `user_stats` row exists and totals match debate count.

### Phase 4 (Polish)
- **Manual:** Test all import/export round-trips. Verify settings persistence across app restarts. Test all debate modes.
- **Verification:** Exported JSON imports cleanly. All settings survive app restart.

---

## 7. CLAUDE CODE HANDOFF NOTES

### Session Strategy
- **Session 1 (2-3 hrs):** Phase 0 — Scaffold, DB, Ollama client. End state: app launches, models listed, streaming works.
- **Session 2 (3 hrs):** Phase 1a — Debate engine + split pane UI. End state: two models debate with concurrent streaming.
- **Session 3 (2-3 hrs):** Phase 1b — Vote/Elo, leaderboard, history. End state: Arena mode fully functional.
- **Session 4 (3 hrs):** Phase 2a — Test suite CRUD + batch runner. End state: benchmarks execute with progress.
- **Session 5 (2-3 hrs):** Phase 2b — Scoring + auto-judge + radar chart. End state: Benchmark mode fully functional.
- **Session 6 (3 hrs):** Phase 3 — Sparring Ring complete. End state: all three modes working.
- **Session 7 (2-3 hrs):** Phase 4 — Polish, modes, settings, export.

### Context Management
- Always include: `CLAUDE.md`, `src-tauri/src/db.rs`, `src-tauri/src/ollama.rs`
- For Arena sessions: add `src-tauri/src/debate.rs`, `src/pages/Arena.tsx`
- For Benchmark sessions: add `src-tauri/src/benchmark.rs`, `src/pages/Benchmark.tsx`
- For Sparring sessions: add `src-tauri/src/debate.rs` (shared), `src/pages/SparringRing.tsx`
- Max 5-7 files per session context

### Known Gotchas
- Do NOT let Claude Code scaffold the entire project in one session — follow the phase plan
- Tauri 2.0 uses `@tauri-apps/api` v2 — the import paths changed from v1. Don't use v1 examples.
- `rusqlite` with `bundled` feature compiles SQLite from source — first build will be slow (~2 min)
- Ollama streaming returns NDJSON (newline-delimited JSON), NOT SSE. Parse line-by-line.
- Tauri events from Rust to frontend use `app_handle.emit("event_name", payload)` in v2

### Resumption Strategy
If context is lost mid-phase:
1. Read `CLAUDE.md` for current phase and completed tasks
2. Check which DB tables have data (indicates completed phases)
3. Run the app — visual state indicates what's built
4. Re-read the specific phase tasks from `IMPLEMENTATION-ROADMAP.md`
