# Model Colosseum

[![Rust](https://img.shields.io/badge/rust-%23dea584?style=flat-square&logo=rust)](#) [![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](#)

> Your local LLMs, fighting for glory — with receipts.

Model Colosseum is a local-first macOS evaluation lab with a playful arena identity. It helps answer which local model to use for a specific capability on the current hardware, and how uncertain that choice is. Every evaluation records model, prompt, runtime, hardware, trial, judge, failure, and uncertainty provenance. No cloud, no API keys, no telemetry.

## Features

- **Arena Mode** — freestyle, formal (3-phase), or Socratic debate formats across 3/5/7 rounds with simultaneous streaming
- **Evaluation Lab** — immutable run manifests, repeated randomized trials, warm-up separation, timeouts/failures, balanced blind A/B positions, per-capability uncertainty, judge disagreement, hardware-aware comparability, and full JSON evidence export
- **Benchmark Mode** — custom task suites with manual scoring, local auto-judge scoring, blind human comparison, TTFT/TPS hardware metrics, and a 15-prompt default suite
- **Sparring Ring** — structured 4-phase human vs AI debates (casual / competitive / expert difficulty) with multi-dimension scorecards and user Elo tracking
- **Elo ratings** — playful Arena ratings updated by debate votes; evaluation trials never mutate Elo
- **Leaderboard** — unified model ranking with per-model Elo history charts
- **Topic suggestions** — let one of your models brainstorm debate topics
- **Split-pane viewer** — PRO on the left, CON on the right, both streaming in real time
- **Vote system** — Left Wins, Draw, or Right Wins; ratings update immediately
- **History** — full searchable record of all past arena debates
- **Settings** — configurable Ollama URL, round counts, word limits, Elo parameters, and per-role prompt templates

## Quick Start

### Prerequisites
- Rust stable toolchain
- Node.js 20+ and pnpm
- [Ollama](https://ollama.com) running locally with at least two completion-capable models already installed

### Installation
```bash
git clone https://github.com/saagpatel/ModelColosseum
cd ModelColosseum
pnpm install
```

### Usage
```bash
# Start Ollama (if not already running)
ollama serve

# Run the app in development
pnpm tauri dev

# Build release app
pnpm tauri build
```

## Tech Stack

| Layer | Technology |
|-------|------------|
| Desktop shell | Tauri 2 |
| Backend | Rust 2021 |
| Database | SQLite (rusqlite, WAL mode) |
| Frontend | React 19 + TypeScript + Tailwind CSS 4 + Recharts |
| State | Zustand 5 |
| LLM runtime | Ollama (local) via streaming REST API |
| Routing | React Router 7 |

## License

MIT

See [Evaluation methodology](docs/evaluation-methodology.md) for metric semantics, confidence rules, comparability gates, exports, and migration/rollback guidance.
