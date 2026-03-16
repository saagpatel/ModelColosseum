import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ModelSelector } from "./ModelSelector";
import type { Model, DebateFormat } from "../types";

interface DebateSetupProps {
  models: Model[];
  onStart: (topic: string, modelAId: number, modelBId: number, rounds: number, format: DebateFormat) => void;
  loading: boolean;
}

const ROUND_OPTIONS = [3, 5, 7] as const;

const FORMATS: { value: DebateFormat; label: string; description: string }[] = [
  { value: "freestyle", label: "Freestyle", description: "Open-ended debate with flexible rounds" },
  { value: "formal", label: "Formal", description: "3 phases: Opening, Rebuttal, Closing" },
  { value: "socratic", label: "Socratic", description: "Question/defense alternation with role swap" },
];

export function DebateSetup({ models, onStart, loading }: DebateSetupProps) {
  const [topic, setTopic] = useState("");
  const [modelAId, setModelAId] = useState<number | null>(null);
  const [modelBId, setModelBId] = useState<number | null>(null);
  const [rounds, setRounds] = useState(5);
  const [format, setFormat] = useState<DebateFormat>("freestyle");
  const [suggestedTopics, setSuggestedTopics] = useState<string[]>([]);
  const [loadingTopics, setLoadingTopics] = useState(false);

  const sameModel = modelAId !== null && modelBId !== null && modelAId === modelBId;
  const effectiveRounds = format === "formal" ? 3 : rounds;
  const canStart = topic.trim() && modelAId !== null && modelBId !== null && !sameModel && !loading;

  const handleStart = () => {
    if (modelAId !== null && modelBId !== null && topic.trim()) {
      onStart(topic.trim(), modelAId, modelBId, effectiveRounds, format);
    }
  };

  const handleSuggestTopics = async () => {
    if (models.length === 0) return;
    setLoadingTopics(true);
    try {
      const sorted = [...models].sort((a, b) => (a.parameter_count ?? Infinity) - (b.parameter_count ?? Infinity));
      const smallest = sorted[0];
      if (!smallest) return;
      const topics = await invoke<string[]>("suggest_topics", { modelName: smallest.name });
      setSuggestedTopics(topics);
    } catch (err) {
      console.error("suggest_topics error:", err);
    } finally {
      setLoadingTopics(false);
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

      {/* Debate Format */}
      <div>
        <label className="mb-2 block text-sm font-medium text-slate-400">Debate Format</label>
        <div className="grid grid-cols-3 gap-3">
          {FORMATS.map((f) => (
            <button
              key={f.value}
              onClick={() => setFormat(f.value)}
              className={`rounded-lg border px-4 py-3 text-left transition-all ${
                format === f.value
                  ? "border-gold-500/50 bg-gold-500/10"
                  : "border-slate-700 bg-slate-800/50 hover:border-slate-600"
              }`}
            >
              <span
                className={`block text-sm font-semibold ${
                  format === f.value ? "text-gold-400" : "text-slate-300"
                }`}
              >
                {f.label}
              </span>
              <span className="mt-1 block text-xs text-slate-500">{f.description}</span>
            </button>
          ))}
        </div>
      </div>

      {/* Topic */}
      <div>
        <div className="mb-2 flex items-center justify-between">
          <label className="text-sm font-medium text-slate-400">Debate Topic</label>
          <button
            onClick={() => void handleSuggestTopics()}
            disabled={loadingTopics || models.length === 0}
            className="rounded-md bg-slate-800 px-3 py-1 text-xs font-medium text-slate-400 transition-colors hover:bg-slate-700 hover:text-slate-200 disabled:cursor-not-allowed disabled:opacity-40"
          >
            {loadingTopics ? "Generating..." : "Suggest Topics"}
          </button>
        </div>
        <input
          type="text"
          value={topic}
          onChange={(e) => setTopic(e.target.value)}
          placeholder="e.g. Should AI systems be open source?"
          className="w-full rounded-lg border border-slate-700 bg-slate-800/50 px-4 py-3 text-slate-200 placeholder-slate-600 outline-none transition-colors focus:border-gold-500/50 focus:ring-1 focus:ring-gold-500/30"
        />
        {suggestedTopics.length > 0 && (
          <div className="mt-3 flex flex-wrap gap-2">
            {suggestedTopics.map((t) => (
              <button
                key={t}
                onClick={() => setTopic(t)}
                className="rounded-full border border-slate-700 bg-slate-800/50 px-3 py-1.5 text-xs text-slate-300 transition-colors hover:border-gold-500/50 hover:text-gold-400"
              >
                {t}
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Model Pickers */}
      <div className="grid grid-cols-2 gap-6">
        <div>
          <ModelSelector models={models} selectedId={modelAId} onSelect={setModelAId} label="Model A (Pro)" />
        </div>
        <div>
          <ModelSelector models={models} selectedId={modelBId} onSelect={setModelBId} label="Model B (Con)" />
        </div>
      </div>

      {sameModel && (
        <p className="text-center text-sm text-red-400">Select two different models for the debate</p>
      )}

      {/* Round Toggle */}
      {format === "formal" ? (
        <div className="flex items-center justify-center">
          <span className="rounded-lg border border-slate-700 bg-slate-800/50 px-4 py-2 text-sm text-slate-400">
            Formal debates use 3 rounds (Opening → Rebuttal → Closing)
          </span>
        </div>
      ) : (
        <div className="flex items-center justify-center gap-3">
          <span className="text-sm text-slate-400">Rounds:</span>
          <div className="flex gap-1 rounded-lg border border-slate-700 bg-slate-800/50 p-1">
            {ROUND_OPTIONS.map((n) => (
              <button
                key={n}
                onClick={() => setRounds(n)}
                className={`rounded-md px-4 py-1.5 text-sm font-medium transition-colors ${
                  rounds === n ? "bg-gold-500/20 text-gold-400" : "text-slate-400 hover:text-slate-200"
                }`}
              >
                {n}
              </button>
            ))}
          </div>
        </div>
      )}

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
