import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  RadarChart,
  Radar,
  PolarGrid,
  PolarAngleAxis,
  PolarRadiusAxis,
  ResponsiveContainer,
  Legend,
  Tooltip,
} from "recharts";
import type { BenchmarkLeaderboardEntry } from "../../types";

const RADAR_COLORS = [
  "#f59e0b",
  "#3b82f6",
  "#10b981",
  "#ef4444",
  "#8b5cf6",
  "#ec4899",
  "#06b6d4",
  "#f97316",
];

export function BenchmarkLeaderboard() {
  const [entries, setEntries] = useState<BenchmarkLeaderboardEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const load = async () => {
      try {
        const data = await invoke<BenchmarkLeaderboardEntry[]>("get_benchmark_leaderboard");
        setEntries(data);
      } catch (err) {
        console.error("get_benchmark_leaderboard error:", err);
        setError(String(err));
      } finally {
        setLoading(false);
      }
    };
    void load();
  }, []);

  if (loading) {
    return (
      <div className="flex h-64 items-center justify-center">
        <span className="animate-pulse text-sm text-slate-500">Loading benchmark leaderboard...</span>
      </div>
    );
  }

  if (error) {
    return <div className="py-4 text-sm text-red-400">{error}</div>;
  }

  if (entries.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center gap-3 py-24 text-center">
        <div className="text-4xl text-slate-700">📊</div>
        <p className="text-sm font-medium text-slate-400">No benchmark results yet</p>
        <p className="text-xs text-slate-600">Run and score a benchmark to populate this leaderboard</p>
      </div>
    );
  }

  // Collect all categories across all entries
  const allCategories = [
    ...new Set(entries.flatMap((e) => Object.keys(e.category_scores))),
  ].sort();

  // Build radar chart data: one object per category
  const radarData = allCategories.map((cat) => {
    const row: Record<string, string | number> = { category: cat };
    for (const entry of entries) {
      row[entry.display_name] = entry.category_scores[cat] ?? 0;
    }
    return row;
  });

  return (
    <div className="space-y-8">
      {/* Radar chart */}
      <div className="rounded-xl border border-slate-800 bg-slate-900 p-6">
        <h2 className="mb-4 text-sm font-semibold text-slate-300">Category Scores</h2>
        <ResponsiveContainer width="100%" height={320}>
          <RadarChart data={radarData}>
            <PolarGrid stroke="#334155" />
            <PolarAngleAxis
              dataKey="category"
              tick={{ fill: "#94a3b8", fontSize: 11 }}
            />
            <PolarRadiusAxis
              angle={30}
              domain={[0, 10]}
              tick={{ fill: "#64748b", fontSize: 9 }}
            />
            {entries.map((entry, i) => (
              <Radar
                key={entry.model_id}
                name={entry.display_name}
                dataKey={entry.display_name}
                stroke={RADAR_COLORS[i % RADAR_COLORS.length]}
                fill={RADAR_COLORS[i % RADAR_COLORS.length]}
                fillOpacity={0.1}
                strokeWidth={2}
              />
            ))}
            <Legend
              wrapperStyle={{ fontSize: "12px", color: "#94a3b8" }}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: "#0f172a",
                border: "1px solid #334155",
                borderRadius: "8px",
                fontSize: "12px",
              }}
            />
          </RadarChart>
        </ResponsiveContainer>
      </div>

      {/* Performance table */}
      <div className="rounded-xl border border-slate-800 bg-slate-900">
        <div className="border-b border-slate-800 px-6 py-3">
          <h2 className="text-sm font-semibold text-slate-300">Performance</h2>
        </div>
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-slate-800">
              <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-slate-500">
                Model
              </th>
              <th className="px-4 py-3 text-center text-xs font-medium uppercase tracking-wider text-slate-500">
                Avg Score
              </th>
              <th className="px-4 py-3 text-center text-xs font-medium uppercase tracking-wider text-slate-500">
                Avg TPS
              </th>
              <th className="px-4 py-3 text-center text-xs font-medium uppercase tracking-wider text-slate-500">
                Avg TTFT
              </th>
              <th className="px-4 py-3 text-center text-xs font-medium uppercase tracking-wider text-slate-500">
                Prompts Scored
              </th>
            </tr>
          </thead>
          <tbody>
            {entries.map((entry, i) => (
              <tr
                key={entry.model_id}
                className="border-b border-slate-800/50 transition-colors hover:bg-slate-800/20"
              >
                <td className="px-4 py-3">
                  <div className="flex items-center gap-2">
                    <span
                      className="h-2.5 w-2.5 rounded-full"
                      style={{ backgroundColor: RADAR_COLORS[i % RADAR_COLORS.length] }}
                    />
                    <span className="font-medium text-slate-200">{entry.display_name}</span>
                  </div>
                </td>
                <td className="px-4 py-3 text-center font-mono text-sm text-gold-400">
                  {entry.avg_score !== null ? entry.avg_score.toFixed(2) : "—"}
                </td>
                <td className="px-4 py-3 text-center font-mono text-xs text-slate-300">
                  {entry.avg_tps !== null ? `${entry.avg_tps.toFixed(1)} t/s` : "—"}
                </td>
                <td className="px-4 py-3 text-center font-mono text-xs text-slate-300">
                  {entry.avg_ttft_ms !== null ? `${entry.avg_ttft_ms.toFixed(0)}ms` : "—"}
                </td>
                <td className="px-4 py-3 text-center text-sm text-slate-300">
                  {entry.total_prompts_scored}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Category breakdown table */}
      {allCategories.length > 0 && (
        <div className="rounded-xl border border-slate-800 bg-slate-900">
          <div className="border-b border-slate-800 px-6 py-3">
            <h2 className="text-sm font-semibold text-slate-300">Category Breakdown</h2>
          </div>
          <div className="overflow-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-slate-800">
                  <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-slate-500">
                    Model
                  </th>
                  {allCategories.map((cat) => (
                    <th
                      key={cat}
                      className="px-3 py-3 text-center text-xs font-medium uppercase tracking-wider text-slate-500"
                    >
                      {cat}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {entries.map((entry) => (
                  <tr
                    key={entry.model_id}
                    className="border-b border-slate-800/50 transition-colors hover:bg-slate-800/20"
                  >
                    <td className="px-4 py-3 text-sm font-medium text-slate-200">
                      {entry.display_name}
                    </td>
                    {allCategories.map((cat) => {
                      const score = entry.category_scores[cat];
                      return (
                        <td
                          key={cat}
                          className="px-3 py-3 text-center font-mono text-xs text-slate-300"
                        >
                          {score !== undefined ? score.toFixed(1) : "—"}
                        </td>
                      );
                    })}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
