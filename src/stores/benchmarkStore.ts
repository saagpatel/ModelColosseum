import { create } from "zustand";
import type { TestSuite, Prompt, BenchmarkProgress } from "../types";

type BenchmarkPhase = "editing" | "configuring" | "running" | "complete" | "error";

interface BenchmarkState {
  phase: BenchmarkPhase;
  suites: TestSuite[];
  selectedSuiteId: number | null;
  prompts: Prompt[];
  runId: number | null;
  progress: {
    completed: number;
    total: number;
    currentModel: string;
    currentPrompt: string;
  } | null;
  streamPreview: string;
  startedAt: number | null;
  errorMessage: string | null;

  setSuites: (suites: TestSuite[]) => void;
  selectSuite: (id: number | null) => void;
  setPrompts: (prompts: Prompt[]) => void;
  startConfiguring: () => void;
  startRun: (runId: number) => void;
  updateProgress: (progress: BenchmarkProgress) => void;
  appendStream: (token: string) => void;
  complete: () => void;
  setError: (message: string) => void;
  reset: () => void;
}

const initialState = {
  phase: "editing" as BenchmarkPhase,
  suites: [] as TestSuite[],
  selectedSuiteId: null,
  prompts: [] as Prompt[],
  runId: null,
  progress: null,
  streamPreview: "",
  startedAt: null,
  errorMessage: null,
};

export const useBenchmarkStore = create<BenchmarkState>((set, get) => ({
  ...initialState,

  setSuites: (suites) => set({ suites }),

  selectSuite: (id) => set({ selectedSuiteId: id, prompts: [] }),

  setPrompts: (prompts) => set({ prompts }),

  startConfiguring: () => set({ phase: "configuring" }),

  startRun: (runId) =>
    set({
      phase: "running",
      runId,
      startedAt: Date.now(),
      streamPreview: "",
    }),

  updateProgress: (progress) =>
    set({
      progress: {
        completed: progress.completed,
        total: progress.total,
        currentModel: progress.current_model,
        currentPrompt: progress.current_prompt,
      },
      streamPreview: "",
    }),

  appendStream: (token) => {
    const current = get().streamPreview + token;
    set({ streamPreview: current.length > 500 ? current.slice(-500) : current });
  },

  complete: () => set({ phase: "complete" }),

  setError: (message) => set({ phase: "error", errorMessage: message }),

  reset: () => set(initialState),
}));
