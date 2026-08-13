# Run Boundary Decision Dossier V1

Selected concept: **Run Boundary**

Decision: **GO_TO_IMPLEMENTATION**

Scope: one bounded upgrade to the existing Evaluation Lab result surface.

## Strongest formulation

For a solo developer comparing already-installed Ollama models on one Mac, Run
Boundary turns one evaluation run into a deterministic answer about whether a
per-capability model choice is supported. When it abstains, it names the exact
blocker, evidence, excluded facts, unknowns, claim ceiling, and proof required
to clear the blocker.

It does not choose a deployment configuration.

## Why this is an upgrade, not a new product

The six-lane CollisionProbeV1 result was `RECAST AS UPGRADE` with complete
coverage. It collided with:

- the current `RunEvidencePanel`;
- the completed immutable Evaluation Lab evidence core;
- the scoped evidence/failure-boundary backlog from Deployment Fit V2.

The material increment is limited to four things: typed reason codes, a stable
claim ceiling, evidence references, and explicit clearance conditions. A new
dashboard, service, database, CLI, or runtime would be false novelty.

## Primary user and decision

User: one solo developer evaluating two local models for a named capability on
their current Mac.

Decision: whether this exact run supports using model A rather than model B for
that capability.

Current failure: aggregate scores and current prose can say “no defensible
winner,” but downstream code and the operator cannot reliably distinguish an
invalid run, insufficient samples, uncertainty overlap, missing provenance,
or unsupported transfer—or see what evidence would clear each state.

## Discriminating proof

The worked fixture uses the frozen Deployment Fit V2 premeasurement record.
Candidate feasibility and one longer-timeout mechanism look promising in
isolation. A throughput-oriented report could still tempt a model choice.

Run Boundary returns `abstain` with `evidence_status: invalid` because:

- candidate identity changed during two freeze attempts;
- no freeze manifest existed;
- zero measured V2 requests ran;
- the holdout remained unopened.

It preserves Gate A and Gate B as observations, excludes them from V2 quality
selection, marks cache and transfer facts unknown, and requires runtime
isolation plus a new preregistration before clearance. The fixture therefore
exposes a decision boundary that an aggregate score cannot represent without
allowing an unsupported deployment conclusion.

## Prior-art decision

Current tools establish that rich logging and benchmarking are commodity:

- lm-evaluation-harness saves result and sample artifacts;
- Inspect provides structured logs, individual samples, errors, metadata, and
  audit history;
- Ollama and llama-bench expose runtime identity and performance parameters;
- MLX-LM makes cache semantics materially configurable;
- MLPerf Client standardizes client inference benchmarking.

ModelColosseum should reuse those patterns, not compete on another logger or
benchmark. The defensible increment is the local deterministic mapping from
existing evidence to a claim boundary and clearance contract.

## Gate decisions

| Gate | Decision | Decisive evidence |
|---|---|---|
| A — Decision clarity | PASS | One user makes one per-capability model-choice decision after a run. |
| B — Evidence leverage | PASS | Immutable manifests, trials, results, judges, comparisons, exports, and raw outputs already exist; no new service or database is required. |
| C — Truthfulness | PASS | The contract separates observation, policy, inference, exclusion, unknown, abstention, and unsupported claims. |
| D — Discriminating value | PASS | The fixture blocks a tempting V2 choice because identity and measured evidence are absent, while preserving useful non-quality observations. |
| E — Integration economy | PASS | A pure Rust derivation extends `RunEvidence` and the existing panel; no migration or dependency is required. |
| F — Privacy and provenance | PASS | The boundary-only receipt is local metadata with IDs and digests; raw content is omitted, and full export remains explicitly separate. |
| G — Non-LLM reliability | PASS | Fixed typed rules derive every outcome; no judge council or generative explanation participates. |
| H — Scope discipline | PASS | The slice has no candidate search, terrain map, telemetry daemon, tournament, or deployment claim. |

## Rejections and mutations

- `Evidence & Abstention Upgrade` → renamed **Run Boundary** because the unit is
  one run, not an abstract platform.
- New report/dashboard → rejected; extend the existing evidence panel.
- Separate CLI → rejected; the current desktop result and JSON bundle are the
  owned surfaces.
- Stored boundary table → rejected; derive from immutable evidence.
- Automatic redaction → rejected for V1; boundary export omits content and says
  so.
- “Winner” or composite score → rejected.
- Deployment Fit card → rejected; Run Boundary cannot select configurations.
- LLM-authored explanation → rejected; deterministic rules own semantics.

## Non-goals

- new inference or workload runs;
- runtime or model acquisition;
- context/cache/thermal instrumentation;
- quality-floor invention for suites that have none;
- cross-host comparison;
- production or demand claims;
- terrain maps, leaderboards, or hardware atlases.

## Delivery shape

The first implementation is bounded to:

1. pure Rust `RunBoundaryV1` derivation and fixtures;
2. an additive `boundary` field in `RunEvidence` and evidence bundle;
3. boundary-first rendering inside `RunEvidencePanel`;
4. local evidence-reference focus;
5. metadata-only boundary export;
6. accessibility, compatibility, and rollback verification.

Estimated effort is 3–5 focused engineering days with no new dependency.

## Failure modes

- prose and code taxonomies drift;
- one blocker hides additional blockers;
- clearance wording promises a pass rather than required evidence;
- missing context/cache fields are inferred from timing;
- boundary export leaks raw content;
- additive JSON breaks a strict downstream consumer;
- UI expansion recreates a dashboard;
- directional evidence is presented as universal choice.

The implementation spec addresses these with one Rust authority, complete
reason lists, typed clearance, explicit unknowns, metadata-only export, and
existing-panel integration.

## Kill criteria

Park Run Boundary if implementation cannot preserve deterministic derivation,
reuse the existing panel and database, keep raw content out of the boundary
receipt, remain backward compatible, or expose a decision-relevant distinction
beyond current prose.

## Claim ceiling and unknowns

This dossier supports an implementation decision for a local, fixture-proven
Evaluation Lab upgrade. It does not prove user demand, usability with real
operators, implementation correctness, app-window behavior, release readiness,
or any Deployment Fit claim. Those remain `UNKNOWN` until their own evidence
exists.

## Final decision

**GO_TO_IMPLEMENTATION**

Implement only the bounded slice above. Deployment Fit remains **PARKED**.
