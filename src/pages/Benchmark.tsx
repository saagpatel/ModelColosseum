import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { ResponsiveContainer, LineChart, Line, Tooltip } from "recharts";
import { useAppStore } from "../stores/appStore";
import { useBenchmarkStore } from "../stores/benchmarkStore";
import { useBenchmarkEvents } from "../hooks/useBenchmarkEvents";
import { ResultsGrid } from "../components/benchmark/ResultsGrid";
import { ScoreAllMode } from "../components/benchmark/ScoreAllMode";
import { AutoJudgePanel } from "../components/benchmark/AutoJudgePanel";
import { RunHistory } from "../components/benchmark/RunHistory";
import { RunComparison } from "../components/benchmark/RunComparison";
import { BlindCompare } from "../components/benchmark/BlindCompare";
import { RunEvidencePanel } from "../components/benchmark/RunEvidencePanel";
import { downloadBlob } from "../utils/download";
import type { TestSuite, Prompt, BenchmarkResult, EvaluationConfig, ReplayReadiness, ReplayPreparation } from "../types";

type PromptCategory =
  | "coding"
  | "creative"
  | "analysis"
  | "summarization"
  | "reasoning"
  | "conversation"
  | "instruction";

const CATEGORIES: PromptCategory[] = [
  "coding",
  "creative",
  "analysis",
  "summarization",
  "reasoning",
  "conversation",
  "instruction",
];

const categoryBadge: Record<PromptCategory, string> = {
  coding: "bg-blue-500/20 text-blue-400",
  creative: "bg-purple-500/20 text-purple-400",
  analysis: "bg-emerald-500/20 text-emerald-400",
  summarization: "bg-amber-500/20 text-amber-400",
  reasoning: "bg-rose-500/20 text-rose-400",
  conversation: "bg-cyan-500/20 text-cyan-400",
  instruction: "bg-orange-500/20 text-orange-400",
};

function badgeClass(category: string): string {
  return (
    categoryBadge[category as PromptCategory] ?? "bg-slate-500/20 text-slate-400"
  );
}

interface PromptFormState {
  title: string;
  category: PromptCategory;
  text: string;
  system_prompt: string;
  ideal_answer: string;
  eval_criteria: string;
}

const emptyForm = (): PromptFormState => ({
  title: "",
  category: "coding",
  text: "",
  system_prompt: "",
  ideal_answer: "",
  eval_criteria: "",
});

