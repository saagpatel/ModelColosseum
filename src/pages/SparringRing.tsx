import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ModelSelector } from "../components/ModelSelector";
import { useAppStore } from "../stores/appStore";
import { useSparringStore } from "../stores/sparringStore";
import { useSparringEvents } from "../hooks/useSparringEvents";
import type { UserStats, SparringScorecard } from "../types";

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
  return round <= 4 ? 1 : 2;
}

function wordCount(text: string): number {
  return text.trim().split(/\s+/).filter(Boolean).length;
}

const DIMENSIONS: { key: "persuasiveness" | "evidence" | "coherence" | "rebuttal"; label: string }[] = [
  { key: "persuasiveness", label: "Persuasiveness" },
  { key: "evidence", label: "Evidence" },
  { key: "coherence", label: "Coherence" },
  { key: "rebuttal", label: "Rebuttal" },
];

function ScorecardView() {
  const { scorecard } = useSparringStore();
  if (!scorecard) return null;

  const parseFailed =
    scorecard.human_persuasiveness === 5 &&
    scorecard.ai_persuasiveness === 5 &&
    scorecard.human_evidence === 5 &&
    scorecard.ai_evidence === 5 &&
    scorecard.human_coherence === 5 &&
    scorecard.ai_coherence === 5 &&
    scorecard.human_rebuttal === 5 &&
    scorecard.ai_rebuttal === 5 &&
    !scorecard.strongest_human_point;

  if (parseFailed) {
    return (
      <div className="mt-4 rounded-xl border border-slate-800 bg-slate-900/50 p-6">
        <p className="mb-3 text-sm font-semibold text-amber-400">
          Judge response could not be parsed into a scorecard.
        </p>
        <details className="text-sm text-slate-400">
          <summary className="cursor-pointer text-xs font-medium uppercase tracking-wider text-slate-500 hover:text-slate-300">
            Raw judge output
          </summary>
          <pre className="mt-3 whitespace-pre-wrap break-words rounded-lg bg-slate-800 p-4 text-xs leading-relaxed text-slate-300">
            {scorecard.raw_judge_output}
          </pre>
        </details>
      </div>
    );
  }

  const humanTotal =
    scorecard.human_persuasiveness +
    scorecard.human_evidence +
    scorecard.human_coherence +
    scorecard.human_rebuttal;
  const aiTotal =
    scorecard.ai_persuasiveness +
    scorecard.ai_evidence +
    scorecard.ai_coherence +
    scorecard.ai_rebuttal;

  function getBarColor(humanScore: number, aiScore: number, isHuman: boolean): string {
    if (humanScore === aiScore) return "bg-amber-500";
    if (isHuman) return humanScore > aiScore ? "bg-emerald-500" : "bg-red-500";
    return aiScore > humanScore ? "bg-emerald-500" : "bg-red-500";
  }

  const humanScores: Record<string, number> = {
    persuasiveness: scorecard.human_persuasiveness,
    evidence: scorecard.human_evidence,
    coherence: scorecard.human_coherence,
    rebuttal: scorecard.human_rebuttal,
  };
  const aiScores: Record<string, number> = {
    persuasiveness: scorecard.ai_persuasiveness,
    evidence: scorecard.ai_evidence,
    coherence: scorecard.ai_coherence,
    rebuttal: scorecard.ai_rebuttal,
  };

  return (
    <div className="mt-4 rounded-xl border border-slate-800 bg-slate-900/50 p-6">
      <div className="mb-6 flex items-center justify-between">
        <h3 className="text-lg font-black tracking-tight text-slate-100">Scorecard</h3>
        <div className="flex gap-6 text-sm">
          <div className="text-center">
            <span className="block font-mono text-2xl font-black text-gold-400">{humanTotal}</span>
            <span className="text-xs text-slate-500">You</span>
          </div>
          <div className="flex items-center text-slate-600">vs</div>
          <div className="text-center">
            <span className="block font-mono text-2xl font-black text-slate-300">{aiTotal}</span>
            <span className="text-xs text-slate-500">AI</span>
          </div>
        </div>
      </div>

      {/* Score bars */}
      <div className="mb-6 flex flex-col gap-3">
        {DIMENSIONS.map(({ key, label }) => {
          const h = humanScores[key] ?? 0;
          const a = aiScores[key] ?? 0;
          return (
            <div key={key} className="flex items-center gap-3">
              {/* Human score */}
              <span className="w-8 text-right font-mono text-sm font-bold text-slate-300">{h}</span>
              {/* Human bar (right-aligned) */}
              <div className="flex flex-1 justify-end">
                <div className="flex w-full justify-end">
                  <div
                    className={`h-4 rounded-l transition-all ${getBarColor(h, a, true)}`}
                    style={{ width: `${(h / 10) * 100}%` }}
                  />
                </div>
              </div>
              {/* Label */}
              <span className="w-28 text-center text-xs font-medium text-slate-400">{label}</span>
              {/* AI bar (left-aligned) */}
              <div className="flex flex-1 justify-start">
                <div
                  className={`h-4 rounded-r transition-all ${getBarColor(h, a, false)}`}
                  style={{ width: `${(a / 10) * 100}%` }}
                />
              </div>
              {/* AI score */}
              <span className="w-8 font-mono text-sm font-bold text-slate-300">{a}</span>
            </div>
          );
        })}
      </div>

      {/* Labels row */}
      <div className="mb-6 flex justify-between text-xs font-semibold text-slate-500">
        <span>YOU</span>
        <span>AI</span>
      </div>

      {/* Feedback cards */}
      <div className="grid grid-cols-2 gap-3">
        <div className="rounded-lg border border-slate-800 bg-slate-900/50 p-4">
          <span className="mb-2 block text-xs font-bold uppercase tracking-wider text-gold-400">
            Strongest Point
          </span>
          <p className="text-sm leading-relaxed text-slate-300">{scorecard.strongest_human_point}</p>
        </div>
        <div className="rounded-lg border border-slate-800 bg-slate-900/50 p-4">
          <span className="mb-2 block text-xs font-bold uppercase tracking-wider text-gold-400">
            Weakest Point
          </span>
          <p className="text-sm leading-relaxed text-slate-300">{scorecard.weakest_human_point}</p>
        </div>
        <div className="rounded-lg border border-slate-800 bg-slate-900/50 p-4">
          <span className="mb-2 block text-xs font-bold uppercase tracking-wider text-gold-400">
            Missed Argument
          </span>
          <p className="text-sm leading-relaxed text-slate-300">{scorecard.missed_argument}</p>
        </div>
        <div className="rounded-lg border border-slate-800 bg-slate-900/50 p-4">
          <span className="mb-2 block text-xs font-bold uppercase tracking-wider text-gold-400">
            Improvement Tip
          </span>
          <p className="text-sm leading-relaxed text-slate-300">{scorecard.improvement_tip}</p>
        </div>
      </div>
    </div>
  );
}

