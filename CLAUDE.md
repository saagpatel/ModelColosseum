# Model Colosseum

## Project Overview
A local-first Tauri 2.0 desktop app for evaluating Ollama models across three modes: Arena (model vs model debates with Elo ratings), Benchmark (custom test suites with manual + auto-judge scoring), and Sparring Ring (structured human vs AI debates with scorecards). All modes feed a unified leaderboard backed by SQLite. macOS-only, dark theme, arena/colosseum aesthetic.

## Tech Stack
- Runtime: Tauri 2.x (Rust backend + webview frontend)
- Frontend: React 19 + TypeScript 5.x strict mode
- Build: Vite 6.x with `@tauri-apps/vite-plugin`
- Styling: Tailwind CSS 4.x (dark theme, gold/amber accents)
- State: Zustand 5.x
- Routing: React Router 7.x
- Charts: Recharts 2.x
- Database: SQLite via `rusqlite` 0.31+ (bundled, WAL mode)
- HTTP: `reqwest` 0.12+ (async streaming)
- Async: `tokio` 1.x
- System info: `sysinfo` 0.31+
- LLM: Ollama REST API (localhost:11434)

## Architecture
React frontend communicates with Rust backend via Tauri IPC (`invoke` for commands, `listen` for streaming events). Rust backend owns all Ollama communication, SQLite access, and Elo calculations. Frontend is purely presentational + state management.

Key modules:
- `src-tauri/src/db.rs` — SQLite connection, migrations, schema (13 tables), seed data
- `src-tauri/src/ollama.rs` — Ollama REST client with streaming (reads configurable URL from settings)
- `src-tauri/src/lib.rs` — All Tauri commands, Model/Setting structs, settings key whitelist
- `src-tauri/src/debate.rs` — Arena (3 formats) + Sparring debate engine, vote + Elo, scorecards
- `src-tauri/src/benchmark.rs` — CRUD, runner, auto-judge, blind comparison, hardware metrics, import/export
- `src-tauri/src/elo.rs` — Elo rating calculations (67 tests)
- `src-tauri/src/prompts.rs` — System prompt templates (arena, formal, socratic, sparring, scorecard judge)

## Development Conventions
- TypeScript strict mode. No `any` types.
- React: Functional components with hooks only. No class components.
- Rust: `clippy` clean. `cargo fmt` on save.
- File naming: `snake_case.rs` for Rust, `PascalCase.tsx` for React components, `camelCase.ts` for utilities
- Git commits: conventional commits (`feat:`, `fix:`, `refactor:`, `chore:`)
- All Tauri commands return `Result<T, String>` — handle errors in Rust, display in frontend
- Database writes wrapped in explicit transactions
- No unwrap() in production Rust code — use ? operator or proper error handling

## Current Phase
**v1.0.0 — Feature Complete** (all phases done, audit remediation applied)

- [x] **Phase 0: Foundation** — Tauri 2.0 scaffold, SQLite (13 tables, WAL), Ollama REST client, Elo module
- [x] **Phase 1: Arena Mode** — Debate engine (freestyle/formal/socratic), vote + Elo, leaderboard, history
- [x] **Phase 2: Benchmark** — CRUD suites/prompts, runner with TTFT/TPS metrics, manual + auto-judge scoring, blind comparison, hardware metrics, import/export
- [x] **Phase 3: Sparring Ring** — Human vs AI debates, 3 difficulty levels, 4-phase structure, scorecards, user Elo
- [x] **Phase 4: Polish** — 3 debate formats, topic suggestions, settings page, blind test, animations, skeleton loading, export (Markdown/CSV/JSON)
- [x] **Audit** — Security hardening (configurable Ollama URL, query limit caps, settings key whitelist), accessibility (ARIA attributes), error handling, 67 Rust tests

## Key Decisions Made
| Decision | Choice | Rationale |
|----------|--------|-----------|
| Concurrent streaming | Concurrent with auto sequential fallback when models > 40B combined | User wants dramatic visual. Fallback prevents OOM. |
| Database access | rusqlite directly, not tauri-plugin-sql | More control over WAL mode, migrations, concurrent access |
| Elo parameters | Start 1500, K=40→32→24 based on game count | Standard chess Elo with decay to stabilize ratings |
| Benchmark scoring | 1-5 manual, 1-10 auto-judge normalized | Fast manual scoring, more granular auto-judge |
| App modes | Arena → Benchmark → Sparring (build order) | Arena builds all shared infra, others plug in |
| DB location | ~/.model-colosseum/colosseum.db | Standard macOS app data location |
| Ollama streaming | NDJSON line-by-line parsing, not SSE | That's what Ollama returns |

## Do NOT
- Do not scaffold the entire project in one session — follow the phased plan strictly
- Do not use Tauri v1 APIs or import paths — this is Tauri 2.x (`@tauri-apps/api` v2)
- Do not use `tauri-plugin-sql` — we use `rusqlite` directly
- Do not use `unwrap()` in Rust production code — use `?` or proper error handling
- Do not make any network calls except to localhost Ollama (no telemetry, no cloud)
- Do not use class components in React — hooks only
- Do not store any data outside `~/.model-colosseum/` — single source of truth
- Do not assume Ollama is running — always health check first and handle absence gracefully
