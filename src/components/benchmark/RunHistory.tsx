import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { BenchmarkRunSummary } from "../../types";

interface RunHistoryProps {
  onViewRun: (runId: number) => void;
  onCompare: (runA: number, runB: number) => void;
}

function statusBadge(status: string): string {
  switch (status) {
    case "completed":
      return "bg-emerald-500/20 text-emerald-400";
    case "cancelled":
      return "bg-red-500/20 text-red-400";
    case "running":
      return "bg-amber-500/20 text-amber-400";
    default:
      return "bg-slate-500/20 text-slate-400";
  }
}

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}

export function RunHistory({ onViewRun, onCompare }: RunHistoryProps) {
  const [runs, setRuns] = useState<BenchmarkRunSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const load = async () => {
      try {
        const data = await invoke<BenchmarkRunSummary[]>("list_benchmark_runs");
        setRuns(data);
      } catch (err) {
        console.error("list_benchmark_runs error:", err);
        setError(String(err));
      } finally {
        setLoading(false);
      }
    };
    void load();
  }, []);

  const toggleSelect = (id: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else if (next.size < 2) {
        next.add(id);
      }
      return next;
    });
  };

  const handleCompare = () => {
    const [a, b] = [...selected];
    if (a !== undefined && b !== undefined) {
      onCompare(a, b);
    }
  };

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center">
        <span className="animate-pulse text-sm text-slate-500">Loading runs...</span>
      </div>
    );
  }

  if (error) {
    return (
      <div className="px-6 py-4 text-sm text-red-400">{error}</div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      {/* Toolbar */}
      {selected.size === 2 && (
        <div className="shrink-0 flex items-center justify-between border-b border-slate-800 bg-slate-900/60 px-6 py-2">
          <span className="text-xs text-slate-400">2 runs selected</span>
          <button
            onClick={handleCompare}
            className="h-8 rounded-lg bg-gold-500 px-4 text-xs font-bold text-slate-950 transition-colors hover:bg-gold-400 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-gold-400"
          >
            Compare
          </button>
        </div>
      )}

      {runs.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-3 py-24 text-center">
          <div className="text-3xl text-slate-700">📋</div>
          <p className="text-sm font-medium text-slate-500">No benchmark runs yet</p>
          <p className="text-xs text-slate-600">Run a benchmark to see history here</p>
        </div>
      ) : (
        <div className="min-h-0 flex-1 overflow-auto">
          <table className="w-full text-sm">
            <thead className="sticky top-0 bg-slate-900/95 backdrop-blur">
              <tr className="border-b border-slate-700">
                <th className="w-8 px-4 py-3" />
                <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-slate-500">
                  Date
                </th>
                <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-slate-500">
                  Suite
                </th>
                <th className="px-4 py-3 text-center text-xs font-medium uppercase tracking-wider text-slate-500">
                  Models
                </th>
                <th className="px-4 py-3 text-center text-xs font-medium uppercase tracking-wider text-slate-500">
                  Prompts
                </th>
                <th className="px-4 py-3 text-center text-xs font-medium uppercase tracking-wider text-slate-500">
                  Scored
                </th>
                <th className="px-4 py-3 text-center text-xs font-medium uppercase tracking-wider text-slate-500">
                  Status
                </th>
              </tr>
            </thead>
            <tbody>
              {runs.map((run) => (
                <tr
                  key={run.id}
                  className="border-b border-slate-800/50 transition-colors hover:bg-slate-800/30"
                >
                  <td className="px-4 py-3">
                    <input
                      type="checkbox"
                      checked={selected.has(run.id)}
                      onChange={() => toggleSelect(run.id)}
                      disabled={selected.size >= 2 && !selected.has(run.id)}
                      className="h-4 w-4 rounded border-slate-600 accent-gold-500 focus-visible:outline-none"
                    />
                  </td>
                  <td className="px-4 py-3">
                    <button
                      onClick={() => onViewRun(run.id)}
                      className="text-left text-xs text-slate-300 transition-colors hover:text-gold-400 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
                    >
                      {formatDate(run.started_at)}
                    </button>
                  </td>
                  <td className="px-4 py-3">
                    <button
                      onClick={() => onViewRun(run.id)}
                      className="text-left text-sm font-medium text-slate-200 transition-colors hover:text-gold-400 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
                    >
                      {run.suite_name}
                    </button>
                  </td>
                  <td className="px-4 py-3 text-center text-sm text-slate-300">
                    {run.model_count}
                  </td>
                  <td className="px-4 py-3 text-center text-sm text-slate-300">
                    {run.prompt_count}
                  </td>
                  <td className="px-4 py-3 text-center text-xs text-slate-400">
                    {run.scored_count}/{run.total_results}
                  </td>
                  <td className="px-4 py-3 text-center">
                    <span
                      className={`rounded px-2 py-0.5 text-xs font-medium ${statusBadge(run.status)}`}
                    >
                      {run.status}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
