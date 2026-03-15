import { useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DebatePanel } from "./DebatePanel";
import { useDebateEvents } from "../hooks/useDebateEvents";
import { useDebateStore } from "../stores/debateStore";
import { useAppStore } from "../stores/appStore";

export function DebateViewer() {
  const panelARef = useRef<HTMLDivElement>(null);
  const panelBRef = useRef<HTMLDivElement>(null);

  const { phase, debateId, currentRound, totalRounds, mode, modelAId, modelBId, errorMessage } =
    useDebateStore();
  const models = useAppStore((s) => s.models);

  useDebateEvents(debateId, panelARef, panelBRef);

  const modelA = models.find((m) => m.id === modelAId);
  const modelB = models.find((m) => m.id === modelBId);

  const isDebating = phase === "debating";
  const isComplete = phase === "complete";
  const isError = phase === "error";
  const isAborted = phase === "aborted";

  const handleAbort = () => {
    if (debateId !== null) {
      void invoke("abort_debate", { debateId });
    }
  };

  return (
    <div className="flex h-full flex-col gap-4">
      {/* Split Pane */}
      <div className="flex min-h-0 flex-1 gap-4">
        {/* Panel A */}
        <DebatePanel
          side="a"
          modelName={modelA?.display_name ?? "Model A"}
          eloRating={modelA?.elo_rating ?? 1500}
          contentRef={panelARef}
          isStreaming={isDebating}
          isWaiting={false}
          isComplete={isComplete || isError || isAborted}
        />

        {/* Center Column */}
        <div className="flex w-20 shrink-0 flex-col items-center justify-center gap-4">
          <div className="flex h-12 w-12 items-center justify-center rounded-full border border-gold-500/30 bg-gold-500/10 text-sm font-black text-gold-400">
            VS
          </div>

          <div className="text-center">
            <span className="text-xs text-slate-500">Round</span>
            <p className="text-lg font-bold text-slate-200">
              {Math.min(currentRound, totalRounds)}/{totalRounds}
            </p>
          </div>

          {mode && (
            <span className="rounded bg-slate-800 px-2 py-0.5 text-[10px] uppercase tracking-wider text-slate-500">
              {mode}
            </span>
          )}

          {isDebating && (
            <button
              onClick={handleAbort}
              className="rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-1.5 text-xs font-medium text-red-400 transition-colors hover:bg-red-500/20"
            >
              Abort
            </button>
          )}

          {isComplete && (
            <span className="rounded bg-gold-500/10 px-2 py-1 text-xs font-medium text-gold-400">
              Awaiting Vote
            </span>
          )}

          {isError && (
            <span className="max-w-full break-words text-center text-xs text-red-400">
              {errorMessage ?? "Error"}
            </span>
          )}

          {isAborted && (
            <span className="text-xs text-slate-500">Aborted</span>
          )}
        </div>

        {/* Panel B */}
        <DebatePanel
          side="b"
          modelName={modelB?.display_name ?? "Model B"}
          eloRating={modelB?.elo_rating ?? 1500}
          contentRef={panelBRef}
          isStreaming={isDebating}
          isWaiting={false}
          isComplete={isComplete || isError || isAborted}
        />
      </div>
    </div>
  );
}
