import { useState } from "react";
import type { BenchmarkResult } from "../../types";
import { StarRating } from "./StarRating";
import { OutputModal } from "./OutputModal";

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

interface ResultsGridProps {
  results: BenchmarkResult[];
  blindMode: boolean;
  onScoreChange: (resultId: number, score: number) => void;
}

export function ResultsGrid({ results, blindMode, onScoreChange }: ResultsGridProps) {
  const [openResult, setOpenResult] = useState<BenchmarkResult | null>(null);

  if (results.length === 0) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 py-24 text-center">
        <div className="text-3xl text-slate-700">📊</div>
        <h3 className="text-sm font-medium text-slate-500">No results to display</h3>
      </div>
    );
  }

  // Derive sorted model list
  const modelIds = [...new Set(results.map((r) => r.model_id))].sort((a, b) => a - b);
  const modelNames = new Map<number, string>();
  for (const r of results) {
    modelNames.set(r.model_id, r.model_name);
  }

  const blindLabels = new Map<number, string>();
  modelIds.forEach((id, i) => {
    blindLabels.set(id, `Model ${String.fromCharCode(65 + i)}`);
  });

  function modelLabel(modelId: number): string {
    return blindMode
      ? (blindLabels.get(modelId) ?? "Unknown")
      : (modelNames.get(modelId) ?? "Unknown");
  }

  // Group prompts by category
  const promptOrder: number[] = [];
  const seenPrompts = new Set<number>();
  for (const r of results) {
    if (!seenPrompts.has(r.prompt_id)) {
      seenPrompts.add(r.prompt_id);
      promptOrder.push(r.prompt_id);
    }
  }

  const promptMeta = new Map<number, { title: string; category: string }>();
  for (const r of results) {
    if (!promptMeta.has(r.prompt_id)) {
      promptMeta.set(r.prompt_id, { title: r.prompt_title, category: r.prompt_category });
    }
  }

  // Group prompts by category (maintain order within category)
  const byCategory = new Map<string, number[]>();
  for (const pid of promptOrder) {
    const meta = promptMeta.get(pid);
    if (!meta) continue;
    const list = byCategory.get(meta.category) ?? [];
    list.push(pid);
    byCategory.set(meta.category, list);
  }

  // Build lookup: promptId -> modelId -> result
  const lookup = new Map<number, Map<number, BenchmarkResult>>();
  for (const r of results) {
    let inner = lookup.get(r.prompt_id);
    if (!inner) {
      inner = new Map();
      lookup.set(r.prompt_id, inner);
    }
    inner.set(r.model_id, r);
  }

  // Per-prompt fastest TPS
  const fastestTps = new Map<number, number>();
  for (const r of results) {
    if (r.tokens_per_second !== null) {
      const cur = fastestTps.get(r.prompt_id) ?? 0;
      if (r.tokens_per_second > cur) fastestTps.set(r.prompt_id, r.tokens_per_second);
    }
  }

  // Per-model avg score (for column headers)
  const modelScoreSums = new Map<number, { sum: number; count: number }>();
  for (const r of results) {
    if (r.manual_score !== null) {
      const cur = modelScoreSums.get(r.model_id) ?? { sum: 0, count: 0 };
      modelScoreSums.set(r.model_id, { sum: cur.sum + r.manual_score, count: cur.count + 1 });
    }
  }

  function avgScore(modelId: number): string {
    const s = modelScoreSums.get(modelId);
    if (!s || s.count === 0) return "—";
    return (s.sum / s.count).toFixed(1);
  }

  const blindLabel = openResult
    ? blindMode
      ? (blindLabels.get(openResult.model_id) ?? null)
      : null
    : null;

  return (
    <div className="min-h-0 flex-1 overflow-auto">
      {openResult && (
        <OutputModal
          result={openResult}
          blindLabel={blindLabel}
          onClose={() => setOpenResult(null)}
          onScoreChange={(id, score) => {
            onScoreChange(id, score);
          }}
        />
      )}

      <table className="w-full border-collapse text-sm">
        <thead className="sticky top-0 z-10 bg-slate-900/95 backdrop-blur">
          <tr className="border-b border-slate-700">
            <th className="w-48 px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-slate-500">
              Prompt
            </th>
            {modelIds.map((mid) => (
              <th
                key={mid}
                className="px-3 py-3 text-left text-xs font-medium text-slate-300"
              >
                <div className="font-semibold">{modelLabel(mid)}</div>
                <div className="text-[10px] font-normal text-slate-500">
                  avg {avgScore(mid)}/5
                </div>
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {[...byCategory.entries()].map(([category, promptIds]) => (
            <>
              {/* Category group header */}
              <tr key={`cat-${category}`} className="border-b border-slate-800 bg-slate-950">
                <td
                  colSpan={modelIds.length + 1}
                  className="px-4 py-1.5"
                >
                  <span
                    className={`rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wider ${badgeClass(category)}`}
                  >
                    {category}
                  </span>
                </td>
              </tr>
              {promptIds.map((pid) => {
                const meta = promptMeta.get(pid);
                const fastest = fastestTps.get(pid);
                return (
                  <tr
                    key={pid}
                    className="border-b border-slate-800/50 transition-colors hover:bg-slate-800/20"
                  >
                    {/* Prompt title */}
                    <td className="w-48 px-4 py-3 align-top">
                      <span className="block text-xs font-medium leading-tight text-slate-300">
                        {meta?.title ?? "Unknown"}
                      </span>
                    </td>

                    {/* One cell per model */}
                    {modelIds.map((mid) => {
                      const r = lookup.get(pid)?.get(mid);
                      const isFastest =
                        r?.tokens_per_second !== null &&
                        r?.tokens_per_second !== undefined &&
                        fastest !== undefined &&
                        r.tokens_per_second === fastest &&
                        fastest > 0;

                      return (
                        <td
                          key={mid}
                          className={`px-3 py-3 align-top ${isFastest ? "bg-emerald-500/10" : ""}`}
                        >
                          {r ? (
                            <button
                              onClick={() => setOpenResult(r)}
                              className="w-full text-left focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
                            >
                              {/* Output preview */}
                              <p className="mb-1.5 line-clamp-2 text-xs leading-relaxed text-slate-400">
                                {r.output.slice(0, 100)}
                                {r.output.length > 100 ? "…" : ""}
                              </p>

                              {/* Metrics */}
                              <div className="mb-1.5 flex flex-wrap gap-1.5 text-[10px] text-slate-500">
                                {r.tokens_per_second !== null && (
                                  <span
                                    className={`font-mono ${isFastest ? "text-emerald-400" : ""}`}
                                  >
                                    {r.tokens_per_second.toFixed(1)} t/s
                                  </span>
                                )}
                                <span className="font-mono">
                                  {(r.total_time_ms / 1000).toFixed(1)}s
                                </span>
                              </div>

                              {/* Manual score */}
                              <div onClick={(e) => e.stopPropagation()}>
                                <StarRating
                                  value={r.manual_score}
                                  onChange={(score) => onScoreChange(r.id, score)}
                                  size="sm"
                                />
                              </div>

                              {/* Auto-judge badge */}
                              {r.auto_judge_score !== null && (
                                <span className="mt-1 inline-block rounded bg-blue-500/20 px-1.5 py-0.5 text-[10px] font-medium text-blue-400">
                                  Auto: {r.auto_judge_score}/10
                                </span>
                              )}
                            </button>
                          ) : (
                            <span className="text-xs text-slate-600">—</span>
                          )}
                        </td>
                      );
                    })}
                  </tr>
                );
              })}
            </>
          ))}
        </tbody>
      </table>
    </div>
  );
}
