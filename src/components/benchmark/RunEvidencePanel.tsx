import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  BoundaryEvidenceReference,
  BoundaryReason,
  RunEvidence,
} from "../../types";

interface RunEvidencePanelProps {
  runId: number;
  refreshKey?: string;
  availableResultIds?: number[];
  onOpenResult?: (resultId: number, returnFocus: HTMLElement) => void;
}

function formatInterval(lower: number | null, upper: number | null): string {
  if (lower === null || upper === null) return "uncertainty unknown";
  return `${lower.toFixed(2)}–${upper.toFixed(2)}`;
}

function label(code: string): string {
  return code.replaceAll("_", " ");
}

function decisionClass(status: "directional_choice" | "abstain"): string {
  return status === "directional_choice"
    ? "border-emerald-700/60 bg-emerald-500/10 text-emerald-300"
    : "border-amber-700/60 bg-amber-500/10 text-amber-200";
}

export function RunEvidencePanel({
  runId,
  refreshKey = "",
  availableResultIds = [],
  onOpenResult,
}: RunEvidencePanelProps) {
  const [evidence, setEvidence] = useState<RunEvidence | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [unknownsExpanded, setUnknownsExpanded] = useState(false);
  const availableResults = useMemo(() => new Set(availableResultIds), [availableResultIds]);

  useEffect(() => {
    let cancelled = false;
    setEvidence(null);
    setError(null);
    setExpanded(new Set());
    setUnknownsExpanded(false);
    const load = async () => {
      try {
        const data = await invoke<RunEvidence>("get_run_evidence", { runId });
        if (!cancelled) setEvidence(data);
      } catch (err) {
        if (!cancelled) setError(String(err));
      }
    };
    void load();
    return () => {
      cancelled = true;
    };
  }, [runId, refreshKey]);

  if (error) {
    return (
      <div role="alert" className="m-4 rounded-lg border border-red-800 bg-red-500/10 p-4 text-sm text-red-300">
        Run Boundary unavailable: {error}. No decision claim was made.
      </div>
    );
  }
  if (!evidence) {
    return (
      <div role="status" aria-live="polite" className="p-4 text-sm text-slate-500">
        Run Boundary pending — loading run evidence…
      </div>
    );
  }

  const boundary = evidence.boundary;
  const blocking = boundary.boundaries.filter((reason) => reason.disposition === "blocking");
  const advisory = boundary.boundaries.filter((reason) => reason.disposition === "advisory");
  const evidenceReferences = Array.from(
    new Map(
      [
        ...boundary.observations,
        ...boundary.exclusions,
        ...boundary.unknowns,
        ...boundary.boundaries,
      ]
        .flatMap((item) => item.evidence_refs)
        .map((reference) => [`${reference.kind}:${reference.id}`, reference]),
    ).values(),
  );
  const invalid =
    evidence.failed_trials +
    evidence.excluded_trials +
    evidence.cancelled_trials +
    evidence.timeout_trials;

  const toggle = (code: string) => {
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(code)) next.delete(code);
      else next.add(code);
      return next;
    });
  };

  return (
    <section aria-labelledby="run-evidence-heading" className="space-y-5 border-b border-slate-800 p-4 lg:p-6">
      <div>
        <div>
          <p className="text-[11px] font-semibold uppercase tracking-[0.18em] text-gold-500">
            Run Boundary · deterministic evidence record
          </p>
          <h2 id="run-evidence-heading" className="mt-1 text-lg font-bold text-slate-100">
            What this run can honestly support
          </h2>
          <span
            role="status"
            className={`mt-3 inline-flex w-fit rounded-lg border px-3 py-1.5 text-xs font-bold uppercase tracking-wide ${decisionClass(boundary.decision.status)}`}
          >
            {boundary.decision.status === "directional_choice" ? "Directional choice" : "Abstain"}
            <span className="ml-1 font-normal normal-case">· {boundary.decision.evidence_status}</span>
          </span>
          <p className="mt-2 max-w-3xl text-sm font-medium leading-relaxed text-slate-200">
            {boundary.decision.summary}
          </p>
          <p className="mt-1 max-w-3xl text-xs leading-relaxed text-slate-400">
            {boundary.decision.claim_ceiling}
          </p>
        </div>
      </div>

      {blocking.length > 0 ? (
        <div aria-labelledby="run-boundary-blockers">
          <h3 id="run-boundary-blockers" className="text-sm font-semibold text-slate-200">
            Why this run stops here
          </h3>
          <div className="mt-2 space-y-2">
            {blocking.map((reason) => (
              <BoundaryDisclosure
                key={reason.code}
                reason={reason}
                isExpanded={expanded.has(reason.code)}
                onToggle={() => toggle(reason.code)}
                availableResults={availableResults}
                onOpenResult={onOpenResult}
              />
            ))}
          </div>
        </div>
      ) : (
        <p className="rounded-lg border border-emerald-800/50 bg-emerald-500/5 p-3 text-sm text-emerald-200">
          No blocking evidence boundary applies to the named per-capability direction.
        </p>
      )}

      <div className="rounded-xl border border-slate-800 bg-slate-900/60 p-3">
        <button
          type="button"
          aria-expanded={unknownsExpanded}
          aria-controls="run-boundary-unknowns"
          onClick={() => setUnknownsExpanded((value) => !value)}
          className="flex w-full items-center justify-between gap-3 rounded text-left text-sm font-semibold text-slate-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-gold-500"
        >
          <span>Unknown, excluded, or advisory evidence</span>
          <span aria-hidden="true" className="text-slate-500">{unknownsExpanded ? "−" : "+"}</span>
        </button>
        {unknownsExpanded && (
          <div id="run-boundary-unknowns" className="mt-3 grid gap-3 lg:grid-cols-3">
            <BoundaryList title="Unknown or not observed" items={boundary.unknowns.map((item) => item.statement)} />
            <BoundaryList
              title="Excluded from the choice"
              items={boundary.exclusions.map((item) => `${item.statement} ${item.consequence}`)}
            />
            <BoundaryList title="Advisories" items={advisory.map((item) => item.statement)} />
          </div>
        )}
      </div>

      <div className="grid grid-cols-2 gap-2 sm:grid-cols-4 xl:grid-cols-6">
        <Metric label="Measured trials" value={`${evidence.completed_measured_trials}/${evidence.planned_measured_trials}`} />
        <Metric label="Excluded / failed" value={String(invalid)} tone={invalid > 0 ? "warn" : "normal"} />
        <Metric label="Judge sources" value={String(evidence.judge_provenance.length)} />
        <Metric label="Human vote pairs" value={String(evidence.position_bias.sample_size)} />
        <Metric
          label="Human / latest judge"
          value={
            evidence.judge_disagreement.disagreement_rate === null
              ? "unknown"
              : `${Math.round(evidence.judge_disagreement.disagreement_rate * 100)}%`
          }
        />
        <Metric label="Arena Elo" value={evidence.elo_updated ? "updated" : "unchanged"} />
      </div>

      <div className="grid gap-4 xl:grid-cols-[1.1fr_1.9fr]">
        <div className="rounded-xl border border-slate-800 bg-slate-900/70 p-4">
          <h3 className="text-sm font-semibold text-slate-200">Provenance and limits</h3>
          <dl className="mt-3 space-y-2 text-xs">
            <DataRow label="Run status" value={evidence.outcome_status} mono />
            <DataRow label="Manifest" value={evidence.manifest_digest?.slice(0, 16) ?? "legacy / unavailable"} mono />
            <DataRow label="Hardware dependence" value={evidence.hardware_dependent ? "Yes — compare like hardware" : "No"} />
            <DataRow
              label="Position bias"
              value={evidence.position_bias.detected ? "detected" : evidence.position_bias.warning ?? "not detected"}
              warn={evidence.position_bias.detected}
            />
          </dl>
          <div className="mt-4 border-t border-slate-800 pt-3">
            <p className="text-[11px] font-semibold uppercase tracking-wide text-slate-500">Judges</p>
            {evidence.judge_provenance.length === 0 ? (
              <p className="mt-1 text-xs text-amber-300">Unscored: no judge evidence yet.</p>
            ) : (
              <ul className="mt-1 space-y-1 text-xs text-slate-300">
                {evidence.judge_provenance.map((source) => <li key={source}>• {source}</li>)}
              </ul>
            )}
          </div>
        </div>

        <div className="rounded-xl border border-slate-800 bg-slate-900/70 p-4">
          <h3 className="text-sm font-semibold text-slate-200">Capability recommendations</h3>
          {evidence.recommendations.length === 0 ? (
            <p className="mt-3 text-sm text-amber-300">
              Score repeated trials with a human or exact local judge method before choosing a model.
            </p>
          ) : (
            <div className="mt-3 grid gap-2 md:grid-cols-2">
              {evidence.recommendations.map((recommendation) => (
                <article key={recommendation.category} className="rounded-lg border border-slate-800 bg-slate-950/50 p-3">
                  <div className="flex items-center justify-between gap-2">
                    <h4 className="text-xs font-bold uppercase tracking-wide text-slate-300">{recommendation.category}</h4>
                    <span className="rounded bg-slate-800 px-2 py-0.5 text-[10px] text-slate-400">
                      {recommendation.confidence}
                    </span>
                  </div>
                  <p className="mt-2 text-sm font-semibold text-gold-400">
                    {recommendation.recommended_model ?? "No defensible direction"}
                  </p>
                  <p className="mt-1 text-xs leading-relaxed text-slate-500">{recommendation.reason}</p>
                </article>
              ))}
            </div>
          )}
        </div>
      </div>

      {evidence.capability_evidence.length > 0 && (
        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full min-w-[720px] text-left text-xs">
            <thead className="bg-slate-900 text-slate-500">
              <tr>
                <th className="px-3 py-2">Capability</th>
                <th className="px-3 py-2">Model</th>
                <th className="px-3 py-2">Judge</th>
                <th className="px-3 py-2 text-right">Mean</th>
                <th className="px-3 py-2 text-right">Approx. 95% interval</th>
                <th className="px-3 py-2 text-right">n</th>
              </tr>
            </thead>
            <tbody>
              {evidence.capability_evidence.map((entry) => (
                <tr key={`${entry.category}-${entry.model_id}-${entry.scoring_method}`} className="border-t border-slate-800 text-slate-300">
                  <td className="px-3 py-2 capitalize">{entry.category}</td>
                  <td className="px-3 py-2 font-medium">{entry.model_name}</td>
                  <td className="px-3 py-2">{entry.scoring_method.replace("_", " ")}</td>
                  <td className="px-3 py-2 text-right font-mono">{entry.confidence.mean?.toFixed(2) ?? "—"}</td>
                  <td className="px-3 py-2 text-right font-mono">{formatInterval(entry.confidence.lower_95, entry.confidence.upper_95)}</td>
                  <td className={`px-3 py-2 text-right ${entry.confidence.sufficient_sample ? "text-slate-300" : "text-amber-300"}`}>
                    {entry.confidence.sample_size}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-3">
        <h3 className="text-sm font-semibold text-slate-200">Evidence references</h3>
        <p className="mt-1 text-xs leading-relaxed text-slate-500">
          Open locally available result evidence. Other stable references remain visible even when this screen has no matching detail surface.
        </p>
        <div className="mt-3 flex flex-wrap gap-2">
          {evidenceReferences.map((reference) => (
            <EvidenceReferenceLink
              key={`${reference.kind}:${reference.id}`}
              reference={reference}
              availableResults={availableResults}
              onOpenResult={onOpenResult}
            />
          ))}
        </div>
        <p className="mt-3 border-t border-slate-800 pt-3 text-xs leading-relaxed text-slate-400">
          Export Boundary Metadata contains IDs, digests, and decision metadata only. It omits prompt and model output content; no redaction is claimed. Export Full Evidence is separate and may contain prompts and outputs.
        </p>
      </div>
    </section>
  );
}

function BoundaryDisclosure({
  reason,
  isExpanded,
  onToggle,
  availableResults,
  onOpenResult,
}: {
  reason: BoundaryReason;
  isExpanded: boolean;
  onToggle: () => void;
  availableResults: Set<number>;
  onOpenResult?: (resultId: number, returnFocus: HTMLElement) => void;
}) {
  const detailsId = `boundary-${reason.code}`;
  return (
    <article className="rounded-xl border border-amber-900/60 bg-amber-500/5">
      <button
        type="button"
        aria-expanded={isExpanded}
        aria-controls={detailsId}
        onClick={onToggle}
        className="flex w-full items-start justify-between gap-4 rounded-xl p-3 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-gold-500"
      >
        <span>
          <span className="block text-[10px] font-bold uppercase tracking-wide text-amber-400">{label(reason.code)}</span>
          <span className="mt-1 block text-sm leading-relaxed text-slate-200">{reason.statement}</span>
          <span className="mt-1 block text-xs text-slate-500">Scope: {reason.scope}</span>
        </span>
        <span aria-hidden="true" className="mt-1 text-amber-300">{isExpanded ? "−" : "+"}</span>
      </button>
      {isExpanded && (
        <div id={detailsId} className="space-y-3 border-t border-amber-900/40 px-3 py-3">
          <div>
            <p className="text-[10px] font-semibold uppercase tracking-wide text-slate-500">Evidence</p>
            <div className="mt-1 flex flex-wrap gap-2">
              {reason.evidence_refs.map((reference) => (
                <EvidenceReferenceLink
                  key={`${reference.kind}:${reference.id}`}
                  reference={reference}
                  availableResults={availableResults}
                  onOpenResult={onOpenResult}
                />
              ))}
            </div>
          </div>
          {reason.clearance.map((clearance) => (
            <div key={clearance.code} className="rounded-lg border border-slate-800 bg-slate-950/60 p-3">
              <p className="text-[10px] font-semibold uppercase tracking-wide text-gold-500">What would clear this</p>
              <p className="mt-1 text-xs leading-relaxed text-slate-300">{clearance.statement}</p>
              <p className="mt-2 text-xs leading-relaxed text-slate-500">
                Required proof: {clearance.proof_required}
              </p>
              <p className="mt-1 text-[10px] font-semibold uppercase tracking-wide text-slate-600">
                {clearance.new_run_required ? "New run required" : "Existing run may be rescored"}
              </p>
            </div>
          ))}
        </div>
      )}
    </article>
  );
}

function EvidenceReferenceLink({
  reference,
  availableResults,
  onOpenResult,
}: {
  reference: BoundaryEvidenceReference;
  availableResults: Set<number>;
  onOpenResult?: (resultId: number, returnFocus: HTMLElement) => void;
}) {
  const resultId = reference.kind === "result" ? Number(reference.id) : Number.NaN;
  const canOpen = Number.isInteger(resultId) && availableResults.has(resultId) && onOpenResult;
  if (canOpen) {
    return (
      <button
        type="button"
        onClick={(event) => onOpenResult(resultId, event.currentTarget)}
        className="rounded border border-slate-700 bg-slate-800 px-2 py-1 font-mono text-[10px] text-gold-300 hover:bg-slate-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-gold-500"
      >
        Open result {reference.id}
      </button>
    );
  }
  return (
    <span className="rounded border border-slate-800 bg-slate-950/60 px-2 py-1 font-mono text-[10px] text-slate-500">
      {reference.kind} {reference.id} · content unavailable here
    </span>
  );
}

function BoundaryList({ title, items }: { title: string; items: string[] }) {
  return (
    <div>
      <h4 className="text-[10px] font-semibold uppercase tracking-wide text-slate-500">{title}</h4>
      {items.length === 0 ? (
        <p className="mt-1 text-xs text-slate-600">None recorded.</p>
      ) : (
        <ul className="mt-1 space-y-1 text-xs leading-relaxed text-slate-300">
          {items.map((item) => <li key={item}>• {item}</li>)}
        </ul>
      )}
    </div>
  );
}

function DataRow({ label: rowLabel, value, mono = false, warn = false }: { label: string; value: string; mono?: boolean; warn?: boolean }) {
  return (
    <div className="flex justify-between gap-3">
      <dt className="text-slate-500">{rowLabel}</dt>
      <dd className={`${mono ? "font-mono" : ""} ${warn ? "text-amber-300" : "text-slate-300"}`}>{value}</dd>
    </div>
  );
}

function Metric({ label: metricLabel, value, tone = "normal" }: { label: string; value: string; tone?: "normal" | "warn" }) {
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-900/70 px-3 py-2.5">
      <p className="text-[10px] font-semibold uppercase tracking-wide text-slate-600">{metricLabel}</p>
      <p className={`mt-1 font-mono text-sm ${tone === "warn" ? "text-amber-300" : "text-slate-200"}`}>{value}</p>
    </div>
  );
}
