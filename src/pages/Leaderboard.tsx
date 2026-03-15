import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { LineChart, Line, ResponsiveContainer } from "recharts";
import type { Model, EloHistoryPoint } from "../types";

type SortKey = "elo_rating" | "arena_wins" | "arena_losses" | "arena_draws" | "total_debates";
type SortDir = "asc" | "desc";

export function Leaderboard() {
  const [models, setModels] = useState<Model[]>([]);
  const [sparklines, setSparklines] = useState<Record<number, EloHistoryPoint[]>>({});
  const [sortKey, setSortKey] = useState<SortKey>("elo_rating");
  const [sortDir, setSortDir] = useState<SortDir>("desc");
  const [loading, setLoading] = useState(true);

  const fetchData = useCallback(async () => {
    setLoading(true);
    try {
      const data = await invoke<Model[]>("get_leaderboard");
      setModels(data);
      const sparks: Record<number, EloHistoryPoint[]> = {};
      await Promise.all(
        data.map(async (m) => {
          try {
            const history = await invoke<EloHistoryPoint[]>("get_model_elo_history", { modelId: m.id });
            sparks[m.id] = history;
          } catch {
            sparks[m.id] = [];
          }
        })
      );
      setSparklines(sparks);
    } catch (err) {
      console.error("Failed to fetch leaderboard:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void fetchData(); }, [fetchData]);

  const sorted = [...models].sort((a, b) => {
    const mul = sortDir === "desc" ? -1 : 1;
    return mul * (a[sortKey] - b[sortKey]);
  });

  const handleSort = (key: SortKey) => {
    if (sortKey === key) {
      setSortDir(sortDir === "desc" ? "asc" : "desc");
    } else {
      setSortKey(key);
      setSortDir("desc");
    }
  };

  const SortHeader = ({ label, field }: { label: string; field: SortKey }) => (
    <th
      className="cursor-pointer px-4 py-3 text-right text-xs font-medium uppercase tracking-wider text-slate-400 transition-colors hover:text-gold-400"
      onClick={() => handleSort(field)}
    >
      {label}
      {sortKey === field && (
        <span className="ml-1">{sortDir === "desc" ? "↓" : "↑"}</span>
      )}
    </th>
  );

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center">
        <span className="animate-pulse text-sm text-slate-500">Loading leaderboard...</span>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col gap-6 p-6">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-black text-slate-100">Leaderboard</h1>
        <button
          onClick={() => void fetchData()}
          className="rounded-lg border border-slate-700 bg-slate-800 px-3 py-1.5 text-xs font-medium text-slate-300 transition-colors hover:bg-slate-700"
        >
          Refresh
        </button>
      </div>

      {models.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-3">
          <div className="text-4xl">⚔️</div>
          <p className="text-sm font-medium text-slate-300">No models rated yet</p>
          <p className="text-xs text-slate-500">Run some arena debates to populate the leaderboard</p>
        </div>
      ) : (
        <div className="min-h-0 flex-1 overflow-auto rounded-xl border border-slate-800">
          <table className="w-full">
            <thead className="sticky top-0 bg-slate-900/95 backdrop-blur">
              <tr className="border-b border-slate-800">
                <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-slate-400">#</th>
                <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-slate-400">Model</th>
                <SortHeader label="Elo" field="elo_rating" />
                <th className="px-4 py-3 text-center text-xs font-medium uppercase tracking-wider text-slate-400">Trend</th>
                <SortHeader label="W" field="arena_wins" />
                <SortHeader label="L" field="arena_losses" />
                <SortHeader label="D" field="arena_draws" />
                <SortHeader label="Debates" field="total_debates" />
              </tr>
            </thead>
            <tbody>
              {sorted.map((model, idx) => {
                const spark = sparklines[model.id] ?? [];
                return (
                  <tr key={model.id} className="border-b border-slate-800/50 transition-colors hover:bg-slate-800/30">
                    <td className="px-4 py-3 text-sm font-bold text-slate-500">{idx + 1}</td>
                    <td className="px-4 py-3">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-semibold text-slate-200">{model.display_name}</span>
                        {model.total_debates < 10 && (
                          <span className="rounded bg-slate-800 px-1.5 py-0.5 text-[10px] text-slate-500">
                            Provisional
                          </span>
                        )}
                      </div>
                    </td>
                    <td className="px-4 py-3 text-right font-mono text-sm font-bold text-gold-400">
                      {model.elo_rating.toFixed(0)}
                    </td>
                    <td className="px-4 py-3">
                      <div className="mx-auto w-20">
                        {spark.length >= 2 ? (
                          <ResponsiveContainer width="100%" height={24}>
                            <LineChart data={spark}>
                              <Line
                                type="monotone"
                                dataKey="rating"
                                stroke="#f59e0b"
                                strokeWidth={1.5}
                                dot={false}
                                isAnimationActive={false}
                              />
                            </LineChart>
                          </ResponsiveContainer>
                        ) : (
                          <span className="block text-center text-[10px] text-slate-600">—</span>
                        )}
                      </div>
                    </td>
                    <td className="px-4 py-3 text-right text-sm text-emerald-400">{model.arena_wins}</td>
                    <td className="px-4 py-3 text-right text-sm text-red-400">{model.arena_losses}</td>
                    <td className="px-4 py-3 text-right text-sm text-slate-400">{model.arena_draws}</td>
                    <td className="px-4 py-3 text-right text-sm text-slate-300">{model.total_debates}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
