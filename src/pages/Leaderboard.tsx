import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { LineChart, Line, ResponsiveContainer } from "recharts";
import { BenchmarkLeaderboard } from "../components/benchmark/BenchmarkLeaderboard";
import { downloadBlob } from "../utils/download";
import type { Model, EloHistoryPoint, UserStats } from "../types";

type SortKey = "elo_rating" | "arena_wins" | "arena_losses" | "arena_draws" | "total_debates";
type SortDir = "asc" | "desc";

const activeTabClass = "rounded-md bg-gold-500 px-4 py-1.5 text-sm font-bold text-slate-950";
const inactiveTabClass =
  "rounded-md px-4 py-1.5 text-sm font-medium text-slate-400 hover:text-slate-200 transition-colors";

export function Leaderboard() {
  const [tab, setTab] = useState<"arena" | "benchmark">("arena");
  const [models, setModels] = useState<Model[]>([]);
  const [sparklines, setSparklines] = useState<Record<number, EloHistoryPoint[]>>({});
  const [sortKey, setSortKey] = useState<SortKey>("elo_rating");
  const [sortDir, setSortDir] = useState<SortDir>("desc");
  const [loading, setLoading] = useState(true);
  const [userStats, setUserStats] = useState<UserStats | null>(null);

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
    try {
      const stats = await invoke<UserStats>("get_user_stats");
      if (stats.total_debates > 0) setUserStats(stats);
    } catch {
      // no sparring history yet
    }
  }, []);

  useEffect(() => { void fetchData(); }, [fetchData]);

  const sorted = [...models].sort((a, b) => {
    const mul = sortDir === "desc" ? -1 : 1;
    return mul * (a[sortKey] - b[sortKey]);
  });

  type LeaderboardEntry =
    | { type: "model"; data: Model }
    | { type: "you"; data: UserStats };

  const entries: LeaderboardEntry[] = sorted.map((m) => ({ type: "model" as const, data: m }));
  if (userStats) {
    const idx = entries.findIndex(
      (e) => e.type === "model" && e.data.elo_rating < userStats.elo_rating
    );
    const youEntry: LeaderboardEntry = { type: "you", data: userStats };
    if (idx === -1) entries.push(youEntry);
    else entries.splice(idx, 0, youEntry);
  }

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
      aria-sort={sortKey === field ? (sortDir === "desc" ? "descending" : "ascending") : "none"}
    >
      {label}
      {sortKey === field && (
        <span className="ml-1">{sortDir === "desc" ? "↓" : "↑"}</span>
      )}
    </th>
  );

  if (loading && tab === "arena") {
    return (
      <div className="flex h-full flex-col gap-6 p-6">
        <div className="flex items-center justify-between">
          <div className="h-7 w-40 animate-pulse rounded bg-slate-800" />
        </div>
        <div className="min-h-0 flex-1 overflow-auto rounded-xl border border-slate-800">
          <table className="w-full">
            <thead className="bg-slate-900/95">
              <tr className="border-b border-slate-800">
                <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-slate-400">#</th>
                <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-slate-400">Model</th>
                <th className="px-4 py-3 text-right text-xs font-medium uppercase tracking-wider text-slate-400">Elo</th>
                <th className="px-4 py-3 text-center text-xs font-medium uppercase tracking-wider text-slate-400">Trend</th>
                <th className="px-4 py-3 text-right text-xs font-medium uppercase tracking-wider text-slate-400">W</th>
                <th className="px-4 py-3 text-right text-xs font-medium uppercase tracking-wider text-slate-400">L</th>
                <th className="px-4 py-3 text-right text-xs font-medium uppercase tracking-wider text-slate-400">D</th>
                <th className="px-4 py-3 text-right text-xs font-medium uppercase tracking-wider text-slate-400">Debates</th>
              </tr>
            </thead>
            <tbody>
              {Array.from({ length: 6 }, (_, i) => (
                <tr key={i} className="border-b border-slate-800/50">
                  <td className="px-4 py-3"><div className="h-4 w-6 animate-pulse rounded bg-slate-800" /></td>
                  <td className="px-4 py-3"><div className="h-4 w-32 animate-pulse rounded bg-slate-800" /></td>
                  <td className="px-4 py-3"><div className="ml-auto h-4 w-12 animate-pulse rounded bg-slate-800" /></td>
                  <td className="px-4 py-3"><div className="mx-auto h-4 w-20 animate-pulse rounded bg-slate-800" /></td>
                  <td className="px-4 py-3"><div className="ml-auto h-4 w-8 animate-pulse rounded bg-slate-800" /></td>
                  <td className="px-4 py-3"><div className="ml-auto h-4 w-8 animate-pulse rounded bg-slate-800" /></td>
                  <td className="px-4 py-3"><div className="ml-auto h-4 w-8 animate-pulse rounded bg-slate-800" /></td>
                  <td className="px-4 py-3"><div className="ml-auto h-4 w-10 animate-pulse rounded bg-slate-800" /></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col gap-6 p-6">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-black text-slate-100">Leaderboard</h1>
        <div className="flex items-center gap-3">
          <div className="flex gap-1 rounded-lg bg-slate-800 p-1">
            <button onClick={() => setTab("arena")} className={tab === "arena" ? activeTabClass : inactiveTabClass}>
              Arena
            </button>
            <button onClick={() => setTab("benchmark")} className={tab === "benchmark" ? activeTabClass : inactiveTabClass}>
              Benchmark
            </button>
          </div>
          {tab === "arena" && (
            <>
              <button
                onClick={() => void fetchData()}
                className="rounded-lg border border-slate-700 bg-slate-800 px-3 py-1.5 text-xs font-medium text-slate-300 transition-colors hover:bg-slate-700 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
              >
                Refresh
              </button>
              <button
                onClick={async () => {
                  try {
                    const csv = await invoke<string>("export_leaderboard");
                    downloadBlob(csv, "leaderboard.csv", "text/csv");
                  } catch (err) {
                    console.error("export_leaderboard error:", err);
                  }
                }}
                className="rounded-lg border border-slate-700 bg-slate-800 px-3 py-1.5 text-xs font-medium text-slate-300 transition-colors hover:bg-slate-700 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
              >
                Export CSV
              </button>
            </>
          )}
        </div>
      </div>

      {tab === "benchmark" && (
        <div className="min-h-0 flex-1 overflow-auto">
          <BenchmarkLeaderboard />
        </div>
      )}

      {tab === "arena" && (
        models.length === 0 ? (
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
                {entries.map((entry, idx) => {
                  if (entry.type === "you") {
                    const u = entry.data;
                    return (
                      <tr key="you" className="border-b border-slate-800/50 border-l-2 border-l-gold-500 bg-gold-500/5 transition-colors hover:bg-gold-500/10">
                        <td className="px-4 py-3 text-sm font-bold text-gold-500">{idx + 1}</td>
                        <td className="px-4 py-3">
                          <div className="flex items-center gap-2">
                            <span className="text-sm font-bold text-gold-400">You</span>
                            <span className="rounded bg-gold-500/10 px-1.5 py-0.5 text-[10px] text-gold-500">
                              Sparring
                            </span>
                          </div>
                        </td>
                        <td className="px-4 py-3 text-right font-mono text-sm font-bold text-gold-400">
                          {u.elo_rating.toFixed(0)}
                        </td>
                        <td className="px-4 py-3">
                          <span className="block text-center text-[10px] text-slate-600">—</span>
                        </td>
                        <td className="px-4 py-3 text-right text-sm text-emerald-400">{u.wins}</td>
                        <td className="px-4 py-3 text-right text-sm text-red-400">{u.losses}</td>
                        <td className="px-4 py-3 text-right text-sm text-slate-400">{u.draws}</td>
                        <td className="px-4 py-3 text-right text-sm text-slate-300">{u.total_debates}</td>
                      </tr>
                    );
                  }
                  const model = entry.data;
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
        )
      )}
    </div>
  );
}
