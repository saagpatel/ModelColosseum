import type { RefObject } from "react";

type Side = "a" | "b";

interface DebatePanelProps {
  side: Side;
  modelName: string;
  eloRating: number;
  contentRef: RefObject<HTMLDivElement | null>;
  isStreaming: boolean;
  isWaiting: boolean;
  isComplete: boolean;
}

export function DebatePanel({
  side,
  modelName,
  eloRating,
  contentRef,
  isStreaming,
  isWaiting,
  isComplete,
}: DebatePanelProps) {
  const isPro = side === "a";
  const label = isPro ? "PRO" : "CON";
  const badgeColor = isPro
    ? "bg-emerald-500/20 text-emerald-400 border-emerald-500/30"
    : "bg-red-500/20 text-red-400 border-red-500/30";

  return (
    <div className="flex min-w-0 flex-1 flex-col rounded-xl border border-slate-800 bg-slate-900/50">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-slate-800 px-5 py-3">
        <div className="flex items-center gap-3">
          <span className={`rounded border px-2 py-0.5 text-xs font-bold ${badgeColor}`}>
            {label}
          </span>
          <span className="truncate font-semibold text-slate-200">{modelName}</span>
        </div>
        <span className="rounded bg-slate-800 px-2.5 py-1 font-mono text-xs text-gold-500">
          {eloRating.toFixed(0)}
        </span>
      </div>

      {/* Content */}
      <div className="relative flex-1 overflow-y-auto px-5 py-4" style={{ minHeight: "300px" }}>
        {isWaiting && !isStreaming && !isComplete && (
          <div className="flex h-full items-center justify-center">
            <span className="animate-pulse text-sm text-slate-500">Waiting for opponent...</span>
          </div>
        )}
        <div
          ref={contentRef}
          className="whitespace-pre-wrap text-sm leading-relaxed text-slate-300"
        />
        {isStreaming && <span className="debate-cursor" />}
      </div>
    </div>
  );
}
