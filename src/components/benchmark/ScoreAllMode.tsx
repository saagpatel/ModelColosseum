import { useBenchmarkStore } from "../../stores/benchmarkStore";
import { StarRating } from "./StarRating";

interface ScoreAllModeProps {
  onScoreChange: (resultId: number, score: number) => void;
}

export function ScoreAllMode({ onScoreChange }: ScoreAllModeProps) {
  const {
    results,
    blindMode,
    scoreAllPromptIndex,
    toggleBlindMode,
    exitScoreAllMode,
    nextPrompt,
    prevPrompt,
  } = useBenchmarkStore();

  // Derive unique prompt IDs in order
  const promptIds: number[] = [];
  const seenIds = new Set<number>();
  for (const r of results) {
    if (!seenIds.has(r.prompt_id)) {
      seenIds.add(r.prompt_id);
      promptIds.push(r.prompt_id);
    }
  }

  const currentPromptId = promptIds[scoreAllPromptIndex];
  const promptResults = results.filter((r) => r.prompt_id === currentPromptId);
  const firstResult = promptResults[0];

  // Blind mode label mapping: sort by model_id for determinism
  const modelIds = [...new Set(results.map((r) => r.model_id))].sort((a, b) => a - b);
  const blindLabels = new Map<number, string>();
  modelIds.forEach((id, i) => {
    blindLabels.set(id, `Model ${String.fromCharCode(65 + i)}`);
  });

  function modelLabel(modelId: number, modelName: string): string {
    return blindMode ? (blindLabels.get(modelId) ?? "Unknown") : modelName;
  }

  if (!currentPromptId || !firstResult) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-slate-500">
        No prompts to score
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col bg-slate-950">
      {/* Toolbar */}
      <div className="flex shrink-0 items-center justify-between border-b border-slate-800 px-6 py-3">
        <button
          onClick={exitScoreAllMode}
          className="flex items-center gap-1.5 text-sm text-slate-400 transition-colors hover:text-slate-200 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
        >
          ← Back to Grid
        </button>

        <div className="flex items-center gap-3">
          <span className="text-xs text-slate-500">
            Prompt {scoreAllPromptIndex + 1} of {promptIds.length}
          </span>
          <button
            onClick={toggleBlindMode}
            className={`rounded-lg px-3 py-1.5 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500 ${
              blindMode
                ? "bg-gold-500 text-slate-950"
                : "bg-slate-800 text-slate-300 hover:bg-slate-700"
            }`}
          >
            {blindMode ? "Blind ON" : "Blind OFF"}
          </button>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={prevPrompt}
            disabled={scoreAllPromptIndex === 0}
            className="h-8 rounded-lg bg-slate-800 px-3 text-xs font-medium text-slate-300 transition-colors hover:bg-slate-700 disabled:cursor-not-allowed disabled:opacity-40 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
          >
            ← Prev
          </button>
          <button
            onClick={nextPrompt}
            disabled={scoreAllPromptIndex >= promptIds.length - 1}
            className="h-8 rounded-lg bg-slate-800 px-3 text-xs font-medium text-slate-300 transition-colors hover:bg-slate-700 disabled:cursor-not-allowed disabled:opacity-40 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
          >
            Next →
          </button>
        </div>
      </div>

      {/* Prompt header */}
      <div className="shrink-0 border-b border-slate-800 bg-slate-900/50 px-6 py-4">
        <h2 className="text-base font-semibold text-slate-100">{firstResult.prompt_title}</h2>
        <span
          className={`mt-1 inline-block rounded px-1.5 py-0.5 text-xs font-medium ${
            {
              coding: "bg-blue-500/20 text-blue-400",
              creative: "bg-purple-500/20 text-purple-400",
              analysis: "bg-emerald-500/20 text-emerald-400",
              summarization: "bg-amber-500/20 text-amber-400",
              reasoning: "bg-rose-500/20 text-rose-400",
              conversation: "bg-cyan-500/20 text-cyan-400",
              instruction: "bg-orange-500/20 text-orange-400",
            }[firstResult.prompt_category] ?? "bg-slate-500/20 text-slate-400"
          }`}
        >
          {firstResult.prompt_category}
        </span>
      </div>

      {/* Model outputs side by side */}
      <div className="min-h-0 flex-1 overflow-auto">
        <div
          className="grid h-full"
          style={{ gridTemplateColumns: `repeat(${promptResults.length}, minmax(0, 1fr))` }}
        >
          {promptResults.map((r) => (
            <div
              key={r.id}
              className="flex flex-col border-r border-slate-800 last:border-r-0"
            >
              {/* Model header */}
              <div className="shrink-0 border-b border-slate-800 px-4 py-3">
                <p className="text-sm font-semibold text-slate-200">
                  {modelLabel(r.model_id, r.model_name)}
                </p>
                <div className="mt-1 flex flex-wrap gap-2 text-[10px] text-slate-500">
                  {r.tokens_per_second !== null && (
                    <span className="font-mono">{r.tokens_per_second.toFixed(1)} t/s</span>
                  )}
                  <span className="font-mono">{(r.total_time_ms / 1000).toFixed(2)}s</span>
                </div>
              </div>

              {/* Output */}
              <div className="min-h-0 flex-1 overflow-y-auto p-4">
                <pre className="whitespace-pre-wrap font-mono text-xs leading-relaxed text-slate-300">
                  {r.output}
                </pre>
              </div>

              {/* Score */}
              <div className="shrink-0 border-t border-slate-800 px-4 py-3">
                <div className="flex items-center gap-2">
                  <span className="text-xs text-slate-500">Score:</span>
                  <StarRating
                    value={r.manual_score}
                    onChange={(score) => onScoreChange(r.id, score)}
                  />
                </div>
                {r.auto_judge_score !== null && (
                  <span className="mt-1 inline-block rounded bg-blue-500/20 px-1.5 py-0.5 text-[10px] font-medium text-blue-400">
                    Auto: {r.auto_judge_score}/10
                  </span>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
