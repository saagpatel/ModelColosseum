# Model Colosseum — Discovery Summary

## Problem Statement
Running 6+ Ollama models locally with no systematic way to compare them. Manual A/B testing is time-consuming, subject to recency bias, and produces no persistent record. Generic benchmarks (MMLU, HumanEval) don't measure what matters for daily use: argumentation quality, creative writing, summarization clarity, and instruction following against YOUR prompts.

## Target User
Primary: Solo developer/IT engineer running multiple Ollama models on an M4 Pro (48GB). Uses models daily for coding assistance, writing, analysis, and creative tasks. Wants definitive answers about which model to use for which task type.

Secondary: Local LLM community members who want a visual, entertaining way to compare models beyond benchmarks. High GitHub showcase potential.

## Success Metrics
1. Arena debate: 5 rounds between two 14B models completes in < 3 minutes
2. Benchmark suite: 15 prompts × 6 models completes in < 20 minutes
3. Elo ratings stabilize (< 50 point variance) after 15+ debates per model
4. App cold launch to first interaction < 2 seconds
5. Database handles 1,000+ transcripts with < 100ms leaderboard queries

## Scope Boundaries

**In scope:**
- Arena mode: model-vs-model debates with Elo
- Benchmark mode: custom test suites, manual + auto-judge scoring, performance metrics
- Sparring Ring: structured human-vs-AI debates with scorecards
- Unified leaderboard with category filtering
- Debate history and transcript export (Markdown)
- Test suite import/export (JSON)
- Dark theme, arena aesthetic
- macOS desktop app (Tauri 2.0)

**Out of scope:**
- Cloud model support (OpenAI, Anthropic APIs)
- Voice synthesis or audio input
- Real-time multiplayer / audience voting
- Windows or Linux builds
- Fine-tuning models based on results
- Light theme

**Deferred to later phases:**
- Tournament bracket mode (Phase 4+)
- Custom system prompt editor in UI (Phase 4)
- Quantization comparison workflows (Phase 4+)
- Prompt template library / community sharing (post-v1)

## Technical Constraints
- Local-only: all data stays on machine, no outbound network except localhost
- Ollama REST API is the sole LLM interface (no direct model loading)
- macOS-only (Apple Silicon: M4 Pro 48GB primary, M3 secondary)
- SQLite single-file database (no external DB dependencies)
- Concurrent streaming limited by Ollama's `OLLAMA_NUM_PARALLEL` setting
- Combined model parameters > 40B triggers sequential fallback

## Key Integrations
| Service | API | Auth Method | Rate Limits | Purpose |
|---------|-----|-------------|-------------|---------|
| Ollama | REST (localhost:11434) | None (local) | None (limited by hardware) | Model inference, metadata, health check |
| SQLite | rusqlite (embedded) | N/A | N/A | All persistence: debates, scores, Elo, settings |
| System (sysinfo) | Rust crate | N/A | N/A | CPU/memory monitoring during benchmarks |
