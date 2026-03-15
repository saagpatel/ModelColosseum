import { create } from "zustand";

type DebatePhase = "idle" | "debating" | "complete" | "error" | "aborted";

interface DebateState {
  phase: DebatePhase;
  debateId: number | null;
  topic: string;
  modelAId: number | null;
  modelBId: number | null;
  totalRounds: number;
  currentRound: number;
  mode: "concurrent" | "sequential" | null;
  errorMessage: string | null;

  startDebate: (
    debateId: number,
    topic: string,
    modelAId: number,
    modelBId: number,
    totalRounds: number,
  ) => void;
  setMode: (mode: "concurrent" | "sequential") => void;
  advanceRound: (round: number) => void;
  complete: () => void;
  abort: () => void;
  setError: (message: string) => void;
  reset: () => void;
}

const initialState = {
  phase: "idle" as DebatePhase,
  debateId: null,
  topic: "",
  modelAId: null,
  modelBId: null,
  totalRounds: 5,
  currentRound: 1,
  mode: null,
  errorMessage: null,
};

export const useDebateStore = create<DebateState>((set) => ({
  ...initialState,

  startDebate: (debateId, topic, modelAId, modelBId, totalRounds) =>
    set({
      phase: "debating",
      debateId,
      topic,
      modelAId,
      modelBId,
      totalRounds,
      currentRound: 1,
      mode: null,
      errorMessage: null,
    }),

  setMode: (mode) => set({ mode }),

  advanceRound: (round) => set({ currentRound: round }),

  complete: () => set({ phase: "complete" }),

  abort: () => set({ phase: "aborted" }),

  setError: (message) => set({ phase: "error", errorMessage: message }),

  reset: () => set(initialState),
}));
