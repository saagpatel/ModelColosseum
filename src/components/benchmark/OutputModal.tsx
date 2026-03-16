import { useEffect, useRef } from "react";
import type { BenchmarkResult } from "../../types";
import { StarRating } from "./StarRating";

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

function fmt(n: number): string {
  return n.toLocaleString();
}

interface OutputModalProps {
  result: BenchmarkResult;
  blindLabel: string | null;
  onClose: () => void;
  onScoreChange: (resultId: number, score: number) => void;
}

export function OutputModal({ result, blindLabel, onClose, onScoreChange }: OutputModalProps) {
  const overlayRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [onClose]);

  const modelLabel = blindLabel ?? result.model_name;
  const tps = result.tokens_per_second !== null ? result.tokens_per_second.toFixed(1) : "—";
  const ttft = result.time_to_first_token_ms !== null ? `${result.time_to_first_token_ms}ms` : "—";
  const totalTime = (result.total_time_ms / 1000).toFixed(2);

  return (
    <div
      ref={overlayRef}
      className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/80 backdrop-blur-sm"
      onClick={(e) => {
        if (e.target === overlayRef.current) onClose();
      }}
    >
      <div className="flex max-h-[90vh] w-full max-w-3xl flex-col rounded-xl border border-slate-700 bg-slate-900 shadow-2xl">
        {/* Header */}
        <div className="flex shrink-0 items-start justify-between border-b border-slate-700 px-6 py-4">
          <div className="min-w-0 flex-1">
            <div className="mb-1 flex items-center gap-2">
              <span className="text-base font-bold text-slate-100">{modelLabel}</span>
              <span
                className={`rounded px-1.5 py-0.5 text-xs font-medium ${badgeClass(result.prompt_category)}`}
              >
                {result.prompt_category}
              </span>
            </div>
            <p className="text-sm text-slate-400">{result.prompt_title}</p>
          </div>
          <button
            onClick={onClose}
            className="ml-4 flex h-8 w-8 shrink-0 items-center justify-center rounded-lg text-slate-500 transition-colors hover:bg-slate-800 hover:text-slate-200 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
          >
            ✕
          </button>
        </div>

        {/* Output */}
        <div className="min-h-0 flex-1 overflow-y-auto p-6">
          <pre className="whitespace-pre-wrap font-mono text-sm leading-relaxed text-slate-200">
            {result.output}
          </pre>
        </div>

        {/* Footer */}
        <div className="shrink-0 border-t border-slate-700 px-6 py-4">
          {/* Metrics */}
          <div className="mb-4 flex flex-wrap gap-4 text-xs text-slate-400">
            <span>
              <span className="text-slate-500">Tokens: </span>
              <span className="font-mono text-slate-200">{fmt(result.tokens_generated)}</span>
            </span>
            <span>
              <span className="text-slate-500">TPS: </span>
              <span className="font-mono text-slate-200">{tps}</span>
            </span>
            <span>
              <span className="text-slate-500">TTFT: </span>
              <span className="font-mono text-slate-200">{ttft}</span>
            </span>
            <span>
              <span className="text-slate-500">Total: </span>
              <span className="font-mono text-slate-200">{totalTime}s</span>
            </span>
          </div>

          {/* Scoring */}
          <div className="flex flex-wrap items-center gap-6">
            <div className="flex items-center gap-2">
              <span className="text-xs text-slate-500">Manual score:</span>
              <StarRating
                value={result.manual_score}
                onChange={(score) => onScoreChange(result.id, score)}
              />
            </div>
            {result.auto_judge_score !== null && (
              <div className="flex items-center gap-2">
                <span className="text-xs text-slate-500">Auto-judge:</span>
                <span className="rounded bg-blue-500/20 px-2 py-0.5 text-xs font-bold text-blue-400">
                  {result.auto_judge_score}/10
                </span>
                {result.auto_judge_notes && (
                  <span className="max-w-xs truncate text-xs text-slate-500">
                    {result.auto_judge_notes}
                  </span>
                )}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
