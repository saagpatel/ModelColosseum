import { useRef, useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DebatePanel } from "./DebatePanel";
import { useDebateEvents } from "../hooks/useDebateEvents";
import { useDebateStore } from "../stores/debateStore";
import { useAppStore } from "../stores/appStore";
import type { VoteResult, DebateFormat } from "../types";

function getPhaseLabel(format: DebateFormat, round: number, totalRounds: number): string {
  if (format === "formal") {
    if (round === 1) return "Opening";
    if (round === 2) return "Rebuttal";
    return "Closing";
  }
  if (format === "socratic") {
    const midpoint = Math.ceil((totalRounds + 1) / 2);
    return round <= midpoint ? "Questions" : "Defense";
  }
  return "";
}

export function DebateViewer() {
  const panelARef = useRef<HTMLDivElement>(null);
  const panelBRef = useRef<HTMLDivElement>(null);
  const [isVoting, setIsVoting] = useState(false);
  const [winnerSide, setWinnerSide] = useState<"a" | "b" | undefined>(undefined);
  const winnerTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (winnerTimerRef.current !== null) {
        clearTimeout(winnerTimerRef.current);
      }
    };
  }, []);

  const {
    phase,
    debateId,
    currentRound,
    totalRounds,
    format,
    mode,
    modelAId,
    modelBId,
    errorMessage,
    eloDeltaA,
    eloDeltaB,
    setVoteResult,
  } = useDebateStore();
  const models = useAppStore((s) => s.models);

  useDebateEvents(debateId, panelARef, panelBRef);

  const modelA = models.find((m) => m.id === modelAId);
  const modelB = models.find((m) => m.id === modelBId);

  const isDebating = phase === "debating";
  const isError = phase === "error";
  const isAborted = phase === "aborted";

  const handleAbort = () => {
    if (debateId !== null) {
      void invoke("abort_debate", { debateId });
    }
  };

  const handleVote = async (winner: string) => {
    if (debateId === null) return;
    setIsVoting(true);
    try {
      const result = await invoke<VoteResult>("vote_debate", { debateId, winner });
      setVoteResult(result.rating_a_before, result.rating_a_after, result.rating_b_before, result.rating_b_after);
      const side = winner === "model_a" ? "a" : winner === "model_b" ? "b" : undefined;
      setWinnerSide(side);
      winnerTimerRef.current = setTimeout(() => {
        setWinnerSide(undefined);
        winnerTimerRef.current = null;
      }, 2000);
    } catch (err) {
      console.error("Vote failed:", err);
    } finally {
      setIsVoting(false);
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
          isComplete={phase === "complete" || phase === "voted" || isError || isAborted}
          eloDelta={eloDeltaA ?? undefined}
          winner={winnerSide}
        />

        {/* Center Column */}
        <div className="flex w-20 shrink-0 flex-col items-center justify-center gap-4">
          <div className={`flex h-12 w-12 items-center justify-center rounded-full border border-gold-500/30 bg-gold-500/10 text-sm font-black text-gold-400 ${isDebating ? "animate-pulse-glow" : ""}`}>
            VS
          </div>

          <div className="text-center">
            <span className="text-xs text-slate-500">Round</span>
            <p className="text-lg font-bold text-slate-200">
              {Math.min(currentRound, totalRounds)}/{totalRounds}
            </p>
          </div>

          {format !== "freestyle" && (
            <span className="rounded bg-gold-500/10 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider text-gold-400">
              {getPhaseLabel(format, Math.min(currentRound, totalRounds), totalRounds)}
            </span>
          )}

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

          {phase === "complete" && (
            <div className="flex flex-col items-center gap-2">
              <span className="text-xs font-medium text-gold-400">Cast Your Vote</span>
              <button
                onClick={() => void handleVote("model_a")}
                className="w-full rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-3 py-1.5 text-xs font-medium text-emerald-400 transition-colors hover:bg-emerald-500/20 disabled:opacity-50"
                disabled={isVoting}
              >
                Left Wins
              </button>
              <button
                onClick={() => void handleVote("draw")}
                className="w-full rounded-lg border border-slate-600 bg-slate-800 px-3 py-1.5 text-xs font-medium text-slate-400 transition-colors hover:bg-slate-700 disabled:opacity-50"
                disabled={isVoting}
              >
                Draw
              </button>
              <button
                onClick={() => void handleVote("model_b")}
                className="w-full rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-1.5 text-xs font-medium text-red-400 transition-colors hover:bg-red-500/20 disabled:opacity-50"
                disabled={isVoting}
              >
                Right Wins
              </button>
            </div>
          )}

          {phase === "voted" && (
            <span className="rounded bg-gold-500/10 px-2 py-1 text-xs font-medium text-gold-400">
              Elo Updated
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
          isComplete={phase === "complete" || phase === "voted" || isError || isAborted}
          eloDelta={eloDeltaB ?? undefined}
          winner={winnerSide}
        />
      </div>
    </div>
  );
}
