import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAppStore } from "../../stores/appStore";
import { useBenchmarkStore } from "../../stores/benchmarkStore";
import { useAutoJudgeEvents } from "../../hooks/useAutoJudgeEvents";

interface AutoJudgePanelProps {
  runId: number;
  onComplete: () => void;
}

export function AutoJudgePanel({ runId, onComplete }: AutoJudgePanelProps) {
  const { models } = useAppStore();
  const { autoJudgeProgress, setAutoJudgeProgress } = useBenchmarkStore();
  const [judgeModelId, setJudgeModelId] = useState<number | null>(
    models[0]?.id ?? null
  );
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleComplete = () => {
    setRunning(false);
    onComplete();
  };

  useAutoJudgeEvents(runId, handleComplete);

  const handleStart = async () => {
    if (judgeModelId === null) return;
    setRunning(true);
    setError(null);
    setAutoJudgeProgress({ completed: 0, total: 0, currentModel: "" });
    try {
      await invoke("auto_judge_benchmark", { runId, judgeModelId });
    } catch (err) {
      console.error("auto_judge_benchmark error:", err);
      setError(String(err));
      setRunning(false);
      setAutoJudgeProgress(null);
    }
  };

  const handleCancel = async () => {
    try {
      await invoke("cancel_auto_judge", { runId });
    } catch (err) {
      console.error("cancel_auto_judge error:", err);
    }
    setRunning(false);
    setAutoJudgeProgress(null);
  };

  const pct =
    autoJudgeProgress && autoJudgeProgress.total > 0
      ? Math.round((autoJudgeProgress.completed / autoJudgeProgress.total) * 100)
      : 0;

  return (
    <div className="rounded-xl border border-slate-700 bg-slate-900 p-5">
      <h3 className="mb-4 text-sm font-semibold text-slate-200">Auto-Judge</h3>

      {error && (
        <p className="mb-3 rounded-md bg-red-900/30 px-3 py-2 text-xs text-red-400">
          {error}
        </p>
      )}

      {!running ? (
        <div className="flex items-center gap-3">
          <select
            value={judgeModelId ?? ""}
            onChange={(e) => setJudgeModelId(Number(e.target.value))}
            className="flex-1 rounded-md border border-slate-600 bg-slate-800 px-3 py-2 text-sm text-slate-100 transition-colors focus:border-gold-500 focus:outline-none"
          >
            {models.length === 0 && (
              <option value="">No models available</option>
            )}
            {models.map((m) => (
              <option key={m.id} value={m.id}>
                {m.display_name}
              </option>
            ))}
          </select>
          <button
            onClick={() => void handleStart()}
            disabled={judgeModelId === null || models.length === 0}
            className="h-10 rounded-lg bg-gold-500 px-4 text-sm font-bold text-slate-950 transition-colors hover:bg-gold-400 disabled:cursor-not-allowed disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-gold-400"
          >
            Run Auto-Judge
          </button>
        </div>
      ) : (
        <div className="space-y-3">
          {/* Progress bar */}
          <div className="h-2 w-full overflow-hidden rounded-full bg-slate-800">
            <div
              className="h-full rounded-full bg-gold-500 transition-all duration-300"
              style={{ width: `${pct}%` }}
            />
          </div>
          <div className="flex items-center justify-between text-xs text-slate-500">
            <span>
              {autoJudgeProgress
                ? `${autoJudgeProgress.completed} / ${autoJudgeProgress.total}`
                : "Starting..."}
            </span>
            <span>{pct}%</span>
          </div>
          {autoJudgeProgress?.currentModel && (
            <p className="text-xs text-slate-400">
              <span className="text-slate-500">Judging: </span>
              <span className="text-slate-200">{autoJudgeProgress.currentModel}</span>
            </p>
          )}
          <button
            onClick={() => void handleCancel()}
            className="h-8 rounded-lg bg-slate-800 px-3 text-xs font-medium text-slate-400 transition-colors hover:bg-slate-700 hover:text-slate-200 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
          >
            Cancel
          </button>
        </div>
      )}
    </div>
  );
}
