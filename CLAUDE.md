# Model Colosseum

Local-first Tauri 2.x desktop app for evaluating Ollama models: Arena (model vs model debates, Elo ratings), Benchmark (custom test suites, TTFT/TPS metrics, manual + auto-judge scoring), and Sparring Ring (human vs AI debates, scorecards). All modes feed a unified leaderboard backed by SQLite. macOS-only, dark theme, arena/colosseum aesthetic.

## Stack

- Runtime: Tauri 2.x (Rust backend + webview frontend)
- Frontend: React 19 + TypeScript 5.x strict mode
- Build: Vite 6.x with `@tauri-apps/vite-plugin`
- Styling: Tailwind CSS 4.x (dark theme, gold/amber accents)
- State: Zustand 5.x; Routing: React Router 7.x; Charts: Recharts 2.x
- Database: SQLite via `rusqlite` 0.31+ (bundled, WAL mode)
- HTTP: `reqwest` 0.12+; Async: `tokio` 1.x; System info: `sysinfo` 0.31+
- LLM: Ollama REST API (`localhost:11434`)

## Architecture

React frontend → Tauri IPC (`invoke` / `listen`) → Rust backend. Rust owns all Ollama communication, SQLite access, and Elo calculations. Frontend is purely presentational + state management.

Key modules:
- `src-tauri/src/db.rs` — SQLite connection, migrations, schema (13 tables), seed data
- `src-tauri/src/ollama.rs` — Ollama REST client with streaming (reads configurable URL from settings)
- `src-tauri/src/lib.rs` — All Tauri commands, Model/Setting structs, settings key whitelist
- `src-tauri/src/debate.rs` — Arena (3 formats) + Sparring debate engine, vote + Elo, scorecards
- `src-tauri/src/benchmark.rs` — CRUD, runner, auto-judge, blind comparison, hardware metrics, import/export
- `src-tauri/src/elo.rs` — Elo rating calculations (67 tests)
- `src-tauri/src/prompts.rs` — System prompt templates (arena, formal, socratic, sparring, scorecard judge)

## Build / Test / Run

```bash
pnpm install          # install deps
pnpm tauri dev        # dev server (hot reload)
pnpm tauri build      # production build
pnpm test             # runs: cd src-tauri && cargo test
cargo clippy -- -D warnings  # lint (must pass clean)
cargo fmt             # format on save
```

## Conventions

- TypeScript strict mode; type with `unknown` + narrowing, never `any`
- React functional components with hooks only; no class components
- Rust: `clippy` clean, `cargo fmt` on save; use `?` or proper error handling — no `unwrap()` in production code
- File naming: `snake_case.rs`, `PascalCase.tsx`, `camelCase.ts`
- Tauri commands return `Result<T, String>` — handle errors in Rust, surface to frontend
- Database writes in explicit transactions
- Data directory: `~/.model-colosseum/` — the only storage location (`colosseum.db` lives here)

## Gotchas

- Use Tauri v2 APIs only — import paths are `@tauri-apps/api` v2; v1 APIs are incompatible
- Use `rusqlite` directly, not `tauri-plugin-sql` — needed for WAL mode, migrations, concurrent access
- Always health-check Ollama before calling it; handle absence gracefully (`localhost:11434`)
- Network calls to localhost Ollama only — no telemetry, no cloud, no external endpoints
- Concurrent streaming: runs concurrent with auto sequential fallback when models > 40B combined (prevents OOM)
- Ollama streaming: NDJSON line-by-line parsing, not SSE

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Concurrent streaming | Concurrent + auto sequential fallback (>40B combined) | Dramatic visual; fallback prevents OOM |
| Database access | `rusqlite` directly, not `tauri-plugin-sql` | WAL mode, migrations, concurrent access |
| Elo parameters | Start 1500, K=40→32→24 by game count | Standard chess Elo with decay to stabilize |
| Benchmark scoring | 1-5 manual, 1-10 auto-judge normalized | Fast manual scoring, more granular auto-judge |
| DB location | `~/.model-colosseum/colosseum.db` | Standard macOS app data location |
| Ollama streaming | NDJSON line-by-line, not SSE | That's what Ollama returns |

<!-- portfolio-context:start -->
# Portfolio Context

## What This Project Is

ModelColosseum is an active local project in the /Users/d/Projects portfolio.

## Current State

**v1.0.0 — Feature Complete** (all phases done, audit remediation applied)

- [x] **Phase 0: Foundation** — Tauri 2.0 scaffold, SQLite (13 tables, WAL), Ollama REST client, Elo module
- [x] **Phase 1: Arena Mode** — Debate engine (freestyle/formal/socratic), vote + Elo, leaderboard, history
- [x] **Phase 2: Benchmark** — CRUD suites/prompts, runner with TTFT/TPS metrics, manual + auto-judge scoring, blind comparison, hardware metrics, import/export
- [x] **Phase 3: Sparring Ring** — Human vs AI debates, 3 difficulty levels, 4-phase structure, scorecards, user Elo
- [x] **Phase 4: Polish** — 3 debate formats, topic suggestions, settings page, blind test, animations, skeleton loading, export (Markdown/CSV/JSON)
- [x] **Audit** — Security hardening (configurable Ollama URL, query limit caps, settings key whitelist), accessibility (ARIA attributes), error handling, 67 Rust tests

## Stack

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

## How To Run

- TypeScript strict mode. No `any` types.
- React: Functional components with hooks only. No class components.
- Rust: `clippy` clean. `cargo fmt` on save.
- File naming: `snake_case.rs` for Rust, `PascalCase.tsx` for React components, `camelCase.ts` for utilities
- Git commits: conventional commits (`feat:`, `fix:`, `refactor:`, `chore:`)
- All Tauri commands return `Result<T, String>` — handle errors in Rust, display in frontend
- Database writes wrapped in explicit transactions
- No unwrap() in production Rust code — use ? operator or proper error handling

## Known Risks

- Do not scaffold the entire project in one session — follow the phased plan strictly
- Do not use Tauri v1 APIs or import paths — this is Tauri 2.x (`@tauri-apps/api` v2)
- Do not use `tauri-plugin-sql` — we use `rusqlite` directly
- Do not use `unwrap()` in Rust production code — use `?` or proper error handling
- Do not make any network calls except to localhost Ollama (no telemetry, no cloud)
- Do not use class components in React — hooks only
- Do not store any data outside `~/.model-colosseum/` — single source of truth
- Do not assume Ollama is running — always health check first and handle absence gracefully

## Next Recommended Move

Use this context plus the README and supporting docs to resume the next active task, then promote the repo beyond minimum-viable by capturing a dedicated handoff, roadmap, or discovery artifact.

<!-- portfolio-context:end -->
