# Evaluation methodology

ModelColosseum is a local evaluation lab with an arena theme. Its decision question is:

> Given these exact local models, this hardware/runtime, and this task suite, which model should I use for each capability, and how uncertain is that choice?

The app does not treat one score, one vote, or one fast generation as a universal model ranking.

## Evidence semantics

| Evidence | Classification | What it supports | Important limit |
|---|---|---|---|
| Raw model output | Model/runtime dependent | Inspection and replay explanation | A recorded seed does not guarantee bit-for-bit output across different Ollama, model, or hardware versions |
| Tokens/second, time to first token, total time | Hardware dependent | Choosing a model on the recorded machine/runtime | Do not compare across different hardware snapshots |
| Human 1–5 score | Human judged | A named operator's assessment of one output | Not blind unless performed through blind comparison |
| Local auto-judge 1–10 score | Model judged | Directional quality evidence under the recorded judge tag/digest and judge prompt | The judge can be biased or disagree with humans |
| Blind left/right/tie vote | Human judged and position randomized | Pairwise preference without model-name exposure | Position bias remains unknown below six votes |
| Arena Elo | Human debate outcome | Playful arena history | Evaluation trials never update Arena Elo |

Legacy runs created without a manifest remain readable but are labeled legacy/incomparable. They are not silently upgraded into reproducible evidence.

## Reproducible run protocol

Before any generation, the app health-checks the local loopback Ollama endpoint and creates one immutable SQLite manifest containing:

- exact Ollama model tags and available digests;
- quantization, parameter size, size, family, modification time, and capabilities when Ollama reports them;
- Ollama server version and loopback endpoint;
- complete suite and prompt snapshots with SHA-256 digests;
- system prompts, evaluation criteria, temperature, token cap, reasoning-mode flag, timeout, and recorded random seed;
- OS, kernel, architecture, CPU, logical CPU count, and memory snapshot;
- planned warm-up and measured trial counts.

The manifest cannot be updated or deleted through SQLite after insertion. Each planned trial is persisted before model execution with its identity, execution order, generation seed, warm-up/measured classification, and comparison position.

Default protocol:

- 3 measured repetitions per prompt/model;
- 1 recorded warm-up per model, excluded from scoring;
- randomized measured execution order;
- balanced randomized left/right positions for two-model comparisons;
- 120-second timeout per generation;
- temperature 0.2 and a 1,024-token cap.
- model thinking disabled so the token budget measures user-visible output; this can be enabled and is recorded.

Operators can change these values before a run. The exact choices are part of the manifest and export.

## Validity and comparability

A run is internally comparable only when every planned measured trial completes with a non-empty output under the same immutable manifest. Cancelled, timed-out, failed, or empty-output trials remain visible and are excluded. Any such trial withholds the model recommendation for that run.

Cross-run comparison classifies the relationship instead of collapsing every mismatch into one binary result:

- `exact_reproduction`: suite and prompt digest, exact model tags and digests, generation settings, Ollama version, and hardware/runtime snapshot match. Quality and performance deltas may be compared.
- `hardware_variant`: evaluation identity and Ollama version match, but hardware differs. Quality evidence may be compared; TTFT, throughput, memory, and other performance deltas remain hardware-dependent.
- `runtime_variant`: evaluation identity matches but the Ollama version differs. Results are exploratory because a runtime change can affect generation and performance.
- `incomparable`: a run is invalid/incomplete or the suite, model identity, or generation settings differ. No recommendation transfers between the runs.

The comparison screen exports a reproduction receipt containing both complete manifests, their stored digests, the classification, and every mismatch reason. The receipt explains the comparison without modifying either immutable run.

The classification considers:

- suite/prompt digest;
- exact model tags and digests;
- Ollama server version;
- hardware/runtime snapshot;
- repetitions, warm-ups, timeout, temperature, token cap, and reasoning mode.

Different recorded seeds are allowed across otherwise comparable repeated runs.

## Uncertainty and recommendations

Scores are summarized independently by capability, model, and judge method. Manual and auto-judge scores are never averaged together. Human scores are normalized from 1–5 to 2–10 only within the human-score lane.

The app reports the sample mean and an approximate 95% interval using `mean ± 1.96 × standard error`. This interval is descriptive, not a claim of population-level benchmark validity. Fewer than three valid samples is explicitly insufficient. A capability recommendation is directional only when:

1. the run is complete and comparable;
2. at least two models have scores from the same judge method;
3. both leading models have at least three valid samples; and
4. their approximate 95% intervals do not overlap.

Otherwise the result is shown as insufficient or inconclusive. There is no universal recommendation across capabilities.

Human-versus-auto-judge disagreement is calculated only for comparison pairs that have both a human blind vote and completed auto-judge scores for both outputs. Left/right preference is flagged when at least six votes exist and the left-side preference differs from 50% by at least 20 percentage points.

## Failure, cancellation, and raw evidence

Trial states are `pending`, `running`, `completed`, `cancelled`, `timeout`, `failed`, or `excluded`. Empty output is preserved as a raw result but marked excluded. Judge attempts separately preserve judge identity/settings, raw judge output, parse failures, cancellation, and errors.

The JSON evidence export contains the immutable manifest, evidence summary, every trial, every raw measured result, every judge attempt, and every human comparison assignment/outcome. It is the preferred reproduction and audit artifact. The older Markdown report remains for human-readable legacy compatibility.

## Migration and rollback

The migration is additive. It adds model provenance columns, run metadata, immutable manifests, trial records, comparison records, and judge-attempt records. Existing suites, prompts, benchmark runs, results, scores, debates, and Elo history are preserved. Applying the migration more than once is tested and safe.

Before a production upgrade, back up `~/.model-colosseum/colosseum.db` together with its WAL/SHM files while the app is stopped. Rolling back to the previous app version leaves the additive tables/columns unused and keeps legacy behavior available, but the previous version cannot interpret new reproducibility evidence. Full schema rollback requires restoring that pre-upgrade backup; the app does not run destructive down-migrations.

## Local-only boundary

Evaluation calls accept only loopback Ollama URLs (`localhost`, `127.0.0.1`, or `::1`). The app does not add cloud judging, telemetry, accounts, or model downloads.
