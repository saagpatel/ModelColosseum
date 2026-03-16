import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { useBenchmarkStore } from "../stores/benchmarkStore";
import type {
  BenchmarkProgress,
  BenchmarkStreamPayload,
  BenchmarkCompletePayload,
  BenchmarkErrorPayload,
  BenchmarkMetricsPayload,
} from "../types";

export function useBenchmarkEvents(runId: number | null) {
  useEffect(() => {
    if (runId === null) return;

    const unlisteners: Promise<UnlistenFn>[] = [];

    unlisteners.push(
      listen<BenchmarkProgress>("benchmark:progress", (event) => {
        if (event.payload.run_id !== runId) return;
        useBenchmarkStore.getState().updateProgress(event.payload);
      }),
    );

    unlisteners.push(
      listen<BenchmarkStreamPayload>("benchmark:stream", (event) => {
        if (event.payload.run_id !== runId) return;
        useBenchmarkStore.getState().appendStream(event.payload.token);
      }),
    );

    unlisteners.push(
      listen<BenchmarkCompletePayload>("benchmark:complete", (event) => {
        if (event.payload.run_id !== runId) return;
        useBenchmarkStore.getState().complete();
      }),
    );

    unlisteners.push(
      listen<BenchmarkErrorPayload>("benchmark:error", (event) => {
        if (event.payload.run_id !== runId) return;
        useBenchmarkStore.getState().setError(event.payload.message);
      }),
    );

    unlisteners.push(
      listen<BenchmarkMetricsPayload>("benchmark:metrics", (event) => {
        if (event.payload.run_id !== runId) return;
        useBenchmarkStore.getState().appendMetric(event.payload);
      }),
    );

    return () => {
      for (const p of unlisteners) {
        void p.then((unlisten) => unlisten());
      }
    };
  }, [runId]);
}
