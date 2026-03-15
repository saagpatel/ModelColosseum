# Implementation Roadmap

## Session Strategy
Each Claude Code session targets ONE phase or sub-phase. Sessions scoped to ~2-3 hours of focused work. Do not attempt multiple phases in a single session. If a session finishes early, polish the current phase rather than starting the next.

Build order: Foundation → Arena → Benchmark → Sparring → Polish

---

## Phase 0: Foundation (Week 1)

### Session 1 scope: Project scaffold + database
- Create Tauri 2.0 project: `npm create tauri-app@latest model-colosseum -- --template react-ts`
- Install and configure: Tailwind 4, Zustand, React Router, Recharts
- Add Rust dependencies: `rusqlite` (bundled), `reqwest`, `tokio`, `serde`, `serde_json`, `sysinfo`
- Implement `src-tauri/src/db.rs`: init_db(), get_db() singleton, full schema creation (all tables from the data model), WAL mode
- Verify: `cargo tauri dev` launches, database created at `~/.model-colosseum/colosseum.db`

### Session 2 scope: Ollama client + model sync + Elo module
- Implement `src-tauri/src/ollama.rs`: health_check(), list_models(), show_model(), generate_stream()
- Implement `list_models` and `refresh_models` Tauri commands that sync Ollama models to SQLite
- Build minimal `<ModelSelector>` React component
- Implement `src-tauri/src/elo.rs`: expected_score(), update_ratings(), k_factor_for_games()
- Write Rust unit tests for Elo calculations
- Verify: model dropdown populated, streaming tokens received in Rust

**Deliverables:** Working app shell, SQLite with schema, Ollama model list in UI, streaming POC
**Verification:** Run `sqlite3 ~/.model-colosseum/colosseum.db ".tables"` — all tables present. Model dropdown matches `ollama list`. Elo tests pass with `cargo test`.

---

## Phase 1: Arena Mode (Weeks 2-3)

### Session 3 scope: Debate engine + concurrent streaming
- Implement `src-tauri/src/debate.rs`: start_debate() Tauri command
- Build round loop with concurrent tokio::spawn tasks for both models
- Implement system prompt builder in `src-tauri/src/prompts.rs`
- Emit Tauri events: `debate:stream:a`, `debate:stream:b`, `debate:round_complete`
- Implement concurrent/sequential fallback based on model parameter sizes
- Save round content to SQLite after each round completes

### Session 4 scope: Arena UI + split pane
- Build `src/pages/Arena.tsx`: topic input, model selectors, start button
- Build split-pane debate viewer with streaming text rendering
- Token-by-token display with typing cursor animation
- Round counter, VS badge, model names/icons on each side
- Wire up Tauri event listeners for both streams
- Dark theme with arena aesthetic (dark slate, gold accents)

### Session 5 scope: Vote, Elo, leaderboard, history
- Build vote UI (Left Wins / Right Wins / Draw) after final round
- Implement `vote_debate` Tauri command: update debate record, calculate Elo, write elo_history
- Build `src/pages/Leaderboard.tsx`: ranked table, Elo sparklines, W/L/D, provisional badges
- Build `src/pages/History.tsx`: debate list with search/filter, expandable transcripts
- Build app navigation shell (sidebar or tabs)

**Deliverables:** Fully functional Arena mode with debates, voting, Elo, leaderboard, history
**Verification:** Run 3 debates between different model pairs. Verify Elo updates correctly. Leaderboard sorts by Elo. History shows full transcripts. Concurrent streaming works for small models, sequential fallback for large.

---

## Phase 2: Benchmark Mode (Weeks 4-5)

### Session 6 scope: Test suite CRUD + batch runner
- Build test suite editor UI in `src/pages/Benchmark.tsx`
- Implement CRUD Tauri commands for suites and prompts
- Insert default "Starter Suite" with 15 prompts on first launch
- Implement `src-tauri/src/benchmark.rs`: start_benchmark() with sequential model iteration
- Emit progress events: `benchmark:progress`, `benchmark:stream`
- Build progress UI: overall bar, current model/prompt, streaming preview, ETA

### Session 7 scope: Scoring + auto-judge + visualization
- Build results grid: rows=prompts, columns=models, cells=output+metrics
- Implement manual 1-5 scoring with star widget
- Implement blind "Score All" mode (hide model names)
- Implement `auto_judge_benchmark`: judge prompt builder, score parser, no-self-judging guard
- Extend leaderboard: category breakdown, radar chart (Recharts), performance table (avg TPS, TTFT)
- Build benchmark run history with comparison view

**Deliverables:** Full Benchmark mode with suites, runner, scoring, visualization
**Verification:** Run default suite with 3 models. Manual score all. Auto-judge all. Radar chart renders. Performance metrics accurate. Run comparison works.

---

## Phase 3: Sparring Ring (Weeks 5-6)

### Session 8 scope: Complete Sparring Ring
- Extend debate engine for sparring mode (4 phases: Opening → Rebuttal × 2 → Closing)
- Build `src/pages/SparringRing.tsx`: topic/side/model selector, split input/output view, phase indicator, word counter
- Build sparring-specific system prompts (Opponent + Judge personas)
- Add difficulty levels (Casual / Competitive / Expert)
- Implement post-debate scorecard: judge prompt, JSON parsing, scorecard display
- Implement user_stats Elo tracking
- Add Markdown transcript export for all debate types

**Deliverables:** Fully functional Sparring Ring with scorecards, difficulty levels, user Elo
**Verification:** Complete sparring sessions at all 3 difficulties. Scorecard displays 8 scores + 4 feedback items. User Elo updates. Markdown export is clean and complete.

---

## Phase 4: Polish & Advanced Features (Weeks 7-8)

### Session 9 scope: Debate modes + settings
- Add Formal Debate mode (Opening/Rebuttal/Closing structure)
- Add Socratic mode (question/defend alternation)
- Build `src/pages/Settings.tsx`: all configurable values, system prompt editor
- Topic suggestion button using lightweight model

### Session 10 scope: Final polish
- Head-to-head blind test mode for Benchmark
- Hardware metrics during benchmark runs (sysinfo integration)
- Visual polish: Elo animations, VS badge, winner effects, loading/empty states
- Import/export: test suites as JSON, leaderboard as CSV
- Final bug sweep and performance optimization

**Deliverables:** Complete app with all modes, settings, polish, import/export
**Verification:** All 3 debate modes produce different debate styles. Settings persist across restart. Import/export round-trips cleanly. No visual jank.

---

## Context Management

**Always include in every session:**
- `CLAUDE.md`
- `src-tauri/src/db.rs`
- `src-tauri/src/ollama.rs`

**Phase-specific additions:**
| Phase | Additional files |
|-------|-----------------|
| Phase 0 | `src-tauri/src/elo.rs`, `Cargo.toml`, `package.json` |
| Phase 1 | `src-tauri/src/debate.rs`, `src-tauri/src/prompts.rs`, `src/pages/Arena.tsx` |
| Phase 2 | `src-tauri/src/benchmark.rs`, `src/pages/Benchmark.tsx` |
| Phase 3 | `src-tauri/src/debate.rs`, `src/pages/SparringRing.tsx` |
| Phase 4 | `src/pages/Settings.tsx`, whichever mode file is being polished |

**Maximum context files per session: 5-7** (keep it focused)
