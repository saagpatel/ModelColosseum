import { invoke } from "@tauri-apps/api/core";
import { DebateSetup } from "../components/DebateSetup";
import { DebateViewer } from "../components/DebateViewer";
import { useAppStore } from "../stores/appStore";
import { useDebateStore } from "../stores/debateStore";

export function Arena() {
  const { ollamaOnline, models, loading } = useAppStore();
  const { phase, reset, startDebate, topic } = useDebateStore();

  const handleStart = async (
    newTopic: string,
    modelAId: number,
    modelBId: number,
    rounds: number,
  ) => {
    try {
      const debateId = await invoke<number>("start_debate", {
        topic: newTopic,
        modelAId,
        modelBId,
        rounds,
      });
      startDebate(debateId, newTopic, modelAId, modelBId, rounds);
    } catch (err) {
      console.error("start_debate error:", err);
      useDebateStore.getState().setError(String(err));
    }
  };

  if (!ollamaOnline) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-4">
        <svg
          className="h-16 w-16 text-slate-600"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          strokeWidth={1.5}
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z"
          />
        </svg>
        <h2 className="text-2xl font-bold text-slate-400">Arena Offline</h2>
        <p className="text-sm text-slate-500">
          Start Ollama to begin battling models
        </p>
      </div>
    );
  }

  const isActive = phase !== "idle";
  const showNewDebate = phase === "complete" || phase === "error" || phase === "aborted";

  return (
    <div className="flex h-full flex-col p-6">
      {isActive && (
        <div className="mb-4 flex items-center justify-between">
          <h3 className="truncate text-sm font-medium text-slate-400">
            {topic}
          </h3>
          {showNewDebate && (
            <button
              onClick={reset}
              className="rounded-lg bg-slate-800 px-4 py-2 text-sm font-medium text-gold-400 transition-colors hover:bg-slate-700"
            >
              New Debate
            </button>
          )}
        </div>
      )}

      {phase === "idle" ? (
        <DebateSetup models={models} onStart={(t, a, b, r) => void handleStart(t, a, b, r)} loading={loading} />
      ) : (
        <DebateViewer />
      )}
    </div>
  );
}
