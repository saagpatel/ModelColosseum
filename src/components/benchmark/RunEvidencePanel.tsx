import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { RunEvidence } from "../../types";

interface RunEvidencePanelProps {
  runId: number;
  refreshKey?: string;
}

function formatInterval(lower: number | null, upper: number | null): string {
  if (lower === null || upper === null) return "uncertainty unknown";
  return `${lower.toFixed(2)}–${upper.toFixed(2)}`;
}

function statusClass(comparable: boolean): string {
  return comparable
    ? "border-emerald-700/60 bg-emerald-500/10 text-emerald-300"
    : "border-amber-700/60 bg-amber-500/10 text-amber-200";
}

export function RunEvidencePanel({ runId, refreshKey = "" }: RunEvidencePanelProps) {
  const [evidence, setEvidence] = useState<RunEvidence | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const data = await invoke<RunEvidence>("get_run_evidence", { runId });
        if (!cancelled) {
          setEvidence(data);
          setError(null);
        }
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
        Evidence summary unavailable: {error}
      </div>
    );
  }
  if (!evidence) {
    return <div className="p-4 text-sm text-slate-500">Loading run evidence…</div>;
  }

  const invalid =
    evidence.failed_trials +
    evidence.excluded_trials +
    evidence.cancelled_trials +
    evidence.timeout_trials;

  return (
    <section aria-labelledby="run-evidence-heading" className="space-y-4 border-b border-slate-800 p-4 lg:p-6">
      <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
        <div>
          <p className="text-[11px] font-semibold uppercase tracking-[0.18em] text-gold-500">
            Reproducibility record
          </p>
          <h2 id="run-evidence-heading" className="mt-1 text-lg font-bold text-slate-100">
            What this run can honestly support
          </h2>
          <p className="mt-1 max-w-3xl text-sm text-slate-400">
            {evidence.comparability_notes ?? "No comparability note was recorded."}
          </p>
        </div>
        <span
          role="status"
          className={`w-fit rounded-lg border px-3 py-1.5 text-xs font-bold uppercase tracking-wide ${statusClass(evidence.comparable)}`}
        >
          {evidence.comparable ? "Comparable" : "Recommendation withheld"}
        </span>
      </div>

      <div className="grid grid-cols-2 gap-2 sm:grid-cols-4 xl:grid-cols-6">
        <Metric label="Measured trials" value={`${evidence.completed_measured_trials}/${evidence.planned_measured_trials}`} />
        <Metric label="Excluded / failed" value={String(invalid)} tone={invalid > 0 ? "warn" : "normal"} />
        <Metric label="Judge sources" value={String(evidence.judge_provenance.length)} />
        <Metric label="Human vote pairs" value={String(evidence.position_bias.sample_size)} />
        <Metric
          label="Judge disagreement"
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
            <div className="flex justify-between gap-3">
              <dt className="text-slate-500">Run status</dt>
              <dd className="font-mono text-slate-300">{evidence.outcome_status}</dd>
            </div>
            <div className="flex justify-between gap-3">
              <dt className="text-slate-500">Manifest</dt>
              <dd className="max-w-[14rem] truncate font-mono text-slate-300" title={evidence.manifest_digest ?? "Unavailable"}>
                {evidence.manifest_digest?.slice(0, 16) ?? "legacy / unavailable"}
              </dd>
            </div>
            <div className="flex justify-between gap-3">
              <dt className="text-slate-500">Hardware dependence</dt>
              <dd className="text-slate-300">{evidence.hardware_dependent ? "Yes — compare like hardware" : "No"}</dd>
            </div>
            <div className="flex justify-between gap-3">
              <dt className="text-slate-500">Position bias</dt>
              <dd className={evidence.position_bias.detected ? "text-amber-300" : "text-slate-300"}>
                {evidence.position_bias.detected ? "detected" : evidence.position_bias.warning ?? "not detected"}
              </dd>
            </div>
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
              Score repeated trials with a human or local auto-judge before choosing a model.
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
                    {recommendation.recommended_model ?? "No defensible winner"}
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
    </section>
  );
}

function Metric({ label, value, tone = "normal" }: { label: string; value: string; tone?: "normal" | "warn" }) {
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-900/70 px-3 py-2.5">
      <p className="text-[10px] font-semibold uppercase tracking-wide text-slate-600">{label}</p>
      <p className={`mt-1 font-mono text-sm ${tone === "warn" ? "text-amber-300" : "text-slate-200"}`}>{value}</p>
    </div>
  );
}
