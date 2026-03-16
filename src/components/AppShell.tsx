import { useEffect } from "react";
import { NavLink, Outlet } from "react-router";
import { useAppStore } from "../stores/appStore";
import { useDebateStore } from "../stores/debateStore";
import { useSparringStore } from "../stores/sparringStore";

const tabs = [
  { name: "Arena", to: "/", enabled: true },
  { name: "Benchmark", to: "/benchmark", enabled: true },
  { name: "Sparring", to: "/sparring", enabled: true },
  { name: "Leaderboard", to: "/leaderboard", enabled: true },
  { name: "History", to: "/history", enabled: true },
  { name: "Settings", to: "/settings", enabled: false },
] as const;

export function AppShell() {
  const ollamaOnline = useAppStore((s) => s.ollamaOnline);
  const debatePhase = useDebateStore((s) => s.phase);
  const sparringPhase = useSparringStore((s) => s.phase);

  useEffect(() => {
    void useAppStore.getState().init();
  }, []);

  return (
    <div className="flex h-screen flex-col bg-slate-950">
      {/* Nav Bar */}
      <nav className="flex h-12 shrink-0 items-center justify-between border-b border-slate-800 px-6">
        {/* Title */}
        <span className="text-sm font-black tracking-tight text-gold-400">
          Model Colosseum
        </span>

        {/* Tabs */}
        <div className="flex items-center gap-1">
          {tabs.map((tab) =>
            tab.enabled ? (
              <NavLink
                key={tab.name}
                to={tab.to}
                className={({ isActive }) =>
                  `relative px-3 py-3 text-xs font-medium transition-colors ${
                    isActive
                      ? "border-b-2 border-gold-500 text-gold-400"
                      : "text-slate-400 hover:text-slate-200"
                  }`
                }
              >
                {tab.name}
                {tab.name === "Arena" && debatePhase === "debating" && (
                  <span className="absolute -right-0.5 top-2 h-1.5 w-1.5 animate-pulse rounded-full bg-gold-400" />
                )}
                {tab.name === "Sparring" && (sparringPhase === "human_turn" || sparringPhase === "ai_turn") && (
                  <span className="absolute -right-0.5 top-2 h-1.5 w-1.5 animate-pulse rounded-full bg-gold-400" />
                )}
              </NavLink>
            ) : (
              <span
                key={tab.name}
                className="cursor-not-allowed px-3 py-3 text-xs font-medium text-slate-400 opacity-40"
              >
                {tab.name}
              </span>
            ),
          )}
        </div>

        {/* Ollama Status */}
        <div className="flex items-center gap-2">
          <div
            className={`h-2 w-2 rounded-full ${
              ollamaOnline === null
                ? "animate-pulse bg-slate-500"
                : ollamaOnline
                  ? "bg-emerald-400"
                  : "bg-red-400"
            }`}
          />
          <span className="text-xs text-slate-500">
            {ollamaOnline === null
              ? "Checking..."
              : ollamaOnline
                ? "Ollama"
                : "Offline"}
          </span>
        </div>
      </nav>

      {/* Content */}
      <main className="min-h-0 flex-1">
        <Outlet />
      </main>
    </div>
  );
}
