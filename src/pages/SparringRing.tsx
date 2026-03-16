import { useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ModelSelector } from "../components/ModelSelector";
import { useAppStore } from "../stores/appStore";
import { useSparringStore } from "../stores/sparringStore";
import { useSparringEvents } from "../hooks/useSparringEvents";

type Difficulty = "casual" | "competitive" | "expert";
type Side = "pro" | "con";

const DIFFICULTIES: { value: Difficulty; label: string; description: string }[] = [
  { value: "casual", label: "Casual", description: "Fair and balanced. Acknowledges good points." },
  { value: "competitive", label: "Competitive", description: "Aggressive. Exploits weaknesses. Never concedes." },
  { value: "expert", label: "Expert", description: "Domain expert. Uses data, examples, and rhetoric." },
];

const PHASES = ["Opening", "Rebuttal 1", "Rebuttal 2", "Closing"] as const;

function phaseIndex(stage: string, round: number): number {
  if (stage === "opening") return 0;
  if (stage === "closing") return 3;
  // rebuttal: rounds 3-4 = index 1, rounds 5-6 = index 2
  return round <= 4 ? 1 : 2;
}

function wordCount(text: string): number {
  return text.trim().split(/\s+/).filter(Boolean).length;
}

function SetupView() {
  const { models } = useAppStore();
  const { startSparring } = useSparringStore();

  const [topic, setTopic] = useState("");
  const [side, setSide] = useState<Side>("pro");
  const [modelId, setModelId] = useState<number | null>(null);
  const [difficulty, setDifficulty] = useState<Difficulty>("competitive");
  const [starting, setStarting] = useState(false);

  const canStart = topic.trim().length > 0 && modelId !== null;

  const handleStart = async () => {
    if (!canStart || modelId === null) return;
    setStarting(true);
    try {
      const debateId = await invoke<number>("start_sparring", {
        topic: topic.trim(),
        modelId,
        humanSide: side,
        difficulty,
      });
      startSparring(debateId, topic.trim(), side, modelId, difficulty, 200);
    } catch (err) {
      console.error("start_sparring error:", err);
      useSparringStore.getState().setError(String(err));
    } finally {
      setStarting(false);
    }
  };

  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-8">
      {/* Title */}
      <div className="text-center">
        <h2 className="text-3xl font-black tracking-tight text-slate-100">
          Sparring Ring
        </h2>
        <p className="mt-2 text-sm text-slate-500">
          Debate an AI opponent across 4 phases. You go first.
        </p>
      </div>

      {/* Topic */}
      <div>
        <label className="mb-2 block text-sm font-medium text-slate-400">
          Debate Topic
        </label>
        <input
          type="text"
          value={topic}
          onChange={(e) => setTopic(e.target.value)}
          placeholder="e.g. Artificial intelligence will create more jobs than it destroys"
          className="w-full rounded-lg border border-slate-700 bg-slate-800/50 px-4 py-3 text-sm text-slate-200 placeholder-slate-600 outline-none transition-colors focus:border-gold-500/50 focus:ring-1 focus:ring-gold-500/30"
        />
      </div>

      {/* Side Toggle */}
      <div>
        <label className="mb-2 block text-sm font-medium text-slate-400">
          Your Position
        </label>
        <div className="flex gap-3">
          <button
            onClick={() => setSide("pro")}
            className={`flex-1 rounded-lg border px-4 py-3 text-sm font-semibold transition-all ${
              side === "pro"
                ? "border-emerald-500/50 bg-emerald-500/10 text-emerald-400"
                : "border-slate-700 bg-slate-800/50 text-slate-400 hover:border-slate-600"
            }`}
          >
            FOR
          </button>
          <button
            onClick={() => setSide("con")}
            className={`flex-1 rounded-lg border px-4 py-3 text-sm font-semibold transition-all ${
              side === "con"
                ? "border-red-500/50 bg-red-500/10 text-red-400"
                : "border-slate-700 bg-slate-800/50 text-slate-400 hover:border-slate-600"
            }`}
          >
            AGAINST
          </button>
        </div>
      </div>

      {/* Model Selector */}
      <ModelSelector
        models={models}
        selectedId={modelId}
        onSelect={setModelId}
        label="AI Opponent"
      />

      {/* Difficulty */}
      <div>
        <label className="mb-2 block text-sm font-medium text-slate-400">
          Difficulty
        </label>
        <div className="grid grid-cols-3 gap-3">
          {DIFFICULTIES.map((d) => (
            <button
              key={d.value}
              onClick={() => setDifficulty(d.value)}
              className={`rounded-lg border px-4 py-3 text-left transition-all ${
                difficulty === d.value
                  ? "border-gold-500/50 bg-gold-500/10"
                  : "border-slate-700 bg-slate-800/50 hover:border-slate-600"
              }`}
            >
              <span
                className={`block text-sm font-semibold ${
                  difficulty === d.value ? "text-gold-400" : "text-slate-300"
                }`}
              >
                {d.label}
              </span>
              <span className="mt-1 block text-xs text-slate-500">
                {d.description}
              </span>
            </button>
          ))}
        </div>
      </div>

      {/* Start Button */}
      <button
        onClick={() => void handleStart()}
        disabled={!canStart || starting}
        className="rounded-lg bg-gold-500 px-6 py-3 text-sm font-bold text-slate-950 transition-all hover:bg-gold-400 disabled:cursor-not-allowed disabled:opacity-40"
      >
        {starting ? "Starting..." : "Begin Debate"}
      </button>
    </div>
  );
}

