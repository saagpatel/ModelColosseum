# Model Colosseum

**Pit your local LLMs against each other. May the most eloquent model win.**

Model Colosseum is a local-first macOS desktop app that turns your Ollama models into gladiators. Pick a topic, pick two models, and watch them argue in real time — streaming token by token, side by side. Then you decide who won. Elo ratings track the carnage.

No cloud. No API keys. No telemetry. Just your hardware, your models, and the colosseum.

---

## Three Ways to Fight

### Arena Mode

The main event. Two models enter, one topic, and you're the judge.

- **Freestyle** — Open-ended back-and-forth across 3, 5, or 7 rounds
- **Formal** — Structured 3-phase debate: Opening Statement, Rebuttal, Closing Argument
- **Socratic** — One model asks probing questions while the other defends, then they swap roles

Both models stream simultaneously (when your hardware allows it). You watch arguments materialize in real time in a split-pane view — PRO on the left, CON on the right. When the debate ends, you cast your vote: Left Wins, Draw, or Right Wins. Elo ratings update accordingly.

Can't think of a topic? Hit "Suggest Topics" and let one of your models brainstorm for you.

### Benchmark Mode

The laboratory. Test your models against a standardized battery of 15 prompts spanning five categories:

| Category | What It Tests |
|---|---|
| Coding | TypeScript refactoring, Rust LRU cache, SQL debugging |
| Creative Writing | Product copy, D&D encounters, technical blog posts |
| Analysis | Metrics interpretation, security analysis, error diagnosis |
| Summarization | Jira tickets, ADRs, incident post-mortems |
| Reasoning | Decision matrices, diagnostic trees, architecture debates |

Run a benchmark, then score the results three different ways:

- **Manual scoring** — Rate each response 1-5 stars, prompt by prompt
- **Auto-judge** — Pick a model to score the others (it reads each response, thinks about it, and returns a 1-10 rating)
- **Blind comparison** — Two models' outputs shown side-by-side with identities hidden. You pick the better one. No bias, just vibes.

Benchmarks track hardware metrics (CPU, RAM, swap) throughout the run, so you can see whether that 32B model is actually using your machine differently than the 7B one.

Export everything: test suites as JSON (import them on another machine), leaderboards as CSV, full benchmark reports as Markdown.

### Sparring Ring

You vs. the machine.

Pick a topic, choose your side (PRO or CON), select an opponent model, and debate. Four phases: Opening, Rebuttal, Rebuttal, Closing — each with word limits that force you to be concise.

Three difficulty levels:

- **Casual** — Fair and balanced. Acknowledges your good points before countering.
- **Competitive** — Directly quotes your words and explains why you're wrong. Exploits logical weaknesses. Never concedes.
- **Expert** — Brings specific data, historical examples, and pointed rhetorical questions. Challenges every assumption.

When the debate ends, request a **scorecard** from a judge model. It evaluates both debaters on four dimensions (Persuasiveness, Evidence, Coherence, Rebuttal Quality) and gives you specific feedback: your strongest point, your weakest point, an argument you missed, and one concrete tip for improvement.

You get your own Elo rating. It shows up on the leaderboard alongside your models.

---

## The Leaderboard

All roads lead to the leaderboard.

- **Arena tab** — Sortable table with Elo ratings, W/L/D records, sparkline trend charts, and a "Provisional" badge for models with fewer than 10 debates. Your Sparring Ring record appears here too, slotted at the correct Elo position in gold.
- **Benchmark tab** — Radar charts comparing models across scoring categories, with average scores and rank tables.

Elo uses a K-factor decay system: K=40 for new models (first 10 games) so they find their level quickly, K=32 for 10-30 games, K=24 for veterans. Ratings stabilize over time but never stop moving entirely.

---

## Features at a Glance

| Feature | Details |
|---|---|
| Real-time streaming | Both models stream simultaneously in split-pane view (auto-falls back to sequential when combined model params exceed threshold) |
| 3 debate formats | Freestyle, Formal (3-phase), Socratic (question-defense) |
| AI topic suggestions | One-click topic generation from your lightest model |
| Elo rating system | Chess-style ratings with K-factor decay |
| 15-prompt benchmark suite | Coding, creative, analysis, summarization, reasoning |
| 3 scoring methods | Manual stars, auto-judge, blind head-to-head |
| Hardware metrics | Live CPU/RAM/swap graphs during benchmark runs |
| Human vs. AI sparring | 3 difficulty levels with word limits |
| AI judge scorecards | 4-dimension scoring with written feedback |
| Debate history | Searchable, filterable, with expandable transcripts |
| Export everything | Markdown transcripts, JSON suites, CSV leaderboards, Markdown benchmark reports |
| Import test suites | Share benchmark suites across machines |
| Configurable Ollama URL | Point at any Ollama instance, not just localhost |
| Dark theme | Gold and amber accents on slate. The colosseum aesthetic. |

---

## Requirements

