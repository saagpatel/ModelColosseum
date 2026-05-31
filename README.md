# Model Colosseum

[![Rust](https://img.shields.io/badge/rust-%23dea584?style=flat-square&logo=rust)](#) [![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](#)

> Your local LLMs, fighting for glory. You decide who wins.

Model Colosseum is a local-first macOS desktop app that turns your Ollama models into gladiators. Pick a topic, pick two models, and watch them argue in real time — streaming token by token, side by side. Elo ratings track performance across debates and benchmarks. No cloud, no API keys, no telemetry.

## Features

- **Arena Mode** — freestyle, formal (3-phase), or Socratic debate formats across 3/5/7 rounds with simultaneous streaming
- **Benchmark Mode** — custom test suites with manual scoring, auto-judge (LLM-scored), blind A/B comparison, TTFT/TPS hardware metrics, and JSON/CSV/Markdown export; ships with a 15-prompt default suite covering coding, creative writing, analysis, summarization, and reasoning
- **Sparring Ring** — structured 4-phase human vs AI debates (casual / competitive / expert difficulty) with multi-dimension scorecards and user Elo tracking
- **Elo ratings** — per-model ratings updated after every voted debate outcome
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
- [Ollama](https://ollama.com) running locally with at least one model pulled

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
