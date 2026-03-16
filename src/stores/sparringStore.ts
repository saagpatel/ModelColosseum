import { create } from "zustand";

type SparringPhase = "idle" | "human_turn" | "ai_turn" | "complete" | "error" | "aborted";
type SparringStage = "opening" | "rebuttal" | "closing";
type Difficulty = "casual" | "competitive" | "expert";
type Side = "pro" | "con";

interface RoundEntry {
  round: number;
  speaker: "human" | "ai";
  phase: string;
  content: string;
}

interface SparringState {
  phase: SparringPhase;
  debateId: number | null;
  topic: string;
  humanSide: Side | null;
  modelId: number | null;
  difficulty: Difficulty | null;
  currentStage: SparringStage;
  currentRound: number;
  wordLimit: number;
  aiStreamContent: string;
  rounds: RoundEntry[];
  errorMessage: string | null;

  startSparring: (debateId: number, topic: string, humanSide: Side, modelId: number, difficulty: Difficulty, wordLimit: number) => void;
  submitHumanRound: (content: string) => void;
  appendAiToken: (token: string) => void;
  completeRound: (round: number, phase: string, aiContent: string, nextPhase: string | null, nextWordLimit: number | null, isComplete: boolean) => void;
  setComplete: () => void;
  setError: (message: string) => void;
  setAborted: () => void;
  reset: () => void;
}

function stageFromPhase(phase: string): SparringStage {
  if (phase === "opening") return "opening";
  if (phase === "closing") return "closing";
  return "rebuttal";
}

const initialState = {
  phase: "idle" as SparringPhase,
  debateId: null,
  topic: "",
  humanSide: null,
  modelId: null,
  difficulty: null,
  currentStage: "opening" as SparringStage,
  currentRound: 1,
  wordLimit: 200,
  aiStreamContent: "",
  rounds: [] as RoundEntry[],
  errorMessage: null,
};

export const useSparringStore = create<SparringState>((set) => ({
  ...initialState,

  startSparring: (debateId, topic, humanSide, modelId, difficulty, wordLimit) =>
    set({
      phase: "human_turn",
      debateId,
      topic,
      humanSide,
      modelId,
      difficulty,
      currentStage: "opening",
      currentRound: 1,
      wordLimit,
      aiStreamContent: "",
      rounds: [],
      errorMessage: null,
    }),

  submitHumanRound: (content) =>
    set((state) => ({
      phase: "ai_turn",
      aiStreamContent: "",
      rounds: [
        ...state.rounds,
        {
          round: state.currentRound,
          speaker: "human" as const,
          phase: state.currentStage,
          content,
        },
      ],
    })),

  appendAiToken: (token) =>
    set((state) => ({
      aiStreamContent: state.aiStreamContent + token,
    })),

  completeRound: (round, phase, aiContent, nextPhase, nextWordLimit, isComplete) =>
    set((state) => ({
      phase: isComplete ? "complete" : "human_turn",
      currentRound: isComplete ? round : round + 1,
      currentStage: nextPhase ? stageFromPhase(nextPhase) : state.currentStage,
      wordLimit: nextWordLimit ?? state.wordLimit,
      aiStreamContent: "",
      rounds: [
        ...state.rounds,
        {
          round,
          speaker: "ai" as const,
          phase,
          content: aiContent,
        },
      ],
    })),

  setComplete: () => set({ phase: "complete" }),
  setError: (message) => set({ phase: "error", errorMessage: message }),
  setAborted: () => set({ phase: "aborted" }),
  reset: () => set(initialState),
}));
