import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { useBenchmarkStore } from "../stores/benchmarkStore";
import type { AutoJudgeProgressPayload, AutoJudgeCompletePayload } from "../types";

export function useAutoJudgeEvents(runId: number | null, onComplete?: () => void) {
  useEffect(() => {
    if (runId === null) return;
    const unlisteners: Promise<UnlistenFn>[] = [];

    unlisteners.push(
      listen<AutoJudgeProgressPayload>("autojudge:progress", (event) => {
        if (event.payload.run_id !== runId) return;
        useBenchmarkStore.getState().setAutoJudgeProgress({
          completed: event.payload.completed,
          total: event.payload.total,
          currentModel: event.payload.current_model,
        });
      })
    );

    unlisteners.push(
      listen<AutoJudgeCompletePayload>("autojudge:complete", (event) => {
        if (event.payload.run_id !== runId) return;
        useBenchmarkStore.getState().setAutoJudgeProgress(null);
        onComplete?.();
      })
    );

    return () => {
      for (const p of unlisteners) {
        void p.then((unlisten) => unlisten());
      }
    };
  }, [runId, onComplete]);
}
