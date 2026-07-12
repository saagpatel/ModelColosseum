import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { RunComparability, RunComparisonEntry } from "../../types";
import { downloadBlob } from "../../utils/download";

interface RunComparisonProps {
  runA: number;
  runB: number;
  onBack: () => void;
}

type PromptCategory =
  | "coding"
  | "creative"
  | "analysis"
  | "summarization"
  | "reasoning"
  | "conversation"
  | "instruction";

const categoryBadge: Record<string, string> = {
  coding: "bg-blue-500/20 text-blue-400",
  creative: "bg-purple-500/20 text-purple-400",
  analysis: "bg-emerald-500/20 text-emerald-400",
  summarization: "bg-amber-500/20 text-amber-400",
  reasoning: "bg-rose-500/20 text-rose-400",
  conversation: "bg-cyan-500/20 text-cyan-400",
  instruction: "bg-orange-500/20 text-orange-400",
};

function badgeClass(category: string): string {
  return categoryBadge[category as PromptCategory] ?? "bg-slate-500/20 text-slate-400";
}

function deltaClass(delta: number | null): string {
  if (delta === null || delta === 0) return "text-slate-500";
  return delta > 0 ? "text-emerald-400" : "text-red-400";
}

function deltaLabel(delta: number | null): string {
  if (delta === null || delta === 0) return "—";
  return delta > 0 ? `+${delta.toFixed(1)}` : delta.toFixed(1);
}

export function RunComparison({ runA, runB, onBack }: RunComparisonProps) {
  const [entries, setEntries] = useState<RunComparisonEntry[]>([]);
  const [comparability, setComparability] = useState<RunComparability | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const exportReceipt = async () => {
    try {
      const json = await invoke<string>("export_reproduction_receipt", { runA, runB });
      downloadBlob(json, `reproduction-receipt-${runA}-vs-${runB}.json`, "application/json");
    } catch (err) {
      setError(String(err));
    }
  };

  useEffect(() => {
    const load = async () => {
      try {
        const [data, comparisonStatus] = await Promise.all([
          invoke<RunComparisonEntry[]>("get_run_comparison", { runA, runB }),
          invoke<RunComparability>("get_run_comparability", { runA, runB }),
        ]);
        setEntries(data);
        setComparability(comparisonStatus);
      } catch (err) {
        console.error("get_run_comparison error:", err);
        setError(String(err));
      } finally {
        setLoading(false);
      }
    };
    void load();
  }, [runA, runB]);

  // Group by category
  const byCategory = new Map<string, RunComparisonEntry[]>();
  for (const e of entries) {
    const list = byCategory.get(e.prompt_category) ?? [];
    list.push(e);
    byCategory.set(e.prompt_category, list);
  }

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="flex shrink-0 items-center gap-4 border-b border-slate-800 px-6 py-4">
        <button
          onClick={onBack}
          className="flex items-center gap-1.5 text-sm text-slate-400 transition-colors hover:text-slate-200 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
        >
          ← Back
        </button>
        <h2 className="text-base font-semibold text-slate-100">
          Run #{runA} vs Run #{runB}
        </h2>
        <button
          onClick={() => void exportReceipt()}
          className="ml-auto rounded-lg border border-slate-700 px-3 py-2 text-xs font-semibold text-slate-200 hover:border-gold-500 hover:text-gold-300 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-gold-400"
        >
          Export reproduction receipt
        </button>
      </div>

      {loading && (
        <div className="flex flex-1 items-center justify-center">
          <span className="animate-pulse text-sm text-slate-500">Loading comparison...</span>
        </div>
      )}

      {error && (
        <div className="px-6 py-4 text-sm text-red-400">{error}</div>
      )}

      {!loading && !error && (
        <div className="min-h-0 flex-1 overflow-auto">
          <div
            role="status"
            className={`m-4 rounded-lg border p-4 ${comparability?.comparable ? "border-emerald-800 bg-emerald-500/10" : "border-amber-800 bg-amber-500/10"}`}
          >
            <p className={`text-sm font-semibold ${comparability?.comparable ? "text-emerald-300" : "text-amber-200"}`}>
              {comparability?.classification === "exact_reproduction" && "Exact reproduction — quality and performance may be compared"}
              {comparability?.classification === "hardware_variant" && "Hardware-variant reproduction — compare quality, not speed"}
              {comparability?.classification === "runtime_variant" && "Runtime-variant replay — exploratory only"}
              {comparability?.classification === "incomparable" && "Runs are incomparable — no recommendation may be transferred"}
            </p>
            {comparability && (
              <p className="mt-2 text-xs text-slate-300">
                Quality: {comparability.quality_comparable ? "comparable" : "not comparable"} · Performance: {comparability.performance_comparable ? "comparable" : "hardware/runtime dependent"}
              </p>
            )}
            {comparability && comparability.reasons.length > 0 && (
              <ul className="mt-2 space-y-1 text-xs text-amber-300/80">
                {comparability.reasons.map((reason) => <li key={reason}>• {reason}</li>)}
              </ul>
            )}
          </div>
          <table className="w-full text-sm">
            <thead className="sticky top-0 bg-slate-900/95 backdrop-blur">
              <tr className="border-b border-slate-700">
                <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-slate-500">
                  Prompt
                </th>
                <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-slate-500">
                  Model
                </th>
                <th className="px-4 py-3 text-center text-xs font-medium uppercase tracking-wider text-slate-500">
                  Run A
                </th>
                <th className="px-4 py-3 text-center text-xs font-medium uppercase tracking-wider text-slate-500">
                  Run B
                </th>
                <th className="px-4 py-3 text-center text-xs font-medium uppercase tracking-wider text-slate-500">
                  Delta
                </th>
              </tr>
            </thead>
            <tbody>
              {[...byCategory.entries()].map(([category, rows]) => (
                <>
                  <tr key={`cat-${category}`} className="border-b border-slate-800 bg-slate-950">
                    <td colSpan={5} className="px-4 py-1.5">
                      <span
                        className={`rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wider ${badgeClass(category)}`}
                      >
                        {category}
                      </span>
                    </td>
                  </tr>
                  {rows.map((row, idx) => (
                    <tr
                      key={`${row.prompt_id}-${row.model_id}-${idx}`}
                      className="border-b border-slate-800/50 transition-colors hover:bg-slate-800/20"
                    >
                      <td className="px-4 py-3 text-xs text-slate-300">{row.prompt_title}</td>
                      <td className="px-4 py-3 text-xs text-slate-400">{row.model_name}</td>
                      <td className="px-4 py-3 text-center font-mono text-xs text-slate-300">
                        {row.run_a_score !== null ? row.run_a_score.toFixed(1) : "—"}
                      </td>
                      <td className="px-4 py-3 text-center font-mono text-xs text-slate-300">
                        {row.run_b_score !== null ? row.run_b_score.toFixed(1) : "—"}
                      </td>
                      <td className={`px-4 py-3 text-center font-mono text-xs font-bold ${deltaClass(row.score_delta)}`}>
                        {deltaLabel(row.score_delta)}
                      </td>
                    </tr>
                  ))}
                </>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