- **macOS** (the app is built with Tauri, which bundles a native macOS binary)
- **[Ollama](https://ollama.ai)** running locally (or on a reachable URL)
- At least two models pulled (`ollama pull llama3.2`, `ollama pull qwen3:8b`, etc.)

---

## Getting Started

### Install Ollama & Pull Models

```bash
# Install Ollama (if you haven't)
brew install ollama

# Pull a few models to fight
ollama pull llama3.2
ollama pull qwen3:8b
ollama pull gemma3:4b

# Make sure Ollama is running
ollama serve
```

### Build & Run

```bash
# Clone
git clone https://github.com/saagpatel/ModelColosseum.git
cd ModelColosseum

# Install frontend dependencies
pnpm install

# Run in development mode (opens the app window)
pnpm tauri dev

# Or build a production .app bundle
pnpm tauri build
```

The production build creates a `.app` in `src-tauri/target/release/bundle/macos/`.

### First Run

1. The app checks for Ollama automatically — you'll see a green dot in the top-right if it's connected.
2. Go to **Arena**, click "Refresh Models" to sync your Ollama library.
3. Enter a topic (or hit "Suggest Topics"), pick two models, choose a format, and hit **Start Debate**.
4. Watch them argue. Vote when it's over.
5. Check the **Leaderboard** to see the standings.

---

## Architecture

```
┌─────────────────────────────────────────────┐
│              React 19 Frontend              │
│  (TypeScript strict, Tailwind CSS, Zustand) │
│    Arena │ Benchmark │ Sparring │ Settings   │
└────────────────┬────────────────────────────┘
                 │ Tauri IPC (invoke + events)
┌────────────────┴────────────────────────────┐
│              Rust Backend (Tauri 2)          │
│  debate.rs │ benchmark.rs │ elo.rs          │
│  ollama.rs │ prompts.rs   │ db.rs           │
└────────────────┬────────────────────────────┘
                 │
    ┌────────────┴────────────┐
    │ SQLite (WAL mode)       │ Ollama REST API
    │ ~/.model-colosseum/     │ localhost:11434
    │ colosseum.db            │ (configurable)
    └─────────────────────────┘
```

The frontend is purely presentational — all Ollama communication, database access, Elo calculations, and prompt engineering happen in Rust. The frontend talks to the backend through Tauri's `invoke` (request-response) and `listen` (streaming events) APIs.

**Database:** 13 SQLite tables in WAL mode, stored at `~/.model-colosseum/colosseum.db`. Models, debates, rounds, Elo history, benchmark suites, prompts, results, scores, scorecards, user stats, and settings.

**Streaming:** Ollama returns NDJSON from `/api/generate`. Rust reads the byte stream line-by-line, parses each JSON chunk, sends tokens through a `tokio::mpsc` channel, and emits Tauri events that the React frontend consumes in real time.

---

## Tech Stack

| Layer | Technology |
|---|---|
| Desktop runtime | Tauri 2.x (Rust backend + webview) |
| Frontend | React 19, TypeScript 5.x strict, Vite 6.x |
| Styling | Tailwind CSS 4.x |
| State management | Zustand 5.x |
| Routing | React Router 7.x |
| Charts | Recharts 2.x |
| Database | SQLite via rusqlite 0.31 (bundled) |
| HTTP client | reqwest 0.12 (async streaming) |
| Async runtime | Tokio 1.x |
| System metrics | sysinfo 0.31 |

---

## Project Structure

```
src/                          # React frontend
  components/
    AppShell.tsx              # Nav bar + routing shell
    DebateSetup.tsx           # Topic, model selection, format picker
    DebateViewer.tsx          # Split-pane debate view with voting
    DebatePanel.tsx           # Single model's streaming content
    ModelSelector.tsx         # Model dropdown component
    Skeleton.tsx              # Loading skeleton component
    benchmark/                # Benchmark UI components
      ResultsGrid.tsx         # Prompt x model scoring grid
      StarRating.tsx          # 1-5 star rating input
      BlindCompare.tsx        # Blind A/B comparison overlay
      AutoJudgePanel.tsx      # Auto-judge configuration
      BenchmarkLeaderboard.tsx # Radar charts + rank tables
      ...
  pages/
    Arena.tsx                 # Arena mode page
    Benchmark.tsx             # Benchmark suite editor + runner
    SparringRing.tsx          # Human vs AI debate interface
    Leaderboard.tsx           # Unified leaderboard (arena + benchmark)
    History.tsx               # Searchable debate history
    Settings.tsx              # App configuration
  stores/                     # Zustand state stores
  hooks/                      # Tauri event listener hooks
  types.ts                    # Shared TypeScript interfaces

src-tauri/src/                # Rust backend
  lib.rs                      # Tauri commands + app setup
  db.rs                       # SQLite connection, schema, migrations
  ollama.rs                   # Ollama REST client (health, list, stream)
  debate.rs                   # Arena + Sparring debate engine
  benchmark.rs                # Benchmark runner, scoring, import/export
  elo.rs                      # Elo rating calculations
  prompts.rs                  # System prompt templates for all modes
```

---

## Settings

Access from the Settings tab. Everything persists in SQLite.

| Setting | What It Does |
|---|---|
| Default Rounds | 3, 5, or 7 rounds for new debates |
| Word Limit | Max words per response (100-500) |
| Ollama URL | Point at a different Ollama instance |
| Concurrent Streaming | Stream both models at once (vs. sequential) |
| Sequential Threshold | Auto-fallback to sequential when combined model params exceed this (20-60B) |
| Reset Elo | Nuclear option: reset all models to 1500 and clear history |

---

## Data & Privacy

- **Local only.** The app talks to Ollama on localhost (or your configured URL). No other network calls. No telemetry. No analytics. No cloud.
- **All data** lives in `~/.model-colosseum/colosseum.db`. Delete that file to start fresh.
- **Exports** are saved as local files (Markdown, CSV, JSON) — nothing leaves your machine unless you put it there.

---

## Development

```bash
# Run tests (67 Rust tests)
pnpm test

# Type check
npx tsc --noEmit

# Lint Rust
cd src-tauri && cargo clippy --all-targets -- -D warnings

# Dev mode with hot reload
pnpm tauri dev
```

---

## License

MIT

---

*Built with Rust, React, and an unreasonable amount of opinions about which 8B model argues better.*
