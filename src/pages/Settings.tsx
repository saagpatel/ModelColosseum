import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Setting } from "../types";

const ROUND_OPTIONS = [3, 5, 7] as const;

export function Settings() {
  const [settings, setSettings] = useState<Map<string, string>>(new Map());
  const [loaded, setLoaded] = useState(false);
  const [ollamaStatus, setOllamaStatus] = useState<boolean | null>(null);
  const [testingConnection, setTestingConnection] = useState(false);
  const [resetConfirm, setResetConfirm] = useState(false);
  const [resetting, setResetting] = useState(false);

  useEffect(() => {
    invoke<Setting[]>("get_settings")
      .then((rows) => {
        const map = new Map<string, string>();
        for (const r of rows) map.set(r.key, r.value);
        setSettings(map);
        setLoaded(true);
      })
      .catch((err: unknown) => console.error("get_settings error:", err));
  }, []);

  const updateSetting = async (key: string, value: string) => {
    setSettings((prev) => {
      const next = new Map(prev);
      next.set(key, value);
      return next;
    });
    try {
      await invoke("update_setting", { key, value });
    } catch (err) {
      console.error("update_setting error:", err);
    }
  };

  const handleTestConnection = async () => {
    setTestingConnection(true);
    setOllamaStatus(null);
    try {
      const online = await invoke<boolean>("health_check");
      setOllamaStatus(online);
    } catch {
      setOllamaStatus(false);
    } finally {
      setTestingConnection(false);
    }
  };

  const handleResetElo = async () => {
    if (!resetConfirm) {
      setResetConfirm(true);
      return;
    }
    setResetting(true);
    try {
      await invoke("reset_elo_ratings");
      setResetConfirm(false);
    } catch (err) {
      console.error("reset_elo_ratings error:", err);
    } finally {
      setResetting(false);
    }
  };

  const get = (key: string, fallback: string = ""): string => settings.get(key) ?? fallback;

  if (!loaded) {
    return (
      <div className="mx-auto max-w-2xl space-y-10 p-6">
        <div>
          <div className="h-8 w-32 animate-pulse rounded bg-slate-800" />
          <div className="mt-2 h-4 w-64 animate-pulse rounded bg-slate-800" />
        </div>
        <div className="space-y-6">
          <div className="h-5 w-20 animate-pulse rounded border-b border-slate-800 pb-2 bg-slate-800" />
          {Array.from({ length: 3 }, (_, i) => (
            <div key={i} className="flex items-center justify-between">
              <div className="space-y-1">
                <div className="h-4 w-40 animate-pulse rounded bg-slate-800" />
                <div className="h-3 w-56 animate-pulse rounded bg-slate-800" />
              </div>
              <div className="h-8 w-24 animate-pulse rounded-lg bg-slate-800" />
            </div>
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-2xl space-y-10 overflow-y-auto p-6" style={{ maxHeight: "100vh" }}>
      <div>
        <h2 className="text-2xl font-black tracking-tight text-slate-100">Settings</h2>
        <p className="mt-1 text-sm text-slate-500">Configure debate defaults and system preferences</p>
      </div>

      {/* Section 1: General */}
      <section className="space-y-6">
        <h3 className="border-b border-slate-800 pb-2 text-sm font-bold uppercase tracking-wider text-gold-400">
          General
        </h3>

        <div className="flex items-center justify-between">
          <div>
            <span className="block text-sm font-medium text-slate-300">Default Rounds</span>
            <span className="text-xs text-slate-500">Number of rounds for new debates</span>
          </div>
          <div className="flex gap-1 rounded-lg border border-slate-700 bg-slate-800/50 p-1">
            {ROUND_OPTIONS.map((n) => (
              <button
                key={n}
                onClick={() => void updateSetting("default_rounds", String(n))}
                className={`rounded-md px-4 py-1.5 text-sm font-medium transition-colors ${
                  get("default_rounds", "5") === String(n)
                    ? "bg-gold-500/20 text-gold-400"
                    : "text-slate-400 hover:text-slate-200"
                }`}
              >
                {n}
              </button>
            ))}
          </div>
        </div>

        <div className="flex items-center justify-between">
          <div>
            <span className="block text-sm font-medium text-slate-300">Word Limit</span>
            <span className="text-xs text-slate-500">Maximum words per response (100-500)</span>
          </div>
          <input
            type="number"
            min={100}
            max={500}
            step={50}
            value={get("default_word_limit", "300")}
            onChange={(e) => void updateSetting("default_word_limit", e.target.value)}
            className="w-24 rounded-lg border border-slate-700 bg-slate-800/50 px-3 py-2 text-right text-sm text-slate-200 outline-none focus:border-gold-500/50"
          />
        </div>

        <div className="flex items-center justify-between gap-4">
          <div className="min-w-0 flex-1">
            <span className="block text-sm font-medium text-slate-300">Ollama URL</span>
            <span className="text-xs text-slate-500">Base URL for the Ollama API</span>
          </div>
          <div className="flex items-center gap-2">
            <input
              type="text"
              value={get("ollama_url", "http://localhost:11434")}
              onChange={(e) => void updateSetting("ollama_url", e.target.value)}
              className="w-56 rounded-lg border border-slate-700 bg-slate-800/50 px-3 py-2 text-sm text-slate-200 outline-none focus:border-gold-500/50"
            />
            <button
              onClick={() => void handleTestConnection()}
              disabled={testingConnection}
              className="flex items-center gap-1.5 rounded-lg border border-slate-700 bg-slate-800/50 px-3 py-2 text-xs font-medium text-slate-400 transition-colors hover:bg-slate-700 disabled:opacity-40"
            >
              {testingConnection ? "Testing..." : "Test"}
              {ollamaStatus !== null && (
                <span className={`h-2 w-2 rounded-full ${ollamaStatus ? "bg-emerald-400" : "bg-red-400"}`} />
              )}
            </button>
          </div>
        </div>
      </section>

      {/* Section 2: Streaming */}
      <section className="space-y-6">
        <h3 className="border-b border-slate-800 pb-2 text-sm font-bold uppercase tracking-wider text-gold-400">
          Streaming
        </h3>

        <div className="flex items-center justify-between">
          <div>
            <span className="block text-sm font-medium text-slate-300">Concurrent Streaming</span>
            <span className="text-xs text-slate-500">Stream both models simultaneously when possible</span>
          </div>
          <button
            role="switch"
            aria-checked={get("concurrent_streaming", "true") === "true"}
            onClick={() =>
              void updateSetting(
                "concurrent_streaming",
                get("concurrent_streaming", "true") === "true" ? "false" : "true",
              )
            }
            className={`relative h-6 w-11 rounded-full transition-colors ${
              get("concurrent_streaming", "true") === "true" ? "bg-gold-500" : "bg-slate-700"
            }`}
          >
            <span
              className={`absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white transition-transform ${
                get("concurrent_streaming", "true") === "true" ? "translate-x-5" : ""
              }`}
            />
          </button>
        </div>

        <div>
          <div className="mb-2 flex items-center justify-between">
            <div>
              <span className="block text-sm font-medium text-slate-300">Sequential Threshold</span>
              <span className="text-xs text-slate-500">
                Fall back to sequential when combined model params exceed this
              </span>
            </div>
            <span className="font-mono text-sm font-bold text-slate-300">
              {get("concurrent_max_params_billions", "40")}B
            </span>
          </div>
          <input
            type="range"
            min={20}
            max={60}
            step={5}
            value={get("concurrent_max_params_billions", "40")}
            onChange={(e) => void updateSetting("concurrent_max_params_billions", e.target.value)}
            className="w-full accent-gold-500"
          />
          <div className="mt-1 flex justify-between text-[10px] text-slate-600">
            <span>20B</span>
            <span>60B</span>
          </div>
        </div>
      </section>

      {/* Section 3: Elo */}
      <section className="space-y-6">
        <h3 className="border-b border-slate-800 pb-2 text-sm font-bold uppercase tracking-wider text-gold-400">
          Elo Ratings
        </h3>

        <div className="rounded-lg border border-slate-800 bg-slate-900/50 p-4">
          <p className="text-sm leading-relaxed text-slate-400">
            Elo ratings use a K-factor decay system: <span className="font-mono text-slate-300">K=40</span> for new
            models (first 10 games), <span className="font-mono text-slate-300">K=32</span> standard (10-30 games),
            and <span className="font-mono text-slate-300">K=24</span> for veterans (30+ games). This stabilizes
            ratings over time while allowing new models to adjust quickly.
          </p>
        </div>

        <div className="flex items-center justify-between">
          <div>
            <span className="block text-sm font-medium text-slate-300">Reset All Elo Ratings</span>
            <span className="text-xs text-slate-500">Resets all models to 1500 and clears history</span>
          </div>
          <button
            onClick={() => void handleResetElo()}
            disabled={resetting}
            className={`rounded-lg px-4 py-2 text-xs font-bold transition-all ${
              resetConfirm
                ? "bg-red-500 text-white hover:bg-red-600"
                : "border border-red-500/30 bg-red-500/10 text-red-400 hover:bg-red-500/20"
            } disabled:opacity-40`}
          >
            {resetting ? "Resetting..." : resetConfirm ? "Click Again to Confirm" : "Reset Elo"}
          </button>
        </div>
      </section>

      {/* Section 4: System Prompts */}
      <section className="space-y-6 pb-10">
        <h3 className="border-b border-slate-800 pb-2 text-sm font-bold uppercase tracking-wider text-gold-400">
          System Prompts
        </h3>

        <p className="text-xs text-slate-500">
          Default prompts used for each debate format. Custom editing coming in a future update.
        </p>

        <PromptPreview
          label="Arena — Pro"
          content="You are arguing IN FAVOR OF the topic. Establish your position clearly with your strongest arguments. Be persuasive but respectful. Use evidence and logic."
        />
        <PromptPreview
          label="Arena — Con"
          content="You are arguing AGAINST the topic. Respond directly to your opponent's points. Quote their words when rebutting. Never concede without pivoting to a stronger counterargument."
        />
        <PromptPreview
          label="Formal — Opening"
          content="Present your core position on this topic. Do not address your opponent yet. Establish a clear thesis with supporting arguments."
        />
        <PromptPreview
          label="Formal — Rebuttal"
          content="Address each of your opponent's points. Quote their words when rebutting. Identify logical flaws and counter with evidence."
        />
        <PromptPreview
          label="Formal — Closing"
          content="Summarize your strongest arguments. Do not introduce new points. This is your final statement."
        />
        <PromptPreview
          label="Socratic — Questioner"
          content="Ask 2-3 pointed, specific questions that expose weaknesses in your opponent's position. Do not argue — only question."
        />
        <PromptPreview
          label="Socratic — Defender"
          content="Answer each question directly. Defend your position with evidence. Do not dodge."
        />
      </section>
    </div>
  );
}

function PromptPreview({ label, content }: { label: string; content: string }) {
  return (
    <details className="group">
      <summary className="flex cursor-pointer items-center gap-2 text-sm font-medium text-slate-300 hover:text-slate-100">
        <svg
          className="h-3 w-3 text-slate-500 transition-transform group-open:rotate-90"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          strokeWidth={2}
        >
          <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
        </svg>
        {label}
      </summary>
      <div className="ml-5 mt-2 rounded-lg border border-slate-800 bg-slate-900/50 p-3">
        <p className="text-xs leading-relaxed text-slate-400">{content}</p>
      </div>
    </details>
  );
}
