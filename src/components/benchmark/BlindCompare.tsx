import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { BlindComparison, BlindReveal } from "../../types";

type Phase = "loading" | "judging" | "reveal" | "error";

interface Props {
  runId: number;
  onClose: () => void;
}

export function BlindCompare({ runId, onClose }: Props) {
  const [phase, setPhase] = useState<Phase>("loading");
  const [comparison, setComparison] = useState<BlindComparison | null>(null);
  const [pairIndex, setPairIndex] = useState(0);
  const [reveal, setReveal] = useState<BlindReveal | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  useEffect(() => {
    const load = async () => {
      try {
        const data = await invoke<BlindComparison>("start_blind_comparison", { runId });
        setComparison(data);
        setPhase("judging");
      } catch (err) {
        console.error("start_blind_comparison error:", err);
        setErrorMsg(String(err));
        setPhase("error");
      }
    };
    void load();
  }, [runId]);

  const handlePick = async (winner: "left" | "right" | "tie") => {
    if (!comparison) return;
    const pair = comparison.pairs[pairIndex];
    if (!pair) return;

    try {
      await invoke("submit_blind_pick", {
        runId,
        promptId: pair.prompt_id,
        winner,
      });

      const isLast = pairIndex >= comparison.pairs.length - 1;
      if (isLast) {
        const result = await invoke<BlindReveal>("finish_blind_comparison", { runId });
        setReveal(result);
        setPhase("reveal");
      } else {
        setPairIndex((i) => i + 1);
      }
    } catch (err) {
      console.error("submit_blind_pick error:", err);
      setErrorMsg(String(err));
      setPhase("error");
    }
  };

  const currentPair = comparison?.pairs[pairIndex];
  const total = comparison?.pairs.length ?? 0;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/80 backdrop-blur-sm">
      {/* Loading */}
      {phase === "loading" && (
        <div className="flex flex-col items-center gap-3">
          <div className="h-8 w-8 animate-spin rounded-full border-2 border-slate-700 border-t-gold-500" />
          <p className="text-sm text-slate-400">Preparing blind comparison...</p>
        </div>
      )}

      {/* Error */}
      {phase === "error" && (
        <div className="w-full max-w-md rounded-xl border border-red-800 bg-slate-900 p-6 shadow-2xl">
          <h2 className="mb-2 text-base font-bold text-red-400">Blind Comparison Failed</h2>
          <p className="mb-4 text-sm text-slate-400">{errorMsg ?? "An unknown error occurred."}</p>
          <button
            onClick={onClose}
            className="h-9 w-full rounded-lg bg-slate-800 text-sm font-medium text-slate-300 transition-colors hover:bg-slate-700 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
          >
            Close
          </button>
        </div>
      )}

      {/* Judging */}
      {phase === "judging" && currentPair && (
        <div className="flex h-full w-full flex-col bg-slate-950 p-6">
          {/* Header */}
          <div className="mb-4 flex items-center justify-between">
            <div>
              <h2 className="text-base font-bold text-slate-100">
                Blind Comparison — Prompt {pairIndex + 1} of {total}
              </h2>
              <p className="mt-0.5 text-xs text-slate-500">
                {currentPair.prompt_title}
                <span className="ml-2 rounded bg-slate-800 px-1.5 py-0.5 text-slate-500">
                  {currentPair.prompt_category}
                </span>
              </p>
            </div>
            {/* Progress dots */}
            <div className="flex items-center gap-1">
              {comparison?.pairs.map((_, i) => (
                <div
                  key={i}
                  className={`h-2 w-2 rounded-full transition-colors ${
                    i < pairIndex
                      ? "bg-gold-500"
                      : i === pairIndex
                        ? "bg-gold-400"
                        : "bg-slate-700"
                  }`}
                />
              ))}
            </div>
          </div>

          {/* Side-by-side panels */}
          <div className="mb-5 grid min-h-0 flex-1 grid-cols-2 gap-4">
            <div className="flex flex-col rounded-xl border border-slate-700 bg-slate-900">
              <div className="border-b border-slate-700 px-4 py-2">
                <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">
                  Response A
                </span>
              </div>
              <div className="max-h-96 flex-1 overflow-auto p-4 font-mono text-xs leading-relaxed text-slate-300">
                {currentPair.left_output || (
                  <span className="text-slate-600">No output</span>
                )}
              </div>
            </div>
            <div className="flex flex-col rounded-xl border border-slate-700 bg-slate-900">
              <div className="border-b border-slate-700 px-4 py-2">
                <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">
                  Response B
                </span>
              </div>
              <div className="max-h-96 flex-1 overflow-auto p-4 font-mono text-xs leading-relaxed text-slate-300">
                {currentPair.right_output || (
                  <span className="text-slate-600">No output</span>
                )}
              </div>
            </div>
          </div>

          {/* Pick buttons */}
          <div className="flex items-center justify-center gap-3">
            <button
              onClick={() => void handlePick("left")}
              className="h-10 rounded-lg bg-emerald-600 px-6 text-sm font-bold text-white transition-colors hover:bg-emerald-500 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
            >
              A is Better
            </button>
            <button
              onClick={() => void handlePick("tie")}
              className="h-10 rounded-lg bg-slate-700 px-6 text-sm font-bold text-slate-300 transition-colors hover:bg-slate-600 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
            >
              Tie
            </button>
            <button
              onClick={() => void handlePick("right")}
              className="h-10 rounded-lg bg-red-700 px-6 text-sm font-bold text-white transition-colors hover:bg-red-600 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
            >
              B is Better
            </button>
          </div>
        </div>
      )}

      {/* Reveal */}
      {phase === "reveal" && reveal && (
        <div className="w-full max-w-2xl rounded-xl border border-slate-700 bg-slate-900 p-6 shadow-2xl">
          {/* Summary banner */}
          <div className="mb-5 rounded-lg border border-gold-500/30 bg-gold-500/10 px-4 py-3">
            <p className="text-center text-sm font-semibold text-gold-400">
              {reveal.model_a_name}: {reveal.model_a_wins} wins
              <span className="mx-3 text-slate-600">|</span>
              {reveal.model_b_name}: {reveal.model_b_wins} wins
              <span className="mx-3 text-slate-600">|</span>
              {reveal.ties} {reveal.ties === 1 ? "tie" : "ties"}
            </p>
          </div>

          {/* Per-prompt table */}
          <div className="mb-5 max-h-72 overflow-auto rounded-lg border border-slate-700">
            <table className="w-full text-sm">
              <thead className="sticky top-0 bg-slate-800/95">
                <tr className="border-b border-slate-700">
                  <th className="px-3 py-2 text-left text-xs font-medium text-slate-400">
                    Prompt
                  </th>
                  <th className="px-3 py-2 text-center text-xs font-medium text-slate-400">
                    Winner
                  </th>
                </tr>
              </thead>
              <tbody>
                {reveal.entries.map((entry) => (
                  <tr
                    key={entry.prompt_id}
                    className="border-b border-slate-800/50 hover:bg-slate-800/30"
                  >
                    <td className="px-3 py-2 text-slate-300">{entry.prompt_title}</td>
                    <td className="px-3 py-2 text-center">
                      {entry.winner === "tie" ? (
                        <span className="text-slate-400">Tie</span>
                      ) : entry.winner === "model_a" ? (
                        <span className="font-medium text-emerald-400">
                          {entry.model_a_name}
                        </span>
                      ) : (
                        <span className="font-medium text-red-400">
                          {entry.model_b_name}
                        </span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <button
            onClick={onClose}
            className="h-10 w-full rounded-lg bg-gold-500 text-sm font-bold text-slate-950 transition-colors hover:bg-gold-400 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-gold-400"
          >
            Done
          </button>
        </div>
      )}
    </div>
  );
}