function SetupView() {
  const { models } = useAppStore();
  const { startSparring, setJudgeModelId, judgeModelId } = useSparringStore();

  const [topic, setTopic] = useState("");
  const [side, setSide] = useState<Side>("pro");
  const [modelId, setModelId] = useState<number | null>(null);
  const [localJudgeModelId, setLocalJudgeModelId] = useState<number | null>(null);
  const [difficulty, setDifficulty] = useState<Difficulty>("competitive");
  const [starting, setStarting] = useState(false);
  const [userStats, setUserStats] = useState<UserStats | null>(null);
  const [suggestedTopics, setSuggestedTopics] = useState<string[]>([]);
  const [loadingTopics, setLoadingTopics] = useState(false);

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

  useEffect(() => {
    invoke<UserStats>("get_user_stats")
      .then((stats) => {
        if (stats.total_debates > 0) setUserStats(stats);
      })
      .catch(() => {
        // ignore — no sparring history yet
      });
  }, []);

  // Keep the store's judgeModelId in sync with local state
  useEffect(() => {
    if (localJudgeModelId !== null) {
      setJudgeModelId(localJudgeModelId);
    }
  }, [localJudgeModelId, setJudgeModelId]);

  // Restore local state from store on mount (e.g. if component remounts)
  useEffect(() => {
    if (judgeModelId !== null) setLocalJudgeModelId(judgeModelId);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const canStart = topic.trim().length > 0 && modelId !== null;
  const judgeIsSameAsOpponent = localJudgeModelId !== null && localJudgeModelId === modelId;

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
      {/* User stats banner */}
      {userStats && (
        <div className="flex items-center gap-6 border-b border-slate-800 pb-4">
          <span className="font-mono text-lg font-black text-gold-400">
            {userStats.elo_rating.toFixed(0)}
          </span>
          <span className="text-xs font-light text-slate-500">Your Rating</span>
          <div className="flex gap-4 text-xs text-slate-500">
            <span>
              <span className="font-semibold text-emerald-400">W {userStats.wins}</span>
              {" / "}
              <span className="font-semibold text-red-400">L {userStats.losses}</span>
              {" / "}
              <span className="font-semibold text-slate-400">D {userStats.draws}</span>
            </span>
          </div>
        </div>
      )}

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
          placeholder="e.g. Artificial intelligence will create more jobs than it destroys"
          className="w-full rounded-lg border border-slate-700 bg-slate-800/50 px-4 py-3 text-sm text-slate-200 placeholder-slate-600 outline-none transition-colors focus:border-gold-500/50 focus:ring-1 focus:ring-gold-500/30"
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

      {/* Judge Model Selector */}
      <div>
        <ModelSelector
          models={models}
          selectedId={localJudgeModelId}
          onSelect={setLocalJudgeModelId}
          label="Judge Model"
        />
        {judgeIsSameAsOpponent && (
          <p className="mt-1.5 text-xs text-amber-400">
            Same model as opponent — scores may be biased
          </p>
        )}
      </div>

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
  const [requestingScorecard, setRequestingScorecard] = useState(false);

  const {
    phase,
    debateId,
    humanSide,
    modelId,
    judgeModelId,
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
  const isTerminal = phase === "complete" || phase === "scoring" || phase === "scored" || phase === "error" || phase === "aborted";
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

  const handleRequestScorecard = async () => {
    if (debateId === null || judgeModelId === null) return;
    setRequestingScorecard(true);
    useSparringStore.getState().startScoring();
    try {
      const result = await invoke<SparringScorecard>("request_scorecard", {
        debateId,
        judgeModelId,
      });
      useSparringStore.getState().setScorecard(result);
    } catch (err) {
      console.error("request_scorecard error:", err);
      useSparringStore.getState().setError(String(err));
    } finally {
      setRequestingScorecard(false);
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
        {phase === "scoring" && (
          <span className="animate-pulse rounded-full bg-slate-800 px-4 py-1 text-xs font-semibold text-slate-400">
            Judge is evaluating...
          </span>
        )}
        {phase === "scored" && (
          <span className="rounded-full bg-emerald-500/10 px-4 py-1 text-xs font-semibold text-emerald-400">
            Scorecard Ready
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
            <div className="flex flex-col items-center gap-2">
              {judgeModelId !== null && (
                <button
                  onClick={() => void handleRequestScorecard()}
                  disabled={requestingScorecard}
                  className="rounded-lg bg-gold-500 px-3 py-1.5 text-xs font-bold text-slate-950 transition-all hover:bg-gold-400 disabled:cursor-not-allowed disabled:opacity-40"
                >
                  Scorecard
                </button>
              )}
            </div>
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

      {/* Scorecard (rendered below the split pane when scored) */}
      {phase === "scored" && <ScorecardView />}
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
