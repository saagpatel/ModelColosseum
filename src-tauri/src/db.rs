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

    // Insert default benchmark test suite and prompts if none exist
    let suite_count: i64 = conn.query_row("SELECT COUNT(*) FROM test_suites", [], |r| r.get(0))?;
    if suite_count == 0 {
        conn.execute(
            "INSERT INTO test_suites (name, description, is_default)
             VALUES ('Default Suite', 'Balanced benchmark suite covering coding, creative writing, analysis, summarization, and reasoning.', 1)",
            [],
        )?;
        let suite_id = conn.last_insert_rowid();

        let prompts: &[(&str, &str, &str)] = &[
            // --- Coding ---
            (
                "coding",
                "TypeScript Refactor",
                r#"Refactor the following TypeScript function to use modern ES2024+ features, improve type safety, and reduce cognitive complexity. Explain each change you make.

```typescript
function processUserData(users: any[], filterActive: boolean, maxAge?: number): any[] {
  var result = [];
  for (var i = 0; i < users.length; i++) {
    var user = users[i];
    if (filterActive && !user.active) continue;
    if (maxAge !== undefined && user.age > maxAge) continue;
    var processed = {
      id: user.id,
      name: user.firstName + ' ' + user.lastName,
      email: user.email.toLowerCase(),
      age: user.age,
      active: user.active
    };
    result.push(processed);
  }
  return result;
}
```"#,
            ),
            (
                "coding",
                "Rust LRU Cache",
                r#"Implement a thread-safe LRU (Least Recently Used) cache in Rust with the following requirements:
- Generic over key and value types (K: Hash + Eq + Clone, V: Clone)
- Configurable maximum capacity set at construction time
- O(1) get and put operations
- Thread-safe: wrap in Arc<Mutex<...>> or use interior mutability
- Implement: new(capacity: usize), get(&self, key: &K) -> Option<V>, put(&mut self, key: K, value: V)
- Include unit tests covering: basic get/put, eviction of LRU entry, capacity=1 edge case, and thread-safety

Provide the complete implementation with tests."#,
            ),
            (
                "coding",
                "SQL Query Debug",
                r#"The following SQL query is supposed to return the top 10 customers by total order value in the last 30 days, but it returns incorrect results and runs slowly on a table with 2M orders. Identify all bugs and performance issues, then provide a corrected query with explanation.

```sql
SELECT c.name, SUM(o.total) as revenue
FROM customers c, orders o
WHERE c.id = o.customer_id
AND o.created_at > NOW() - 30
GROUP BY c.name
ORDER BY revenue
LIMIT 10;
```

Tables: customers(id, name, email), orders(id, customer_id, total, created_at, status)"#,
            ),
            // --- Creative Writing ---
            (
                "creative",
                "Product Description",
                r#"Write a compelling product description for 'BrewMind Pro', an AI-powered coffee machine. Target audience: tech-savvy professionals aged 25-40 who value both quality and efficiency. Tone: sophisticated yet approachable, not overly technical.

Requirements:
- Maximum 200 words
- Open with a hook that is NOT a question
- Highlight 3 key differentiating features (AI-driven personalization, voice control, predictive restocking)
- Include a subtle call to action
- Avoid clichés like 'game-changer' or 'revolutionary'"#,
            ),
            (
                "creative",
                "D&D Encounter",
                r#"Design a complete D&D 5e encounter for a level-5 party of 4 players. The setting is an ancient library that has been overtaken by a corrupted archivist.

Include:
1. The main antagonist with stat block highlights (AC, HP, key abilities — no need for full stat block)
2. Two environmental hazards that interact with the combat
3. A non-combat resolution path with skill check DCs
4. Rewards: one specific magic item with flavour text, plus XP
5. One plot hook that connects to a larger campaign arc

Balance the encounter for Medium difficulty (XP budget ~2,000)."#,
            ),
            (
                "creative",
                "Technical Blog Post Intro",
                r#"Write the opening section (introduction + first argument, ~300 words) of a technical blog post titled: "Stop Defaulting to Microservices: When a Monolith Is the Right Call"

Requirements:
- Challenge the assumption that microservices are always the modern, correct choice
- Use a concrete failure scenario as the hook (not hypothetical — write it as if it happened)
- Establish credibility without being arrogant
- End the section with a clear thesis statement
- Audience: senior engineers and tech leads who have built both architectures"#,
            ),
            // --- Analysis ---
            (
                "analysis",
                "Metrics Divergence Analysis",
                r#"A B2B SaaS company reports: Monthly Active Users (MAU) increased 40% YoY, but revenue grew only 5% YoY. The company has a seat-based pricing model with three tiers: Starter ($29/seat/mo), Growth ($79/seat/mo), Enterprise (custom).

Provide:
1. At least 5 distinct hypotheses that explain this divergence (consider pricing, product usage, customer mix, expansion/contraction, and external factors)
2. For each hypothesis: what specific data or metric would confirm or refute it
3. Which hypothesis you consider most likely given the information, and why
4. Your top recommendation for the executive team to investigate first"#,
            ),
            (
                "analysis",
                "Auth Token Storage Security",
                r#"Compare three approaches for storing API authentication tokens in a web application:
1. HttpOnly cookies (SameSite=Strict)
2. localStorage
3. In-memory JavaScript variable (no persistence)

Analyse each approach across these dimensions:
- XSS vulnerability
- CSRF vulnerability
- Persistence across page refresh/tab close
- Mobile app compatibility
- Server-Side Rendering (SSR/Next.js) compatibility
- Implementation complexity
- Logout/revocation reliability

Conclude with a clear recommendation for: (a) a standard SPA, (b) a Next.js app with SSR, and (c) a React Native mobile app."#,
            ),
            (
                "analysis",
                "Error Pattern Analysis",
                r#"Analyse the following production error logs and identify the root cause, contributing factors, and proposed fix.

```
2024-03-15 14:23:01 ERROR [pool-worker-3] HikariPool-1 - Connection is not available, request timed out after 30000ms
2024-03-15 14:23:01 ERROR [api-handler-12] Failed to acquire DB connection: timeout
2024-03-15 14:23:02 ERROR [pool-worker-7] HikariPool-1 - Connection is not available, request timed out after 30000ms
2024-03-15 14:23:03 WARN  [pool-monitor] Active connections: 20/20, Pending: 47, Idle: 0
2024-03-15 14:23:03 INFO  [slow-query-log] Query took 28400ms: SELECT * FROM reports WHERE user_id=? AND date >= ?
2024-03-15 14:23:03 INFO  [slow-query-log] Query took 31200ms: SELECT * FROM reports WHERE user_id=? AND date >= ?
2024-03-15 14:22:45 INFO  [scheduler] Nightly report export job started (batch_size=500)
```

Provide: root cause analysis, why it cascaded, the immediate fix, and the long-term architectural recommendation."#,
            ),
            // --- Summarization ---
            (
                "summarization",
                "Jira Ticket Summary",
                r#"Transform the following verbose Jira ticket into a concise, actionable format suitable for a sprint planning session.

---
ORIGINAL TICKET:
Title: CSV Export is not working properly and users are complaining
Reporter: Sarah M. (Customer Success)
Description: Hi team, so I've been getting a bunch of complaints from customers. Basically what happens is that when users try to export their data as a CSV file from the Reports section, the download either takes forever (like 5+ minutes) and then times out, or it does download but the file is completely empty or sometimes has the wrong data in it. I talked to Dave from enterprise customer Acme Corp and he said it started happening about 2 weeks ago. We haven't deployed anything major recently I think? But maybe something changed. The customers are really frustrated. Some of them have mentioned they need this for their end-of-month reporting. I think it might be related to large datasets because the smaller exports seem to work fine. Not sure if this is a backend or frontend issue.
---

Output format:
- One-line summary (max 15 words)
- Problem statement (2 sentences)
- Observed behaviour vs expected behaviour
- Acceptance criteria (bulleted, testable)
- Suggested labels/components
- Priority recommendation with reasoning"#,
            ),
            (
                "summarization",
                "Architecture Decision Summary",
                r#"Convert the following meeting transcript excerpt into a structured Architecture Decision Record (ADR).

---
TRANSCRIPT:
"...so we've been going back and forth on this for three weeks. The core issue is that our current MongoDB setup is causing us pain with our reporting queries — they're just not performant at scale, and our data science team keeps complaining. Maria pointed out that we already have Postgres running for auth, so the ops overhead isn't zero but it's manageable. The concern from the backend team was migration risk — we've got like 18 months of document data with variable schemas. Tom suggested we do a strangler fig pattern, starting new features in Postgres while keeping existing data in Mongo. The risk there is we maintain two databases for potentially 12-18 months. Finance wants a decision by end of Q2. We decided we'd go with the migration to Postgres for new features immediately, and plan a 6-month migration window for historical data, starting with the reporting collections first..."
---

Format as a proper ADR with: Title, Status, Context, Decision, Consequences (positive and negative), and Alternatives Considered."#,
            ),
            (
                "summarization",
                "Incident Post-Mortem",
                r#"Condense the following incident timeline into a professional 3-paragraph post-mortem summary suitable for stakeholder communication.

---
TIMELINE:
14:05 - Engineer pushed migration to add NOT NULL column to payments table (200M rows) without backfill
14:06 - Deploy began on production
14:08 - First alerts: payment service error rate spiked from 0.1% to 34%
14:09 - PagerDuty triggered, on-call engineer (Raj) acknowledged
14:12 - Raj identified migration as cause from error logs: "column payment_method cannot be null"
14:15 - Decision made to roll back migration
14:17 - Rollback deploy initiated
14:23 - Rollback completed, error rate returning to baseline
14:31 - Error rate back to 0.1%, incident resolved
14:45 - Post-incident review scheduled
Total impact: 25 minutes, ~12,400 failed payment attempts, ~$47K in lost GMV
---

Include: what happened, immediate impact, how it was resolved. Tone: clear, factual, non-blaming."#,
            ),
            // --- Reasoning ---
            (
                "reasoning",
                "MDM Solution Decision Matrix",
                r#"A 200-person company needs to choose a Mobile Device Management (MDM) solution. They have 180 macOS laptops and 20 Windows machines, a 3-person IT team, no existing MDM infrastructure, and a budget of ~$15/device/month max.

The three shortlisted options are:
1. Jamf Pro — industry standard for Apple, powerful, complex, ~$14/device/month
2. Kandji — modern Apple-focused, strong automation, ~$10/device/month
3. Microsoft Intune — cross-platform, included in their existing Microsoft 365 E3 licence (so effectively $0 additional cost)

Build a weighted decision matrix evaluating all three options across: total cost of ownership, time-to-value, macOS feature depth, Windows support, IT team learning curve, automation capabilities, and vendor lock-in risk.

Show your weights, scores, and reasoning. Provide a final recommendation with caveats."#,
            ),
            (
                "reasoning",
                "Laptop Diagnostic Decision Tree",
                r#"A user reports: "My laptop has been running really slow for the past week." You are a technical support specialist.

Create a systematic diagnostic decision tree that:
1. Starts with the 3 most important triage questions to ask the user (and what each answer implies)
2. Branches into the most likely root causes: hardware (thermal, storage, RAM), software (OS/background processes, malware, drivers), and environmental (power settings, updates)
3. For each branch: specifies the exact diagnostic steps and tools to use (be OS-specific where relevant: macOS vs Windows)
4. Defines clear decision points: when to escalate to hardware replacement vs. software remediation vs. user education
5. Ends each path with a concrete resolution action

Present as a structured tree with clear branching logic."#,
            ),
            (
                "reasoning",
                "Monorepo vs Polyrepo Decision",
                r#"A 15-engineer startup is debating their repository strategy as they scale from 3 to 10 services over the next year. Their current stack: TypeScript/Node.js backend services, a React frontend, a shared component library, and 2 data pipeline scripts. They use GitHub Actions for CI, deploy to AWS, and have 3 frontend engineers and 12 fullstack/backend engineers.

Task: Argue both sides of the monorepo vs. polyrepo debate as they apply specifically to this team's context, then provide a nuanced recommendation.

Structure your response as:
1. Top 3 arguments FOR monorepo (specific to their situation)
2. Top 3 arguments FOR polyrepo (specific to their situation)
3. The key decision factors that should tip the balance
4. Your recommendation with the specific tooling setup you'd suggest (e.g., if monorepo: Turborepo vs Nx vs Lerna)
5. One thing they should NOT do regardless of which they choose"#,
            ),
        ];

        for (order, (category, title, text)) in prompts.iter().enumerate() {
            conn.execute(
                "INSERT INTO prompts (suite_id, category, title, text, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![suite_id, category, title, text, order as i64],
            )?;
        }
    }

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