function formatEta(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m}m ${s}s`;
}

// ─── Editing phase ───────────────────────────────────────────────────────────

function SuitesSidebar({
  suites,
  selectedId,
  onSelect,
  onNewSuite,
  onExportSuite,
  onImportSuite,
}: {
  suites: TestSuite[];
  selectedId: number | null;
  onSelect: (id: number) => void;
  onNewSuite: () => void;
  onExportSuite: () => void;
  onImportSuite: () => void;
}) {
  return (
    <div className="flex w-64 shrink-0 flex-col border-r border-slate-700 bg-slate-900">
      <div className="flex h-11 items-center border-b border-slate-700 px-4">
        <span className="text-xs font-semibold uppercase tracking-wider text-slate-500">
          Suites
        </span>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto py-2">
        {suites.length === 0 && (
          <p className="px-4 py-3 text-xs text-slate-600">No suites yet</p>
        )}
        {suites.map((suite) => (
          <button
            key={suite.id}
            onClick={() => onSelect(suite.id)}
            className={`w-full px-4 py-2.5 text-left text-sm transition-colors hover:bg-slate-800 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500 ${
              selectedId === suite.id
                ? "border-r-2 border-gold-500 bg-slate-800/60 font-medium text-gold-400"
                : "text-slate-300"
            }`}
          >
            {suite.name}
            {suite.is_default === 1 && (
              <span className="ml-2 text-xs text-slate-500">default</span>
            )}
          </button>
        ))}
      </div>
      <div className="border-t border-slate-700 p-3 space-y-2">
        <button
          onClick={onNewSuite}
          className="flex w-full items-center justify-center gap-1.5 rounded-lg bg-slate-800 py-2 text-xs font-medium text-slate-300 transition-colors hover:bg-slate-700 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
        >
          <span className="text-base leading-none">+</span> New Suite
        </button>
        <div className="flex gap-2">
          <button
            onClick={onExportSuite}
            className="flex-1 rounded-lg bg-slate-800 py-1.5 text-xs font-medium text-slate-400 transition-colors hover:bg-slate-700 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
          >
            Export
          </button>
          <button
            onClick={onImportSuite}
            className="flex-1 rounded-lg bg-slate-800 py-1.5 text-xs font-medium text-slate-400 transition-colors hover:bg-slate-700 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
          >
            Import
          </button>
        </div>
      </div>
    </div>
  );
}

function CategoryGroup({
  category,
  prompts,
  onMoveUp,
  onMoveDown,
  onDelete,
}: {
  category: string;
  prompts: Prompt[];
  onMoveUp: (id: number) => void;
  onMoveDown: (id: number) => void;
  onDelete: (id: number) => void;
}) {
  const [expanded, setExpanded] = useState(true);

  return (
    <div className="mb-2">
      <button
        onClick={() => setExpanded((v) => !v)}
        className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm font-medium text-slate-300 transition-colors hover:bg-slate-800/50 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
      >
        <span className="text-xs text-slate-500">{expanded ? "▾" : "▸"}</span>
        <span className={`rounded px-1.5 py-0.5 text-xs ${badgeClass(category)}`}>
          {category}
        </span>
        <span className="text-xs text-slate-500">({prompts.length})</span>
      </button>

      {expanded && (
        <div className="ml-4 mt-1 space-y-0.5">
          {prompts.map((prompt, idx) => (
            <div
              key={prompt.id}
              className="group flex items-center gap-2 rounded-md px-2 py-1.5 transition-colors hover:bg-slate-800/40"
            >
              <span className="min-w-0 flex-1 truncate text-sm text-slate-200">
                {prompt.title}
              </span>
              <div className="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100">
                <button
                  onClick={() => onMoveUp(prompt.id)}
                  disabled={idx === 0}
                  className="flex h-6 w-6 items-center justify-center rounded text-xs text-slate-400 transition-colors hover:bg-slate-700 hover:text-slate-200 disabled:cursor-not-allowed disabled:opacity-30 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
                  title="Move up"
                >
                  ↑
                </button>
                <button
                  onClick={() => onMoveDown(prompt.id)}
                  disabled={idx === prompts.length - 1}
                  className="flex h-6 w-6 items-center justify-center rounded text-xs text-slate-400 transition-colors hover:bg-slate-700 hover:text-slate-200 disabled:cursor-not-allowed disabled:opacity-30 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
                  title="Move down"
                >
                  ↓
                </button>
                <button
                  onClick={() => onDelete(prompt.id)}
                  className="flex h-6 w-6 items-center justify-center rounded text-xs text-slate-500 transition-colors hover:bg-red-900/40 hover:text-red-400 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-red-500"
                  title="Delete prompt"
                >
                  ×
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function PromptForm({
  suiteId,
  onAdded,
  onCancel,
}: {
  suiteId: number;
  onAdded: () => void;
  onCancel: () => void;
}) {
  const [form, setForm] = useState<PromptFormState>(emptyForm());
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!form.title.trim() || !form.text.trim()) {
      setError("Title and prompt text are required.");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await invoke("create_prompt", {
        suiteId,
        category: form.category,
        title: form.title.trim(),
        text: form.text.trim(),
        systemPrompt: form.system_prompt.trim() || null,
        idealAnswer: form.ideal_answer.trim() || null,
        evalCriteria: form.eval_criteria.trim() || null,
      });
      onAdded();
    } catch (err) {
      console.error("create_prompt error:", err);
      setError(String(err));
      setSaving(false);
    }
  };

  return (
    <form
      onSubmit={(e) => void handleSubmit(e)}
      className="mt-4 rounded-lg border border-slate-700 bg-slate-800/60 p-4"
    >
      <h4 className="mb-4 text-sm font-semibold text-slate-200">Add Prompt</h4>

      {error && (
        <p className="mb-3 rounded-md bg-red-900/30 px-3 py-2 text-xs text-red-400">
          {error}
        </p>
      )}

      <div className="space-y-3">
        <div className="flex gap-3">
          <div className="flex-1">
            <label className="mb-1 block text-xs font-medium text-slate-400">
              Title <span className="text-red-400">*</span>
            </label>
            <input
              type="text"
              value={form.title}
              onChange={(e) => setForm((f) => ({ ...f, title: e.target.value }))}
              placeholder="e.g. TypeScript Refactor"
              className="w-full rounded-md border border-slate-600 bg-slate-900 px-3 py-2 text-sm text-slate-100 placeholder-slate-600 transition-colors focus:border-gold-500 focus:outline-none"
            />
          </div>
          <div className="w-40">
            <label className="mb-1 block text-xs font-medium text-slate-400">
              Category
            </label>
            <select
              value={form.category}
              onChange={(e) =>
                setForm((f) => ({ ...f, category: e.target.value as PromptCategory }))
              }
              className="w-full rounded-md border border-slate-600 bg-slate-900 px-3 py-2 text-sm text-slate-100 transition-colors focus:border-gold-500 focus:outline-none"
            >
              {CATEGORIES.map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </select>
          </div>
        </div>

        <div>
          <label className="mb-1 block text-xs font-medium text-slate-400">
            Prompt Text <span className="text-red-400">*</span>
          </label>
          <textarea
            value={form.text}
            onChange={(e) => setForm((f) => ({ ...f, text: e.target.value }))}
            placeholder="The prompt sent to the model..."
            rows={3}
            className="w-full resize-y rounded-md border border-slate-600 bg-slate-900 px-3 py-2 text-sm text-slate-100 placeholder-slate-600 transition-colors focus:border-gold-500 focus:outline-none"
          />
        </div>

        <div className="grid grid-cols-3 gap-3">
          <div>
            <label className="mb-1 block text-xs font-medium text-slate-400">
              System Prompt
            </label>
            <textarea
              value={form.system_prompt}
              onChange={(e) =>
                setForm((f) => ({ ...f, system_prompt: e.target.value }))
              }
              placeholder="Optional system prompt..."
              rows={2}
              className="w-full resize-y rounded-md border border-slate-600 bg-slate-900 px-3 py-2 text-sm text-slate-100 placeholder-slate-600 transition-colors focus:border-gold-500 focus:outline-none"
            />
          </div>
          <div>
            <label className="mb-1 block text-xs font-medium text-slate-400">
              Ideal Answer
            </label>
            <textarea
              value={form.ideal_answer}
              onChange={(e) =>
                setForm((f) => ({ ...f, ideal_answer: e.target.value }))
              }
              placeholder="Expected/ideal answer..."
              rows={2}
              className="w-full resize-y rounded-md border border-slate-600 bg-slate-900 px-3 py-2 text-sm text-slate-100 placeholder-slate-600 transition-colors focus:border-gold-500 focus:outline-none"
            />
          </div>
          <div>
            <label className="mb-1 block text-xs font-medium text-slate-400">
              Eval Criteria
            </label>
            <textarea
              value={form.eval_criteria}
              onChange={(e) =>
                setForm((f) => ({ ...f, eval_criteria: e.target.value }))
              }
              placeholder="Scoring criteria..."
              rows={2}
              className="w-full resize-y rounded-md border border-slate-600 bg-slate-900 px-3 py-2 text-sm text-slate-100 placeholder-slate-600 transition-colors focus:border-gold-500 focus:outline-none"
            />
          </div>
        </div>
      </div>

      <div className="mt-4 flex items-center justify-end gap-2">
        <button
          type="button"
          onClick={onCancel}
          className="h-9 rounded-lg bg-slate-700 px-4 text-sm font-medium text-slate-300 transition-colors hover:bg-slate-600 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
        >
          Cancel
        </button>
        <button
          type="submit"
          disabled={saving}
          className="h-9 rounded-lg bg-gold-500 px-4 text-sm font-bold text-slate-950 transition-colors hover:bg-gold-400 disabled:cursor-not-allowed disabled:opacity-60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-gold-400"
        >
          {saving ? "Adding..." : "Add Prompt"}
        </button>
      </div>
    </form>
  );
}

// ─── Configure phase ──────────────────────────────────────────────────────────

function ConfigureModal({
  onStart,
  onCancel,
}: {
  onStart: (modelIds: number[], config: EvaluationConfig) => void;
  onCancel: () => void;
}) {
  const dialogRef = useRef<HTMLFormElement>(null);
  const { models } = useAppStore();
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [repetitions, setRepetitions] = useState(3);
  const [warmups, setWarmups] = useState(1);
  const [timeoutSeconds, setTimeoutSeconds] = useState(120);
  const [temperature, setTemperature] = useState(0.2);
  const [numPredict, setNumPredict] = useState(1024);
  const [think, setThink] = useState(false);
  const [seed, setSeed] = useState("");

  useEffect(() => {
    dialogRef.current?.focus();
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onCancel]);

  const toggle = (id: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const handleSubmit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (selected.size < 2) return;
    onStart(Array.from(selected), {
      repetitions,
      warmup_repetitions: warmups,
      timeout_seconds: timeoutSeconds,
      temperature,
      num_predict: numPredict,
      think,
      seed: seed === "" ? null : Number(seed),
    });
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center backdrop-blur-sm">
      <form
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="benchmark-configure-title"
        tabIndex={-1}
        onSubmit={handleSubmit}
        className="max-h-[90vh] w-full max-w-2xl overflow-y-auto rounded-xl border border-slate-700 bg-slate-900 p-6 shadow-2xl"
      >
        <h2 id="benchmark-configure-title" className="mb-1 text-lg font-bold text-slate-100">
          Select Models to Benchmark
        </h2>
        <p className="mb-5 text-sm text-slate-500">
          Repeated, randomized trials produce uncertainty estimates. Warm-ups are recorded but excluded.
        </p>

        <div className="mb-5 max-h-64 space-y-1 overflow-y-auto">
          {models.length === 0 && (
            <p className="py-4 text-center text-sm text-slate-600">
              No models available — check Ollama
            </p>
          )}
          {models.map((model) => (
            <label
              key={model.id}
              className="flex cursor-pointer items-center gap-3 rounded-lg px-3 py-2.5 transition-colors hover:bg-slate-800"
            >
              <input
                type="checkbox"
                checked={selected.has(model.id)}
                onChange={() => toggle(model.id)}
                className="h-4 w-4 rounded border-slate-600 accent-gold-500 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
              />
              <span className="min-w-0 flex-1">
                <span className="block text-sm text-slate-200">{model.display_name}</span>
                <span className="block truncate font-mono text-[10px] text-slate-500">
                  {model.name} · {model.quantization ?? "quantization unknown"} · {model.digest?.slice(0, 12) ?? "digest unavailable"}
                </span>
              </span>
            </label>
          ))}
        </div>

        <fieldset className="mb-5 rounded-xl border border-slate-800 bg-slate-950/40 p-4">
          <legend className="px-1 text-xs font-bold uppercase tracking-wide text-slate-400">Trial protocol</legend>
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            <NumberField label="Measured repetitions" value={repetitions} min={1} max={20} onChange={setRepetitions} />
            <NumberField label="Warm-ups per model" value={warmups} min={0} max={3} onChange={setWarmups} />
            <NumberField label="Timeout (seconds)" value={timeoutSeconds} min={5} max={3600} onChange={setTimeoutSeconds} />
            <NumberField label="Temperature" value={temperature} min={0} max={2} step={0.1} onChange={setTemperature} />
            <NumberField label="Max generated tokens" value={numPredict} min={1} max={32768} onChange={setNumPredict} />
            <label className="flex items-center gap-2 self-end rounded-lg border border-slate-800 px-3 py-2 text-xs text-slate-400">
              <input
                type="checkbox"
                checked={think}
                onChange={(event) => setThink(event.target.checked)}
                className="h-4 w-4 accent-gold-500"
              />
              Allow model thinking output
            </label>
            <label className="text-xs text-slate-400 sm:col-span-2">
              Replay seed <span className="text-slate-600">(blank = generated and recorded)</span>
              <input
                inputMode="numeric"
                value={seed}
                onChange={(event) => setSeed(event.target.value.replace(/\D/g, ""))}
                placeholder="Auto"
                className="mt-1 h-9 w-full rounded-lg border border-slate-700 bg-slate-900 px-3 font-mono text-sm text-slate-200 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
              />
            </label>
          </div>
          <p className="mt-3 text-xs text-slate-500">
            {selected.size < 2
              ? "Select at least two completion models."
              : `${selected.size} models × ${repetitions} measured repetitions per prompt, plus ${warmups} warm-up${warmups === 1 ? "" : "s"} per model.`}
          </p>
        </fieldset>

        <div className="flex items-center justify-end gap-2">
          <button
            type="button"
            onClick={onCancel}
            className="h-10 rounded-lg bg-slate-800 px-4 text-sm font-medium text-slate-300 transition-colors hover:bg-slate-700 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={selected.size < 2}
            className="h-10 rounded-lg bg-gold-500 px-5 text-sm font-bold text-slate-950 transition-colors hover:bg-gold-400 disabled:cursor-not-allowed disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-gold-400"
          >
            Start Reproducible Run
          </button>
        </div>
      </form>
    </div>
  );
}

function NumberField({
  label,
  value,
  min,
  max,
  step = 1,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="text-xs text-slate-400">
      {label}
      <input
        type="number"
        value={value}
        min={min}
        max={max}
        step={step}
        onChange={(event) => onChange(Number(event.target.value))}
        className="mt-1 h-9 w-full rounded-lg border border-slate-700 bg-slate-900 px-3 font-mono text-sm text-slate-200 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
      />
    </label>
  );
}

// ─── Running phase ────────────────────────────────────────────────────────────

function RunningOverlay({ runId }: { runId: number }) {
  const { progress, streamPreview, startedAt, hardwareMetrics } = useBenchmarkStore();
  const previewRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (previewRef.current) {
      previewRef.current.scrollTop = previewRef.current.scrollHeight;
    }
  }, [streamPreview]);

  const handleCancel = async () => {
    try {
      await invoke("cancel_benchmark", { runId });
    } catch (err) {
      console.error("cancel_benchmark error:", err);
    }
  };

  const pct = progress && progress.total > 0
    ? Math.round((progress.completed / progress.total) * 100)
    : 0;

  let etaLabel = "";
  if (progress && progress.completed > 0 && startedAt !== null) {
    const elapsed = Date.now() - startedAt;
    const remaining = Math.round(
      ((elapsed / progress.completed) * (progress.total - progress.completed)) / 1000,
    );
    etaLabel = `~${formatEta(remaining)} remaining`;
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/80 backdrop-blur-sm">
      <div className="w-full max-w-xl rounded-xl border border-slate-700 bg-slate-900 p-6 shadow-2xl">
        <div className="mb-5 flex items-center justify-between">
          <h2 className="text-lg font-bold text-slate-100">Running Benchmark</h2>
          <button
            onClick={() => void handleCancel()}
            className="h-8 rounded-lg bg-slate-800 px-3 text-xs font-medium text-slate-400 transition-colors hover:bg-slate-700 hover:text-slate-200 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
          >
            Cancel
          </button>
        </div>

        {/* Progress bar */}
        <div className="mb-2 h-2.5 w-full overflow-hidden rounded-full bg-slate-800">
          <div
            className="h-full rounded-full bg-gold-500 transition-all duration-300"
            style={{ width: `${pct}%` }}
          />
        </div>
        <div className="mb-4 flex items-center justify-between text-xs text-slate-500">
          <span>
            {progress
              ? `${progress.completed} / ${progress.total} trials`
              : "Starting..."}
          </span>
          <span>{pct}%</span>
        </div>

        {/* Current context */}
        {progress && (
          <div className="mb-4 space-y-1">
            <p className="text-sm text-slate-400">
              <span className="text-slate-500">Model: </span>
              <span className="font-medium text-slate-200">{progress.currentModel}</span>
            </p>
            <p className="text-sm text-slate-400">
              <span className="text-slate-500">Prompt: </span>
              <span className="font-medium text-slate-200">{progress.currentPrompt}</span>
            </p>
          </div>
        )}

        {/* Stream preview */}
        <div className="rounded-lg border border-slate-700 bg-slate-800/50">
          <div className="border-b border-slate-700 px-3 py-1.5">
            <span className="text-xs font-medium text-slate-500">Live Preview</span>
          </div>
          <div
            ref={previewRef}
            className="max-h-48 overflow-auto p-4 font-mono text-xs leading-relaxed text-slate-300"
          >
            {streamPreview || (
              <span className="text-slate-600">Waiting for output...</span>
            )}
          </div>
        </div>

        {etaLabel && (
          <p className="mt-3 text-xs text-slate-500">{etaLabel}</p>
        )}

        {hardwareMetrics.length > 0 && (
          <div className="mt-4">
            <div className="mb-1.5 flex items-center justify-between">
              <span className="text-xs font-medium text-slate-500">System Metrics</span>
              <div className="flex items-center gap-3">
                <span className="flex items-center gap-1 text-[10px] text-slate-500">
                  <span className="inline-block h-2 w-3 rounded-sm bg-amber-500" /> CPU
                </span>
                <span className="flex items-center gap-1 text-[10px] text-slate-500">
                  <span className="inline-block h-2 w-3 rounded-sm bg-blue-400" /> Mem
                </span>
              </div>
            </div>
            <ResponsiveContainer width="100%" height={80}>
              <LineChart data={hardwareMetrics}>
                <Tooltip
                  contentStyle={{ background: "#1e293b", border: "1px solid #334155", fontSize: 11 }}
                  formatter={(value, name) => [
                    typeof value === "number" ? `${value.toFixed(1)}%` : "—",
                    String(name),
                  ]}
                />
                <Line
                  type="monotone"
                  dataKey="cpu_percent"
                  name="CPU"
                  stroke="#f59e0b"
                  strokeWidth={1.5}
                  dot={false}
                  isAnimationActive={false}
                />
                <Line
                  type="monotone"
                  dataKey="memory_percent"
                  name="Memory"
                  stroke="#60a5fa"
                  strokeWidth={1.5}
                  dot={false}
                  isAnimationActive={false}
                />
              </LineChart>
            </ResponsiveContainer>
          </div>
        )}
      </div>
    </div>
  );
}

// ─── Main page ────────────────────────────────────────────────────────────────

export function Benchmark() {
  const {
    phase,
    suites,
    selectedSuiteId,
    prompts,
    runId,
    results,
    blindMode,
    scoreAllMode,
    viewingRunId,
    setSuites,
    selectSuite,
    setPrompts,
    startConfiguring,
    startRun,
    viewRun,
    toggleBlindMode,
    enterScoreAllMode,
    reset,
  } = useBenchmarkStore();

  const [showAddPrompt, setShowAddPrompt] = useState(false);
  const [newSuiteName, setNewSuiteName] = useState("");
  const [showNewSuiteInput, setShowNewSuiteInput] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [showRunHistory, setShowRunHistory] = useState(false);
  const [compareRuns, setCompareRuns] = useState<[number, number] | null>(null);
  const [showAutoJudge, setShowAutoJudge] = useState(false);
  const [showBlindCompare, setShowBlindCompare] = useState(false);
  const [blindOnePerPrompt, setBlindOnePerPrompt] = useState(false);
  const [evidenceRevision, setEvidenceRevision] = useState(0);
  const importFileRef = useRef<HTMLInputElement>(null);
  const replayFileRef = useRef<HTMLInputElement>(null);
  const replayDialogRef = useRef<HTMLElement>(null);
  const [replayJson, setReplayJson] = useState<string | null>(null);
  const [replayReadiness, setReplayReadiness] = useState<ReplayReadiness | null>(null);
  const [replayBusy, setReplayBusy] = useState(false);

  useEffect(() => {
    if (!replayReadiness) return;
    replayDialogRef.current?.focus();
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !replayBusy) {
        setReplayReadiness(null);
        setReplayJson(null);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [replayReadiness, replayBusy]);

  useBenchmarkEvents(runId);

  // Auto-load results when benchmark completes
  useEffect(() => {
    if (phase !== "complete" || runId === null) return;
    const load = async () => {
      try {
        const data = await invoke<BenchmarkResult[]>("get_benchmark_results", { runId });
        viewRun(runId, data);
      } catch (err) {
        console.error("get_benchmark_results error:", err);
      }
    };
    void load();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [phase, runId]);

  // Load suites on mount
  useEffect(() => {
    const load = async () => {
      try {
        const result = await invoke<TestSuite[]>("list_test_suites");
        setSuites(result);
        if (result.length > 0 && selectedSuiteId === null) {
          const defaultSuite = result.find((s) => s.is_default === 1) ?? result[0];
          if (defaultSuite) selectSuite(defaultSuite.id);
        }
      } catch (err) {
        console.error("list_suites error:", err);
        setLoadError(String(err));
      }
    };
    void load();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Load prompts when selected suite changes
  useEffect(() => {
    if (selectedSuiteId === null) return;
    const load = async () => {
      try {
        const result = await invoke<Prompt[]>("list_prompts", {
          suiteId: selectedSuiteId,
        });
        setPrompts(result);
      } catch (err) {
        console.error("list_prompts error:", err);
      }
    };
    void load();
  }, [selectedSuiteId, setPrompts]);

  const reloadPrompts = async () => {
    if (selectedSuiteId === null) return;
    try {
      const result = await invoke<Prompt[]>("list_prompts", {
        suiteId: selectedSuiteId,
      });
      setPrompts(result);
    } catch (err) {
      console.error("list_prompts error:", err);
    }
  };

  const handleSelectSuite = (id: number) => {
    setShowAddPrompt(false);
    selectSuite(id);
  };

  const handleNewSuite = async () => {
    if (!newSuiteName.trim()) return;
    try {
      await invoke("create_test_suite", { name: newSuiteName.trim() });
      const result = await invoke<TestSuite[]>("list_test_suites");
      setSuites(result);
      setNewSuiteName("");
      setShowNewSuiteInput(false);
    } catch (err) {
      console.error("create_suite error:", err);
    }
  };

  const handleMoveUp = async (promptId: number) => {
    const idx = prompts.findIndex((p) => p.id === promptId);
    if (idx <= 0) return;
    const prev = prompts[idx - 1] as Prompt;
    const curr = prompts[idx] as Prompt;
    try {
      await invoke("reorder_prompts", {
        items: [
          { id: curr.id, sort_order: prev.sort_order },
          { id: prev.id, sort_order: curr.sort_order },
        ],
      });
      await reloadPrompts();
    } catch (err) {
      console.error("reorder_prompts error:", err);
    }
  };

  const handleMoveDown = async (promptId: number) => {
    const idx = prompts.findIndex((p) => p.id === promptId);
    if (idx < 0 || idx >= prompts.length - 1) return;
    const curr = prompts[idx] as Prompt;
    const next = prompts[idx + 1] as Prompt;
    try {
      await invoke("reorder_prompts", {
        items: [
          { id: curr.id, sort_order: next.sort_order },
          { id: next.id, sort_order: curr.sort_order },
        ],
      });
      await reloadPrompts();
    } catch (err) {
      console.error("reorder_prompts error:", err);
    }
  };

  const handleDelete = async (promptId: number) => {
    try {
      await invoke("delete_prompt", { id: promptId });
      await reloadPrompts();
    } catch (err) {
      console.error("delete_prompt error:", err);
    }
  };

  const handleScoreChange = async (resultId: number, score: number) => {
    try {
      await invoke("score_result", { resultId, score });
      useBenchmarkStore.getState().updateResultScore(resultId, score);
    } catch (err) {
      console.error("score_result error:", err);
    }
  };

  const handleViewRun = async (rid: number) => {
    try {
      const data = await invoke<BenchmarkResult[]>("get_benchmark_results", { runId: rid });
      viewRun(rid, data);
      setShowRunHistory(false);
    } catch (err) {
      console.error("get_benchmark_results error:", err);
    }
  };

  const handleStartBenchmark = async (modelIds: number[], config: EvaluationConfig) => {
    if (selectedSuiteId === null) return;
    try {
      const newRunId = await invoke<number>("start_benchmark", {
        suiteId: selectedSuiteId,
        modelIds,
        config,
      });
      startRun(newRunId);
    } catch (err) {
      console.error("start_benchmark error:", err);
      useBenchmarkStore.getState().setError(String(err));
    }
  };

  const handleExportSuite = async () => {
    if (selectedSuiteId === null) return;
    try {
      const json = await invoke<string>("export_test_suite", { suiteId: selectedSuiteId });
      const suiteName = suites.find((s) => s.id === selectedSuiteId)?.name ?? "suite";
      downloadBlob(json, `${suiteName}.json`, "application/json");
    } catch (err) {
      console.error("export_test_suite error:", err);
    }
  };

  const handleImportSuite = () => {
    importFileRef.current?.click();
  };

  const handleImportFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setLoadError(null);
    const reader = new FileReader();
    reader.onload = async (ev) => {
      const jsonData = ev.target?.result as string;
      try {
        await invoke("import_test_suite", { jsonData });
        const result = await invoke<TestSuite[]>("list_test_suites");
        setSuites(result);
      } catch (err) {
        console.error("import_test_suite error:", err);
        setLoadError(`Suite import failed: ${String(err)}`);
      }
    };
    reader.onerror = () => {
      setLoadError(`Suite import failed: could not read ${file.name}`);
    };
    reader.readAsText(file);
    // Reset so same file can be re-imported
    e.target.value = "";
  };

  const handleReplayFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setLoadError(null);
    setReplayBusy(true);
    try {
      const jsonData = await file.text();
      const readiness = await invoke<ReplayReadiness>("inspect_replay_bundle", { jsonData });
      setReplayJson(jsonData);
      setReplayReadiness(readiness);
    } catch (err) {
      setLoadError(`Replay inspection failed: ${String(err)}`);
      setReplayJson(null);
      setReplayReadiness(null);
    } finally {
      setReplayBusy(false);
      e.target.value = "";
    }
  };

  const handlePrepareReplay = async () => {
    if (!replayJson || !replayReadiness?.ready) return;
    setReplayBusy(true);
    try {
      const prepared = await invoke<ReplayPreparation>("prepare_replay_bundle", { jsonData: replayJson });
      const newRunId = await invoke<number>("start_benchmark", {
        suiteId: prepared.suite_id,
        modelIds: prepared.model_ids,
        config: prepared.config,
        replaySourceManifestDigest: prepared.source_manifest_digest,
        replaySourceRunKey: prepared.source_run_key,
      });
      setReplayJson(null);
      setReplayReadiness(null);
      startRun(newRunId);
    } catch (err) {
      setLoadError(`Replay start failed: ${String(err)}`);
    } finally {
      setReplayBusy(false);
    }
  };

  const handleExportReport = async () => {
    if (viewingRunId === null) return;
    try {
      const path = await save({
        defaultPath: `evaluation-run-${viewingRunId}.json`,
        filters: [{ name: "Evaluation evidence", extensions: ["json"] }],
      });
      if (!path) return;
      await invoke("save_evaluation_bundle", { runId: viewingRunId, path });
    } catch (err) {
      console.error("export_evaluation_bundle error:", err);
    }
  };

  // Group prompts by category
  const byCategory: Record<string, Prompt[]> = {};
  for (const p of prompts) {
    (byCategory[p.category] ??= []).push(p);
  }
  const categoryKeys = Object.keys(byCategory);

  const selectedSuite = suites.find((s) => s.id === selectedSuiteId);

  // ── Phase: running ──
  if (phase === "running" && runId !== null) {
    return <RunningOverlay runId={runId} />;
  }

  // ── Phase: complete (transitioning to results via useEffect) ──
  if (phase === "complete") {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-4">
        <div className="flex h-16 w-16 items-center justify-center rounded-full bg-gold-500/20 text-4xl">
          ✓
        </div>
        <h2 className="text-2xl font-bold text-slate-100">Benchmark complete!</h2>
        <p className="animate-pulse text-sm text-slate-500">Loading results...</p>
      </div>
    );
  }

  // ── Phase: results ──
  if (phase === "results") {
    // Run comparison view
    if (compareRuns !== null) {
      return (
        <div className="flex h-full flex-col bg-slate-950">
          <RunComparison
            runA={compareRuns[0]}
            runB={compareRuns[1]}
            onBack={() => setCompareRuns(null)}
          />
        </div>
      );
    }

    // Run history view
    if (showRunHistory) {
      return (
        <div className="flex h-full flex-col bg-slate-950">
          <div className="flex shrink-0 items-center gap-4 border-b border-slate-800 px-6 py-4">
            <button
              onClick={() => setShowRunHistory(false)}
              className="flex items-center gap-1.5 text-sm text-slate-400 transition-colors hover:text-slate-200 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
            >
              ← Back to Results
            </button>
            <h2 className="text-base font-semibold text-slate-100">Past Runs</h2>
          </div>
          <RunHistory
            onViewRun={(rid) => void handleViewRun(rid)}
            onCompare={(a, b) => {
              setCompareRuns([a, b]);
              setShowRunHistory(false);
            }}
          />
        </div>
      );
    }

    // Score all mode
    if (scoreAllMode) {
      return (
        <ScoreAllMode onScoreChange={(id, score) => void handleScoreChange(id, score)} />
      );
    }

    // Main results grid
    return (
      <div className="flex h-full flex-col bg-slate-950">
        {/* Blind compare overlay */}
        {showBlindCompare && viewingRunId !== null && (
          <BlindCompare
            runId={viewingRunId}
            onePerPrompt={blindOnePerPrompt}
            onClose={() => {
              setShowBlindCompare(false);
              setEvidenceRevision((revision) => revision + 1);
            }}
          />
        )}

        {/* Auto-judge overlay */}
        {showAutoJudge && viewingRunId !== null && (
          <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/80 backdrop-blur-sm">
            <div className="w-full max-w-md">
              <AutoJudgePanel
                runId={viewingRunId}
                onComplete={async () => {
                  setShowAutoJudge(false);
                  if (viewingRunId !== null) {
                    try {
                      const data = await invoke<BenchmarkResult[]>("get_benchmark_results", {
                        runId: viewingRunId,
                      });
                      viewRun(viewingRunId, data);
                    } catch (err) {
                      console.error("get_benchmark_results error:", err);
                    }
                  }
                }}
              />
              <button
                onClick={() => setShowAutoJudge(false)}
                className="mt-3 w-full rounded-lg bg-slate-800 py-2 text-sm font-medium text-slate-400 transition-colors hover:bg-slate-700 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
              >
                Close
              </button>
            </div>
          </div>
        )}

        {/* Header */}
        <div className="flex shrink-0 items-center justify-between border-b border-slate-800 px-6 py-3">
          <div className="flex items-center gap-3">
            <button
              onClick={reset}
              className="flex items-center gap-1.5 text-sm text-slate-400 transition-colors hover:text-slate-200 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
            >
              ← Editor
            </button>
            <span className="text-slate-700">|</span>
            <h1 className="text-sm font-bold text-slate-100">Benchmark Results</h1>
            {viewingRunId !== null && (
              <span className="rounded bg-slate-800 px-2 py-0.5 text-xs text-slate-500">
                Run #{viewingRunId}
              </span>
            )}
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => setShowRunHistory(true)}
              className="h-8 rounded-lg bg-slate-800 px-3 text-xs font-medium text-slate-300 transition-colors hover:bg-slate-700 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
            >
              Past Runs
            </button>
            <button
              onClick={toggleBlindMode}
              className={`h-8 rounded-lg px-3 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500 ${
                blindMode
                  ? "bg-gold-500 text-slate-950"
                  : "bg-slate-800 text-slate-300 hover:bg-slate-700"
              }`}
            >
              {blindMode ? "Blind ON" : "Blind Mode"}
            </button>
            <button
              onClick={() => {
                setBlindOnePerPrompt(true);
                setShowBlindCompare(true);
              }}
              disabled={new Set(results.map((r) => r.model_id)).size !== 2}
              className="h-8 rounded-lg bg-gold-500/15 px-3 text-xs font-medium text-gold-300 transition-colors hover:bg-gold-500/25 disabled:cursor-not-allowed disabled:opacity-40 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
              title="Judge one deterministic randomized trial per prompt"
            >
              Blind Sample
            </button>
            <button
              onClick={() => {
                setBlindOnePerPrompt(false);
                setShowBlindCompare(true);
              }}
              disabled={new Set(results.map((r) => r.model_id)).size !== 2}
              className="h-8 rounded-lg bg-slate-800 px-3 text-xs font-medium text-slate-300 transition-colors hover:bg-slate-700 disabled:cursor-not-allowed disabled:opacity-40 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
            >
              Blind Compare All
            </button>
            <button
              onClick={() => setShowAutoJudge(true)}
              className="h-8 rounded-lg bg-slate-800 px-3 text-xs font-medium text-slate-300 transition-colors hover:bg-slate-700 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
            >
              Auto-Judge
            </button>
            <button
              onClick={() => void handleExportReport()}
              className="h-8 rounded-lg bg-slate-800 px-3 text-xs font-medium text-slate-300 transition-colors hover:bg-slate-700 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
            >
              Export Evidence
            </button>
            <button
              onClick={enterScoreAllMode}
              className="h-8 rounded-lg bg-gold-500 px-3 text-xs font-bold text-slate-950 transition-colors hover:bg-gold-400 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-gold-400"
            >
              Score All
            </button>
          </div>
        </div>

        <div className="min-h-0 flex-1 overflow-auto">
          {viewingRunId !== null && (
            <RunEvidencePanel
              runId={viewingRunId}
              refreshKey={`${evidenceRevision}:${results.map((result) => `${result.id}:${result.manual_score ?? "-"}:${result.auto_judge_score ?? "-"}`).join("|")}`}
            />
          )}
          <ResultsGrid
            results={results}
            blindMode={blindMode}
            onScoreChange={(id, score) => void handleScoreChange(id, score)}
          />
        </div>
      </div>
    );
  }

  // ── Phase: error ──
  if (phase === "error") {
    const errorMessage = useBenchmarkStore.getState().errorMessage;
    const wasCancelled = errorMessage === "Benchmark cancelled";

    return (
      <div className="flex h-full flex-col items-center justify-center gap-4">
        <div
          className={`flex h-16 w-16 items-center justify-center rounded-full text-4xl ${
            wasCancelled
              ? "bg-amber-500/20 text-amber-400"
              : "bg-red-500/20 text-red-400"
          }`}
        >
          {wasCancelled ? "■" : "✗"}
        </div>
        <h2
          className={`text-2xl font-bold ${wasCancelled ? "text-amber-400" : "text-red-400"}`}
        >
          {wasCancelled ? "Benchmark cancelled" : "Benchmark failed"}
        </h2>
        <p className="max-w-sm text-center text-sm text-slate-500">
          {wasCancelled
            ? "Completed work was preserved for audit, and all unfinished trials were marked cancelled and excluded from recommendations."
            : errorMessage ?? "An unknown error occurred."}
        </p>
        <button
          onClick={reset}
          className="h-10 rounded-lg bg-slate-800 px-6 text-sm font-medium text-slate-300 transition-colors hover:bg-slate-700 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
        >
          Back to Editor
        </button>
      </div>
    );
  }

  // ── Phase: editing + configuring ──

  if (compareRuns !== null) {
    return (
      <div className="flex h-full flex-col bg-slate-950">
        <RunComparison
          runA={compareRuns[0]}
          runB={compareRuns[1]}
          onBack={() => {
            setCompareRuns(null);
            setShowRunHistory(true);
          }}
        />
      </div>
    );
  }

  // Past runs modal from editor
  if (showRunHistory) {
    return (
      <div className="flex h-full flex-col bg-slate-950">
        <div className="flex shrink-0 items-center gap-4 border-b border-slate-800 px-6 py-4">
          <button
            onClick={() => setShowRunHistory(false)}
            className="flex items-center gap-1.5 text-sm text-slate-400 transition-colors hover:text-slate-200 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
          >
            ← Back to Editor
          </button>
          <h2 className="text-base font-semibold text-slate-100">Past Runs</h2>
        </div>
        <RunHistory
          onViewRun={(rid) => void handleViewRun(rid)}
          onCompare={(a, b) => {
            setCompareRuns([a, b]);
            setShowRunHistory(false);
          }}
        />
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col bg-slate-950">
      {replayReadiness && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/85 p-4 backdrop-blur-sm">
          <section ref={replayDialogRef} role="dialog" aria-modal="true" aria-labelledby="replay-readiness-title" tabIndex={-1} className="max-h-[90vh] w-full max-w-2xl overflow-auto rounded-xl border border-slate-700 bg-slate-900 p-4 shadow-2xl focus:outline-none sm:p-6">
            <h2 id="replay-readiness-title" className="text-xl font-bold text-slate-100">Replay readiness</h2>
            <p className="mt-1 text-sm text-slate-400">{replayReadiness.classification.replaceAll("_", " ")} · source {replayReadiness.source_manifest_digest.slice(0, 12)}</p>
            <div className="mt-4 grid gap-2 sm:grid-cols-3">
              <div className="rounded-lg bg-slate-950 p-3 text-xs text-slate-300">Ollama<br/><strong>{replayReadiness.ollama_available ? (replayReadiness.ollama_version_matches ? "version matched" : "version differs") : "unavailable"}</strong></div>
              <div className="rounded-lg bg-slate-950 p-3 text-xs text-slate-300">Hardware<br/><strong>{replayReadiness.hardware_matches ? "matched" : "variant"}</strong></div>
              <div className="rounded-lg bg-slate-950 p-3 text-xs text-slate-300">Source run<br/><strong>{replayReadiness.source_run_valid ? "valid" : "invalid"}</strong></div>
            </div>
            <ul className="mt-4 space-y-2" aria-label="Required model readiness">
              {replayReadiness.models.map((model) => <li key={model.exact_tag} className="flex justify-between rounded-lg border border-slate-800 px-3 py-2 text-sm"><span className="text-slate-200">{model.exact_tag}</span><span className={model.status === "matched" ? "text-emerald-400" : "text-red-400"}>{model.status.replaceAll("_", " ")}</span></li>)}
            </ul>
            {replayReadiness.blockers.length > 0 && <div role="alert" className="mt-4 rounded-lg bg-red-950/50 p-3 text-sm text-red-300">{replayReadiness.blockers.join(" · ")}</div>}
            {replayReadiness.warnings.length > 0 && <div className="mt-4 rounded-lg bg-amber-950/40 p-3 text-sm text-amber-200">{replayReadiness.warnings.join(" · ")}</div>}
            <p className="mt-4 text-xs text-slate-500">ModelColosseum will not download missing models. Preparing creates a local suite snapshot and links the new run to this source manifest.</p>
            <div className="mt-6 flex justify-end gap-3">
              <button type="button" onClick={() => { setReplayReadiness(null); setReplayJson(null); }} className="rounded-lg bg-slate-800 px-4 py-2 text-sm text-slate-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-slate-400">Cancel</button>
              <button type="button" disabled={!replayReadiness.ready || replayBusy} onClick={() => void handlePrepareReplay()} className="rounded-lg bg-gold-500 px-4 py-2 text-sm font-bold text-slate-950 disabled:cursor-not-allowed disabled:opacity-40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-gold-300">{replayBusy ? "Preparing…" : "Prepare & Run"}</button>
            </div>
          </section>
        </div>
      )}
      {/* Configure modal overlay */}
      {phase === "configuring" && (
        <ConfigureModal
          onStart={(ids, config) => void handleStartBenchmark(ids, config)}
          onCancel={reset}
        />
      )}

      {/* Header */}
      <div className="flex min-h-14 shrink-0 flex-wrap items-center justify-between gap-2 border-b border-slate-800 px-4 py-2 sm:px-6">
        <h1 className="text-base font-bold text-slate-100">Benchmark Mode</h1>
        <div className="flex flex-wrap items-center justify-end gap-2">
          <button
            onClick={() => setShowRunHistory(true)}
            className="h-9 rounded-lg bg-slate-800 px-4 text-sm font-medium text-slate-300 transition-colors hover:bg-slate-700 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
          >
            Past Runs
          </button>
          <button
            type="button"
            onClick={() => replayFileRef.current?.click()}
            disabled={replayBusy}
            className="h-9 rounded-lg border border-slate-700 px-4 text-sm font-medium text-slate-300 transition-colors hover:border-gold-500 hover:text-gold-300 disabled:opacity-40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-gold-400"
          >
            Import Replay
          </button>
          <button
            onClick={startConfiguring}
            disabled={selectedSuiteId === null || prompts.length === 0}
            className="h-10 rounded-lg bg-gold-500 px-5 text-sm font-bold text-slate-950 transition-colors hover:bg-gold-400 disabled:cursor-not-allowed disabled:opacity-40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-gold-400"
          >
            Run Benchmark
          </button>
        </div>
      </div>

      {loadError && (
        <div
          role="alert"
          className="mx-4 mt-4 flex items-start justify-between gap-3 rounded-lg bg-red-900/30 px-4 py-3 text-sm text-red-400 sm:mx-6"
        >
          <span>{loadError}</span>
          <button
            type="button"
            onClick={() => setLoadError(null)}
            className="shrink-0 rounded px-2 py-1 text-xs font-medium text-red-300 hover:bg-red-900/40 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-red-400"
            aria-label="Dismiss import error"
          >
            Dismiss
          </button>
        </div>
      )}

      {/* Body */}
      <div className="flex min-h-0 flex-1">
        {/* Hidden file input for suite import */}
        <input
          ref={importFileRef}
          type="file"
          accept=".json"
          className="hidden"
          onChange={(e) => void handleImportFileChange(e)}
        />
        <input
          ref={replayFileRef}
          type="file"
          accept="application/json,.json"
          onChange={(event) => void handleReplayFileChange(event)}
          className="hidden"
          aria-label="Import evaluation evidence for replay"
        />

        {/* Sidebar */}
        <SuitesSidebar
          suites={suites}
          selectedId={selectedSuiteId}
          onSelect={handleSelectSuite}
          onNewSuite={() => setShowNewSuiteInput(true)}
          onExportSuite={() => void handleExportSuite()}
          onImportSuite={handleImportSuite}
        />

        {/* New suite input (shown inline in sidebar area via absolute, or inline below) */}
        {showNewSuiteInput && (
          <div className="fixed inset-0 z-40 flex items-center justify-center backdrop-blur-sm">
            <div className="w-full max-w-sm rounded-xl border border-slate-700 bg-slate-900 p-5 shadow-2xl">
              <h3 className="mb-3 text-sm font-semibold text-slate-200">New Suite</h3>
              <input
                autoFocus
                type="text"
                value={newSuiteName}
                onChange={(e) => setNewSuiteName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void handleNewSuite();
                  if (e.key === "Escape") setShowNewSuiteInput(false);
                }}
                placeholder="Suite name..."
                className="w-full rounded-md border border-slate-600 bg-slate-800 px-3 py-2 text-sm text-slate-100 placeholder-slate-600 focus:border-gold-500 focus:outline-none"
              />
              <div className="mt-3 flex justify-end gap-2">
                <button
                  onClick={() => setShowNewSuiteInput(false)}
                  className="h-9 rounded-lg bg-slate-700 px-3 text-xs font-medium text-slate-300 transition-colors hover:bg-slate-600"
                >
                  Cancel
                </button>
                <button
                  onClick={() => void handleNewSuite()}
                  disabled={!newSuiteName.trim()}
                  className="h-9 rounded-lg bg-gold-500 px-4 text-xs font-bold text-slate-950 transition-colors hover:bg-gold-400 disabled:opacity-40"
                >
                  Create
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Prompt list */}
        <div className="flex min-h-0 flex-1 flex-col overflow-y-auto p-6">
          {selectedSuite ? (
            <>
              <div className="mb-4 flex items-baseline justify-between">
                <h2 className="text-base font-semibold text-slate-200">
                  Prompts for{" "}
                  <span className="text-gold-400">"{selectedSuite.name}"</span>
                </h2>
                <span className="text-xs text-slate-500">
                  {prompts.length} prompt{prompts.length !== 1 ? "s" : ""}
                </span>
              </div>

              {prompts.length === 0 && !showAddPrompt && (
                <div className="flex flex-col items-center justify-center gap-3 rounded-xl border border-dashed border-slate-700 py-16 text-center">
                  <div className="text-3xl text-slate-700">📋</div>
                  <h3 className="text-sm font-medium text-slate-400">No prompts yet</h3>
                  <p className="text-xs text-slate-600">
                    Add prompts to this suite to start benchmarking
                  </p>
                  <button
                    onClick={() => setShowAddPrompt(true)}
                    className="mt-1 h-9 rounded-lg bg-gold-500 px-4 text-xs font-bold text-slate-950 transition-colors hover:bg-gold-400"
                  >
                    + Add Prompt
                  </button>
                </div>
              )}

              {categoryKeys.map((cat) => (
                <CategoryGroup
                  key={cat}
                  category={cat}
                  prompts={byCategory[cat] ?? []}
                  onMoveUp={handleMoveUp}
                  onMoveDown={handleMoveDown}
                  onDelete={handleDelete}
                />
              ))}

              {prompts.length > 0 && !showAddPrompt && (
                <button
                  onClick={() => setShowAddPrompt(true)}
                  className="mt-4 flex items-center gap-1.5 self-start text-sm font-medium text-gold-400 transition-colors hover:text-gold-300 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500"
                >
                  <span className="text-base leading-none">+</span> Add Prompt
                </button>
              )}

              {showAddPrompt && selectedSuiteId !== null && (
                <PromptForm
                  suiteId={selectedSuiteId}
                  onAdded={async () => {
                    setShowAddPrompt(false);
                    await reloadPrompts();
                  }}
                  onCancel={() => setShowAddPrompt(false)}
                />
              )}
            </>
          ) : (
            <div className="flex flex-col items-center justify-center gap-3 py-24 text-center">
              <h3 className="text-sm font-medium text-slate-500">
                Select or create a suite to get started
              </h3>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
