# ModelColosseum Codex Playbook

## Communication Contract

Follow the global Codex communication contract. Keep updates short, PM-readable, operator-grade, and focused on what changed, what passed, and what still needs attention.

## Project Goal

ModelColosseum is a local-first Tauri 2 desktop app for evaluating Ollama models through arenas, benchmarks, sparring, scorecards, and a SQLite-backed leaderboard.

## First Read

- `README.md`
- `CLAUDE.md`
- `src-tauri/Cargo.toml`
- `.codex/verify.commands`

## Core Rules

- Keep all model calls local to Ollama unless the user explicitly changes the product contract.
- Do not add telemetry, cloud sync, or remote judging.
- Keep SQLite as the source of truth under the app data path.
- Keep Rust responsible for Ollama communication, scoring, Elo calculations, database writes, and streaming events.
- Frontend should stay presentational/stateful; avoid duplicating scoring or persistence rules in React.
- Do not assume Ollama is running; health check and fail gracefully.

## Codex App Usage

- Use Codex App Projects for repo-scoped implementation, debugging, and verification.
- Use Worktrees for debate engine, benchmark runner, auto-judge, Elo, database migration, Ollama streaming, import/export, or Tauri capability changes.
- Use file search before editing because behavior spans Rust engines, SQLite schema, prompt templates, Tauri commands/events, and React mode views.
- Use app-window or browser evidence when arena, benchmark, sparring, leaderboard, settings, or export UI changes.
- Use artifacts when benchmark results, scorecards, or comparison reports need reusable review.

## Verification

Use `.codex/verify.commands` as the canonical local gate.

## Done Criteria

- The relevant verifier commands have been run, or the exact blocker is recorded.
- Scoring, Elo, benchmark, and database changes have focused tests or fixture evidence.
- UI changes have app-window or screenshot evidence when visual behavior matters.
