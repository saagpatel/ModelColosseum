import { useState } from "react";
import { ModelSelector } from "./ModelSelector";
import type { Model } from "../types";

interface DebateSetupProps {
  models: Model[];
  onStart: (topic: string, modelAId: number, modelBId: number, rounds: number) => void;
  loading: boolean;
}

const ROUND_OPTIONS = [3, 5, 7] as const;

export function DebateSetup({ models, onStart, loading }: DebateSetupProps) {
  const [topic, setTopic] = useState("");
  const [modelAId, setModelAId] = useState<number | null>(null);
  const [modelBId, setModelBId] = useState<number | null>(null);
  const [rounds, setRounds] = useState(5);

  const sameModel = modelAId !== null && modelBId !== null && modelAId === modelBId;
  const canStart = topic.trim() && modelAId !== null && modelBId !== null && !sameModel && !loading;

  const handleStart = () => {
    if (modelAId !== null && modelBId !== null && topic.trim()) {
      onStart(topic.trim(), modelAId, modelBId, rounds);
    }
  };

  return (
    <div className="mx-auto max-w-3xl space-y-8">
      <div className="text-center">
        <h2 className="text-3xl font-black tracking-tight text-gold-400">Arena Battle</h2>
        <p className="mt-2 text-sm font-light text-slate-400">
          Pit two models against each other in structured debate
        </p>
      </div>

      {/* Topic */}
      <div>
        <label className="mb-2 block text-sm font-medium text-slate-400">Debate Topic</label>
        <input
          type="text"
          value={topic}
          onChange={(e) => setTopic(e.target.value)}
          placeholder="e.g. Should AI systems be open source?"
          className="w-full rounded-lg border border-slate-700 bg-slate-800/50 px-4 py-3 text-slate-200 placeholder-slate-600 outline-none transition-colors focus:border-gold-500/50 focus:ring-1 focus:ring-gold-500/30"
        />
      </div>

      {/* Model Pickers */}
      <div className="grid grid-cols-2 gap-6">
        <div>
          <ModelSelector
            models={models}
            selectedId={modelAId}
            onSelect={setModelAId}
            label="Model A (Pro)"
          />
        </div>
        <div>
          <ModelSelector
            models={models}
            selectedId={modelBId}
            onSelect={setModelBId}
            label="Model B (Con)"
          />
        </div>
      </div>

      {sameModel && (
        <p className="text-center text-sm text-red-400">
          Select two different models for the debate
        </p>
      )}

      {/* Round Toggle */}
      <div className="flex items-center justify-center gap-3">
        <span className="text-sm text-slate-400">Rounds:</span>
        <div className="flex gap-1 rounded-lg border border-slate-700 bg-slate-800/50 p-1">
          {ROUND_OPTIONS.map((n) => (
            <button
              key={n}
              onClick={() => setRounds(n)}
              className={`rounded-md px-4 py-1.5 text-sm font-medium transition-colors ${
                rounds === n
                  ? "bg-gold-500/20 text-gold-400"
                  : "text-slate-400 hover:text-slate-200"
              }`}
            >
              {n}
            </button>
          ))}
        </div>
      </div>

      {/* Start Button */}
      <div className="text-center">
        <button
          onClick={handleStart}
          disabled={!canStart}
          className="rounded-lg bg-gold-500 px-8 py-3 text-base font-bold text-slate-950 transition-all hover:bg-gold-400 disabled:cursor-not-allowed disabled:opacity-40"
        >
          {loading ? "Starting..." : "Start Debate"}
        </button>
      </div>
    </div>
  );
}
