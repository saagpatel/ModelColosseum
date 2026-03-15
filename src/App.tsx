import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ModelSelector } from "./components/ModelSelector";

interface Model {
  id: number;
  name: string;
  display_name: string;
  parameter_count: number | null;
  quantization: string | null;
  family: string | null;
  elo_rating: number;
  arena_wins: number;
  arena_losses: number;
  arena_draws: number;
  total_debates: number;
}

function App() {
  const [models, setModels] = useState<Model[]>([]);
  const [ollamaOnline, setOllamaOnline] = useState<boolean | null>(null);
  const [loading, setLoading] = useState(true);
  const [selectedModelId, setSelectedModelId] = useState<number | null>(null);

  useEffect(() => {
    async function init() {
      try {
        const healthy = await invoke<boolean>("health_check");
        setOllamaOnline(healthy);

        if (healthy) {
          const modelList = await invoke<Model[]>("refresh_models");
          setModels(modelList);
          if (modelList.length > 0 && modelList[0]) {
            setSelectedModelId(modelList[0].id);
          }
        }
      } catch (err) {
        console.error("init error:", err);
        setOllamaOnline(false);
      } finally {
        setLoading(false);
      }
    }
    void init();
  }, []);

  const handleRefresh = async () => {
    setLoading(true);
    try {
      const modelList = await invoke<Model[]>("refresh_models");
      setModels(modelList);
    } catch (err) {
      console.error("refresh error:", err);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-slate-950 p-8">
      <header className="mb-12 text-center">
        <h1 className="text-5xl font-black tracking-tight text-gold-400">
          Model Colosseum
        </h1>
        <p className="mt-2 text-lg font-light text-slate-400">
          Evaluate your Ollama models through battle
        </p>
      </header>

      <div className="mx-auto max-w-2xl">
        {/* Ollama Status */}
        <div className="mb-8 flex items-center justify-between rounded-xl border border-slate-800 bg-slate-900/50 px-6 py-4">
          <div className="flex items-center gap-3">
            <div
              className={`h-3 w-3 rounded-full ${
                ollamaOnline === null
                  ? "bg-slate-500 animate-pulse"
                  : ollamaOnline
                    ? "bg-emerald-400"
                    : "bg-red-400"
              }`}
            />
            <span className="text-sm font-medium text-slate-300">
              {ollamaOnline === null
                ? "Checking Ollama..."
                : ollamaOnline
                  ? "Ollama Connected"
                  : "Ollama Not Detected"}
            </span>
          </div>
          {!ollamaOnline && ollamaOnline !== null && (
            <span className="text-xs text-slate-500">
              Start Ollama to load models
            </span>
          )}
        </div>

        {/* Model Selector */}
        {ollamaOnline && (
          <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
            <div className="mb-4 flex items-center justify-between">
              <h2 className="text-lg font-bold text-slate-200">Models</h2>
              <button
                onClick={() => void handleRefresh()}
                disabled={loading}
                className="rounded-lg bg-slate-800 px-4 py-2 text-sm font-medium text-gold-400 transition-colors hover:bg-slate-700 disabled:opacity-50"
              >
                {loading ? "Syncing..." : "Refresh"}
              </button>
            </div>

            {models.length > 0 ? (
              <ModelSelector
                models={models}
                selectedId={selectedModelId}
                onSelect={setSelectedModelId}
              />
            ) : loading ? (
              <div className="py-8 text-center text-slate-500">
                Loading models...
              </div>
            ) : (
              <div className="py-8 text-center text-slate-500">
                No models found. Pull some models with{" "}
                <code className="rounded bg-slate-800 px-2 py-1 text-gold-400">
                  ollama pull
                </code>
              </div>
            )}

            {selectedModelId && models.length > 0 && (
              <div className="mt-6 rounded-lg border border-slate-700 bg-slate-800/50 p-4">
                {(() => {
                  const m = models.find((m) => m.id === selectedModelId);
                  if (!m) return null;
                  return (
                    <div className="grid grid-cols-2 gap-4 text-sm">
                      <div>
                        <span className="text-slate-500">Elo Rating</span>
                        <p className="text-xl font-bold text-gold-400">
                          {m.elo_rating.toFixed(0)}
                        </p>
                      </div>
                      <div>
                        <span className="text-slate-500">Record</span>
                        <p className="font-medium text-slate-200">
                          {m.arena_wins}W / {m.arena_losses}L / {m.arena_draws}D
                        </p>
                      </div>
                      {m.parameter_count && (
                        <div>
                          <span className="text-slate-500">Parameters</span>
                          <p className="font-medium text-slate-200">
                            {m.parameter_count}B
                          </p>
                        </div>
                      )}
                      {m.quantization && (
                        <div>
                          <span className="text-slate-500">Quantization</span>
                          <p className="font-medium text-slate-200">
                            {m.quantization}
                          </p>
                        </div>
                      )}
                    </div>
                  );
                })()}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
