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
- `src-tauri/src/db.rs` — SQLite connection, migrations, queries
- `src-tauri/src/ollama.rs` — Ollama REST client with streaming
- `src-tauri/src/debate.rs` — Arena + Sparring debate engine
- `src-tauri/src/benchmark.rs` — Benchmark runner
- `src-tauri/src/elo.rs` — Elo rating calculations
- `src-tauri/src/prompts.rs` — System prompt templates

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
**Phase 0: Foundation** (target: Week 1)
- [x] Scaffold Tauri 2.0 with React 19 + TS + Vite + Tailwind
- [x] Set up SQLite with rusqlite (WAL mode, all tables)
- [x] Build Ollama REST client (list, show, health, generate_stream)
- [x] Wire model list to frontend with ModelSelector component
- [x] Implement Elo calculation module with unit tests

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