function PhaseProgress({ currentStage, currentRound, isComplete }: { currentStage: string; currentRound: number; isComplete: boolean }) {
  const activeIndex = isComplete ? 4 : phaseIndex(currentStage, currentRound);

  return (
    <div className="flex items-center justify-center gap-8">
      {PHASES.map((label, i) => {
        const done = i < activeIndex;
        const active = i === activeIndex;
        return (
          <div key={label} className="flex items-center gap-2">
            <div
              className={`h-3 w-3 rounded-full transition-colors ${
                done
                  ? "bg-gold-500"
                  : active
                    ? "bg-gold-400 ring-2 ring-gold-400/30"
                    : "bg-slate-700"
              }`}
            />
            <span
              className={`text-xs font-medium ${
                done || active ? "text-gold-400" : "text-slate-600"
              }`}
            >
              {label}
            </span>
          </div>
        );
      })}
    </div>
  );
}

function DebateView() {
  const aiPanelRef = useRef<HTMLDivElement>(null);
  const [humanInput, setHumanInput] = useState("");

  const {
    phase,
    debateId,
    humanSide,
    modelId,
    difficulty,
    currentStage,
    currentRound,
    wordLimit,
    rounds,
    errorMessage,
    reset,
  } = useSparringStore();

  const models = useAppStore((s) => s.models);
  const model = models.find((m) => m.id === modelId);

  useSparringEvents(debateId, aiPanelRef);

  const words = wordCount(humanInput);
  const overLimit = words > wordLimit;
  const nearLimit = words >= wordLimit * 0.8;
  const isHumanTurn = phase === "human_turn";
  const isAiTurn = phase === "ai_turn";
  const isTerminal = phase === "complete" || phase === "error" || phase === "aborted";
  const canSubmit = isHumanTurn && humanInput.trim().length > 0 && !overLimit;

  const humanRounds = rounds.filter((r) => r.speaker === "human");
  const aiRounds = rounds.filter((r) => r.speaker === "ai");

  const handleSubmit = async () => {
    if (!canSubmit || debateId === null) return;
    const content = humanInput.trim();
    setHumanInput("");
    useSparringStore.getState().submitHumanRound(content);
    try {
      await invoke("submit_human_argument", { debateId, content });
    } catch (err) {
      console.error("submit_human_argument error:", err);
      useSparringStore.getState().setError(String(err));
    }
  };

  const handleAbort = () => {
    if (debateId !== null) {
      void invoke("abort_sparring", { debateId });
    }
  };

  const aiSide = humanSide === "pro" ? "CON" : "PRO";
  const humanSideLabel = humanSide === "pro" ? "PRO" : "CON";

  return (
    <div className="flex h-full flex-col gap-4">
      {/* Phase Progress */}
      <PhaseProgress currentStage={currentStage} currentRound={currentRound} isComplete={isTerminal} />

      {/* Turn Indicator */}
      <div className="flex justify-center">
        {isHumanTurn && (
          <span className="rounded-full bg-gold-500/10 px-4 py-1 text-xs font-semibold text-gold-400">
            Your Turn — {currentStage.charAt(0).toUpperCase() + currentStage.slice(1)}
          </span>
        )}
        {isAiTurn && (
          <span className="animate-pulse rounded-full bg-slate-800 px-4 py-1 text-xs font-semibold text-slate-400">
            AI is responding...
          </span>
        )}
        {phase === "complete" && (
          <span className="rounded-full bg-emerald-500/10 px-4 py-1 text-xs font-semibold text-emerald-400">
            Debate Complete
          </span>
        )}
      </div>

      {/* Split Pane */}
      <div className="flex min-h-0 flex-1 gap-4">
        {/* Human Panel (Left) */}
        <div className="flex flex-1 flex-col rounded-lg border border-slate-800 bg-slate-900/50">
          {/* Header */}
          <div className="flex items-center justify-between border-b border-slate-800 px-4 py-3">
            <div className="flex items-center gap-2">
              <span
                className={`rounded px-2 py-0.5 text-xs font-bold ${
                  humanSide === "pro"
                    ? "bg-emerald-500/10 text-emerald-400"
                    : "bg-red-500/10 text-red-400"
                }`}
              >
                {humanSideLabel}
              </span>
              <span className="text-sm font-semibold text-slate-300">You</span>
            </div>
          </div>

          {/* Previous rounds */}
          <div className="flex-1 overflow-y-auto px-4 py-3">
            {humanRounds.length === 0 && !isHumanTurn && (
              <p className="text-sm italic text-slate-600">Waiting...</p>
            )}
            {humanRounds.map((r) => (
              <div key={r.round} className="mb-4">
                <span className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-slate-600">
                  {r.phase} — Round {r.round}
                </span>
                <p className="text-sm leading-relaxed text-slate-300">{r.content}</p>
              </div>
            ))}
          </div>

          {/* Input area */}
          {isHumanTurn && (
            <div className="border-t border-slate-800 p-4">
              <textarea
                value={humanInput}
                onChange={(e) => setHumanInput(e.target.value)}
                placeholder={`Write your ${currentStage} argument...`}
                rows={5}
                className="w-full resize-none rounded-lg border border-slate-700 bg-slate-800/50 px-3 py-2 text-sm text-slate-200 placeholder-slate-600 outline-none transition-colors focus:border-gold-500/50"
              />
              <div className="mt-2 flex items-center justify-between">
                <span
                  className={`text-xs font-medium ${
                    overLimit
                      ? "text-red-400"
                      : nearLimit
                        ? "text-amber-400"
                        : "text-slate-500"
                  }`}
                >
                  {words} / {wordLimit} words
                </span>
                <button
                  onClick={() => void handleSubmit()}
                  disabled={!canSubmit}
                  className="rounded-lg bg-gold-500 px-4 py-2 text-xs font-bold text-slate-950 transition-all hover:bg-gold-400 disabled:cursor-not-allowed disabled:opacity-40"
                >
                  Submit
                </button>
              </div>
            </div>
          )}
        </div>

        {/* Center Column */}
        <div className="flex w-20 shrink-0 flex-col items-center justify-center gap-4">
          <div className="flex h-12 w-12 items-center justify-center rounded-full border border-gold-500/30 bg-gold-500/10 text-sm font-black text-gold-400">
            VS
          </div>

          <div className="text-center">
            <span className="text-xs text-slate-500">Phase</span>
            <p className="text-xs font-bold capitalize text-slate-300">
              {currentStage}
            </p>
          </div>

          {(isHumanTurn || isAiTurn) && (
            <button
              onClick={handleAbort}
              className="rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-1.5 text-xs font-medium text-red-400 transition-colors hover:bg-red-500/20"
            >
              Abort
            </button>
          )}

          {phase === "complete" && (
            <span className="rounded bg-emerald-500/10 px-2 py-1 text-xs font-medium text-emerald-400">
              Complete
            </span>
          )}

          {phase === "error" && (
            <span className="max-w-full break-words text-center text-xs text-red-400">
              {errorMessage ?? "Error"}
            </span>
          )}

          {phase === "aborted" && (
            <span className="text-xs text-slate-500">Aborted</span>
          )}

          {isTerminal && (
            <button
              onClick={reset}
              className="rounded-lg bg-slate-800 px-3 py-1.5 text-xs font-medium text-gold-400 transition-colors hover:bg-slate-700"
            >
              New Match
            </button>
          )}
        </div>

        {/* AI Panel (Right) */}
        <div className="flex flex-1 flex-col rounded-lg border border-slate-800 bg-slate-900/50">
          {/* Header */}
          <div className="flex items-center justify-between border-b border-slate-800 px-4 py-3">
            <div className="flex items-center gap-2">
              <span
                className={`rounded px-2 py-0.5 text-xs font-bold ${
                  humanSide === "pro"
                    ? "bg-red-500/10 text-red-400"
                    : "bg-emerald-500/10 text-emerald-400"
                }`}
              >
                {aiSide}
              </span>
              <span className="text-sm font-semibold text-slate-300">
                {model?.display_name ?? "AI"}
              </span>
            </div>
            {difficulty && (
              <span className="rounded bg-slate-800 px-2 py-0.5 text-[10px] uppercase tracking-wider text-slate-500">
                {difficulty}
              </span>
            )}
          </div>

          {/* Previous rounds + streaming */}
          <div className="flex-1 overflow-y-auto px-4 py-3">
            {aiRounds.length === 0 && !isAiTurn && (
              <p className="text-sm italic text-slate-600">
                Waiting for your opening argument...
              </p>
            )}
            {aiRounds.map((r) => (
              <div key={r.round} className="mb-4">
                <span className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-slate-600">
                  {r.phase} — Round {r.round}
                </span>
                <p className="text-sm leading-relaxed text-slate-300">{r.content}</p>
              </div>
            ))}

            {/* Streaming content */}
            {isAiTurn && (
              <div className="mb-4">
                <span className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-slate-600">
                  {currentStage} — responding
                </span>
                <p className="text-sm leading-relaxed text-slate-300">
                  <span ref={aiPanelRef} />
                  <span className="debate-cursor" />
                </p>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

export function SparringRing() {
  const { ollamaOnline } = useAppStore();
  const { phase } = useSparringStore();

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
        <h2 className="text-2xl font-bold text-slate-400">Sparring Ring Offline</h2>
        <p className="text-sm text-slate-500">
          Start Ollama to begin sparring
        </p>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col p-6">
      {phase === "idle" ? <SetupView /> : <DebateView />}
    </div>
  );
}
