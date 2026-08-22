# Run Boundary V1 Contract

Status: implementation specification

Selected name: **Run Boundary**

`RunBoundaryV1` is a deterministic explanation attached to one Evaluation Lab
run. It answers one question:

> Does this exact run support a scoped model choice, and if not, what exact
> evidence blocks that choice and what would clear the block?

It is not a deployment recommendation, leaderboard, confidence score, or new
evaluation engine.

## Decision states

| State | Meaning | Allowed claim |
|---|---|---|
| `directional_choice` | One capability has a same-method, sufficiently repeated, non-overlapping result under a valid manifest. | A directional choice for that capability and recorded environment only. |
| `abstain` | Evidence is valid, partial, or invalid and does not support a choice. | The named blocker and its clearance condition. |

`evidence_status` separately records `valid`, `partial`, or `invalid`. This
prevents a failed or premeasurement attempt from masquerading as an ordinary
tie. An invalid attempt still emits an `abstain` decision.

## Truth classes

Every statement is one of:

- `observation`: directly present in an immutable manifest, trial, result,
  judge attempt, comparison, or bound external proof record;
- `policy`: a preregistered threshold or product rule;
- `inference`: a deterministic conclusion whose source observations and rule
  are named;
- `unknown`: not observed or not transferable.

Unknown is never converted into pass, zero, or a ranking penalty.

## Boundary taxonomy

Blocking codes are evaluated in this order:

1. `provenance_missing` — required manifest, model digest, runtime identity, or
   source digest is absent.
2. `runtime_identity_unstable` — exact candidate identity changed or could not
   be held through the evidence window.
3. `run_incomplete` — a planned measured trial is failed, timed out, cancelled,
   excluded, empty, or never started.
4. `no_measured_evidence` — no eligible measured result exists.
5. `no_scored_evidence` — measured outputs exist, but no eligible score
   evidence exists yet.
6. `model_failure` — a measured request completed without a usable model
   output, such as an explicitly excluded empty output. Runtime, persistence,
   cancellation, and timeout errors do not become model failures.
7. `judge_method_mismatch` — candidates do not share one scoring method, or
   eligible method-specific comparisons support conflicting directions.
8. `insufficient_samples` — either leading candidate has fewer than three valid
   same-method samples.
9. `quality_gate_failed` — an explicitly recorded deterministic quality floor
   failed. This code is unavailable when no floor was preregistered.
10. `uncertainty_overlap` — the existing descriptive intervals overlap.
11. `constraint_failed` — a named, preregistered latency, memory, reliability,
   or other observed constraint failed.

Advisory codes do not become ranking penalties:

- `judge_disagreement`
- `position_bias_risk`
- `cache_state_unknown`
- `thermal_attribution_unknown`
- `context_effective_value_unknown`
- `transfer_unsupported`
- `development_only_evidence`
- `small_sample_warning`

An implementation may add a code only with a deterministic rule, a required
source field, a claim it blocks, and a clearance condition. Free-form codes are
not allowed.

## Required structure

The normative JSON shape is
`proof/evidence-boundary/run-boundary-v1.schema.json`. At minimum a receipt
contains:

- the source run key and immutable manifest/evidence digests;
- a scoped decision and claim ceiling;
- exact candidate, runtime, workload, and execution-semantics fields, each
  marked `observed`, `declared`, or `unknown`;
- typed boundaries with evidence references and clearance conditions;
- exclusions and unknowns;
- unsupported claims;
- a metadata-only privacy envelope.

## Candidate and execution identity

Where the source records them, identity includes exact model tag, artifact
digest, format, quantization, runtime name/version, host snapshot digest,
effective context readback, cache semantics, concurrency, timeout, retries,
warm-up status, and workload digest. Missing fields remain explicit unknowns.

The first implementation must not invent effective context, prompt-cache,
thermal, or energy facts from timing. It may only surface those dimensions when
an authoritative source record exists.

## Clearance rules

Every blocking boundary has at least one clearance entry containing:

- a stable clearance code;
- a plain-language action;
- the exact proof required;
- whether a new run is required.

Clearance is prospective guidance, not an automatic promise that the next run
will pass. For example, more samples can clear `insufficient_samples`; they do
not guarantee that `uncertainty_overlap` will clear.

## Raw evidence and privacy

The boundary receipt is metadata-only. It may contain stable trial keys, result
IDs, judge-attempt IDs, repository-relative proof locators, and SHA-256 digests.
It must not contain prompt text, model output, judge output, credentials, home
paths, or free-form logs.

Raw evidence remains in the existing local SQLite database and full evidence
bundle. The UI may drill into it locally by ID. A boundary-only export omits raw
content rather than pretending to redact it. V1 performs no automatic
redaction. The existing full bundle remains an explicit separate export and
continues to contain prompts and outputs.

For deterministic re-export, `generated_at` records the source run's start
timestamp. It is not an export timestamp and does not claim that the evidence
was freshly measured when the receipt was saved.

## Claim ceiling

`RunBoundaryV1` can support only a decision already justified by the recorded
run and its deterministic rules. It cannot establish deployment fitness,
cross-host transfer, objective judge correctness, production reliability,
energy use, demand, or product-market fit.

## Primary-source fit

- [lm-evaluation-harness](https://github.com/EleutherAI/lm-evaluation-harness)
  can save results and model-response samples for post-hoc analysis.
- [Inspect logs](https://inspect.aisi.org.uk/eval-logs.html) preserve evaluation
  status, plans, results, errors, metadata, samples, and audit trails.
- [Ollama `/api/ps`](https://docs.ollama.com/api/ps) exposes loaded-model
  digest, format, quantization, memory, and effective context.
- [llama-bench](https://github.com/ggml-org/llama.cpp/blob/master/tools/llama-bench/README.md)
  measures prompt processing and generation across parameters and emits
  machine-readable output, but explicitly omits some end-to-end costs.
- [MLX-LM](https://github.com/ml-explore/mlx-lm) exposes prompt-cache and KV-cache
  mechanisms whose state materially changes performance semantics.
- [MLPerf Client](https://github.com/mlcommons/mlperf_client) standardizes
  client-system benchmark workloads and execution providers.
- [W3C PROV-O](https://www.w3.org/TR/prov-o/) motivates separating activities,
  entities, agents, derivations, and timestamps; V1 reuses that discipline
  without adding an RDF dependency.

These tools provide strong logs, metrics, runtime controls, or standardized
benchmarks. Run Boundary's bounded increment is the deterministic bridge from
those facts to a claim ceiling, abstention reason, and clearance condition for
one local run.
