import { create } from "zustand";
import type { TestSuite, Prompt, BenchmarkProgress, BenchmarkResult } from "../types";

type BenchmarkPhase = "editing" | "configuring" | "running" | "complete" | "results" | "error";

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

  // Results phase state
  results: BenchmarkResult[];
  viewingRunId: number | null;
  blindMode: boolean;
  scoreAllMode: boolean;
  scoreAllPromptIndex: number;
  autoJudgeProgress: { completed: number; total: number; currentModel: string } | null;

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

  // Results actions
  setResults: (results: BenchmarkResult[]) => void;
  viewRun: (runId: number, results: BenchmarkResult[]) => void;
  toggleBlindMode: () => void;
  enterScoreAllMode: () => void;
  exitScoreAllMode: () => void;
  nextPrompt: () => void;
  prevPrompt: () => void;
  setAutoJudgeProgress: (progress: { completed: number; total: number; currentModel: string } | null) => void;
  updateResultScore: (resultId: number, score: number) => void;
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
  results: [] as BenchmarkResult[],
  viewingRunId: null,
  blindMode: false,
  scoreAllMode: false,
  scoreAllPromptIndex: 0,
  autoJudgeProgress: null,
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

  setResults: (results) => set({ results }),

  viewRun: (runId, results) =>
    set({
      phase: "results",
      viewingRunId: runId,
      results,
      scoreAllMode: false,
      scoreAllPromptIndex: 0,
      autoJudgeProgress: null,
    }),

  toggleBlindMode: () => set((s) => ({ blindMode: !s.blindMode })),

  enterScoreAllMode: () => set({ scoreAllMode: true, scoreAllPromptIndex: 0 }),

  exitScoreAllMode: () => set({ scoreAllMode: false }),

  nextPrompt: () => {
    const { scoreAllPromptIndex, results } = get();
    const promptIds = [...new Set(results.map((r) => r.prompt_id))];
    if (scoreAllPromptIndex < promptIds.length - 1) {
      set({ scoreAllPromptIndex: scoreAllPromptIndex + 1 });
    }
  },

  prevPrompt: () => {
    const { scoreAllPromptIndex } = get();
    if (scoreAllPromptIndex > 0) {
      set({ scoreAllPromptIndex: scoreAllPromptIndex - 1 });
    }
  },

  setAutoJudgeProgress: (progress) => set({ autoJudgeProgress: progress }),

  updateResultScore: (resultId, score) =>
    set((s) => ({
      results: s.results.map((r) =>
        r.id === resultId ? { ...r, manual_score: score } : r
      ),
    })),
}));
