# Run Boundary V1 Integration Specification

Decision target: smallest implementation slice only

## User and job

The first user is a solo developer evaluating two already-installed Ollama
models on one Mac for a named local task suite. After a run, they must decide
whether the evidence supports choosing one model for one capability.

Their current failure is not lack of scores. It is that `No defensible winner`
and `Recommendation withheld` do not provide a stable machine-readable reason,
the exact evidence behind that reason, or the proof needed to clear it.

## Product unit

Run Boundary is an additive section inside the existing **What this run can
honestly support** panel. It is not a route, dashboard, CLI, database, runtime,
or general deployment card.

The magical moment is a boundary-first answer:

1. `ABSTAIN — runtime identity was unstable before measurement.`
2. The operator opens the blocker and sees two failed freeze records and zero
   measured cases.
3. Candidate feasibility and longer timeout evidence remain visible but are
   explicitly excluded from quality selection.
4. The receipt says a dedicated runtime lease plus a new preregistered holdout
   is required; clicking the old faster result cannot produce a winner claim.

## Current architecture fit

| Existing surface | Reuse | Bounded change |
|---|---|---|
| `src-tauri/src/evaluation.rs` | Pure uncertainty, comparability, and eligibility rules | Add pure Run Boundary types and derivation rules. |
| `src-tauri/src/benchmark.rs` | Immutable manifest, trials, scores, judge attempts, `get_run_evidence`, JSON export | Derive `RunBoundaryV1` from already-loaded evidence; attach it to `RunEvidence`. |
| `src/components/benchmark/RunEvidencePanel.tsx` | Existing truthful-support panel and accessible status surface | Render primary boundary, ordered blockers, unknowns, and clearance actions. |
| `src/components/benchmark/ResultsGrid.tsx` and `OutputModal.tsx` | Existing raw-output inspection | Accept evidence-reference focus/filter callbacks; do not duplicate raw content. |
| Evaluation bundle v2 | Manifest, evidence, trials, results, judges, comparisons | Add nested `evidence.boundary`; existing replay importer already treats evidence as JSON value. |

No database migration is required for the first slice. The receipt is derived
from current immutable records. Dimensions not present in those records are
emitted as unknown, never inferred.

## Backend contract

Add these Rust concepts:

- `RunBoundary`
- `BoundaryDecision`
- `BoundaryReason`
- `BoundaryClearance`
- `EvidenceReference`
- `ObservedField<T>` with `observed | declared | unknown`

Add a pure function:

```text
derive_run_boundary(manifest, run_state, trials, capability_evidence,
                    recommendations, judge_state, comparison_state)
    -> RunBoundary
```

Rule evaluation order is fixed:

1. provenance and identity;
2. run completeness and measured evidence;
3. same-method score eligibility;
4. sample sufficiency;
5. explicit quality or constraint floors when present;
6. interval separation;
7. advisory disagreement, bias, observability, and transfer limits.

The first blocking rule sets the summary. All applicable rules remain in the
receipt. No score averaging, LLM judge, or free-form model explanation occurs.

## UI hierarchy

Inside `RunEvidencePanel`:

1. decision badge: `Directional choice` or `Abstain`;
2. one-sentence claim ceiling;
3. ordered blocking boundaries;
4. `What would clear this` action under each blocker;
5. collapsed `Unknown or not observed` list;
6. existing capability evidence and recommendations;
7. local evidence links by stable result/trial ID;
8. `Export boundary receipt` next to the existing full evidence export.

There is no winner badge. Directional recommendations remain per capability.

## Interaction states

- Loading: keep the existing skeleton text and mark the decision as pending.
- Empty legacy run: `ABSTAIN — provenance missing`; do not synthesize a
  manifest.
- Incomplete run: show the exact failed, excluded, timed-out, and cancelled
  trial keys.
- Contradictory state: fail closed with `provenance_missing` or
  `runtime_identity_unstable`; show both sources.
- Stale external proof: treat it as excluded evidence, not current truth.
- Error: retain the current `role="alert"` behavior and make no decision claim.
- Raw evidence absent: keep the reference and label its content unavailable.

Keyboard and focus:

- every boundary disclosure is a native button;
- evidence links receive visible focus and move focus to the referenced result;
- expanded content follows its trigger in DOM order;
- the badge is not color-only and uses visible text;
- the single-column order remains intact below 768 px;
- wide evidence tables keep their existing horizontal scroll behavior.

## Privacy and export

Two explicit export modes exist:

1. **Boundary receipt** — metadata only; no prompts or outputs; no redaction is
   claimed because content is omitted.
2. **Full evidence bundle** — existing local export containing prompt and model
   content; UI copy must state this before save.

The boundary is computed on demand and adds no retention store. Everything
remains loopback/local-only. No telemetry, account, sync, or cloud judge is
added.

## Backward compatibility

- Legacy runs emit `provenance_missing` and remain readable.
- Existing `RunEvidence` fields remain unchanged; `boundary` is additive.
- Evaluation bundle version remains 2 because replay already treats `evidence`
  as an opaque JSON value. Add a nested `schema_version` to the boundary.
- Unknown nested fields must be ignored by old consumers; a strict downstream
  consumer must continue using the existing bundle version contract.
- Rollback removes the derived UI and field; no stored data needs migration.

## Dependencies and cost

New dependencies: **none**.

Estimated implementation: 3–5 focused engineering days.

- 1–2 days: Rust types, deterministic derivation, fixtures, export.
- 1–2 days: existing-panel UI, evidence focus, responsive/accessibility states.
- 1 day: tests, documentation, and app-window verification.

The main risk is rule drift between prose, Rust, TypeScript, and exports. Keep
one Rust derivation as authority and serialize its result to every consumer.

## Test strategy

Pure Rust fixtures must cover:

- complete directional run;
- incomplete run with mixed failure states;
- no scores;
- one model or mismatched judge methods;
- fewer than three samples;
- overlapping and separated intervals;
- legacy manifest absence;
- judge disagreement and position-bias advisories;
- stable reason ordering;
- every blocking reason has a clearance action;
- no raw prompt/output content in boundary-only export.

Frontend tests or app-window checks must cover loading, empty, error, legacy,
abstain, directional, keyboard disclosure, focus transfer, and narrow layout.
Existing export, replay, cancellation, comparability, and Elo tests remain
required.

## Implementation sequence

1. Add pure Rust types and fixture tests.
2. Attach the derived boundary to `get_run_evidence` and bundle export.
3. Add the boundary-only save command.
4. Render it inside the existing panel and wire evidence focus.
5. Verify accessibility, responsive states, full export compatibility, and
   rollback.

Do not add context readback, cache instrumentation, thermal telemetry, a new
runtime, or Deployment Fit logic in this slice. Those fields stay unknown.

## Kill criteria

Stop implementation if any of these becomes true:

- the boundary requires a new database or service;
- the UI cannot reuse `RunEvidencePanel`;
- reason derivation needs an LLM;
- the boundary duplicates current prose without stable codes and clearance;
- raw content leaks into the boundary-only receipt;
- existing exports or legacy runs cannot remain compatible;
- the slice expands into candidate search, constraint optimization, or a map.
