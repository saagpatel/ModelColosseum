import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { DebateSummary, RoundTranscript, Model } from "../types";

export function History() {
  const [debates, setDebates] = useState<DebateSummary[]>([]);
  const [models, setModels] = useState<Model[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [filterModelId, setFilterModelId] = useState<number | null>(null);
  const [expandedId, setExpandedId] = useState<number | null>(null);
  const [transcript, setTranscript] = useState<RoundTranscript[]>([]);
  const [loadingTranscript, setLoadingTranscript] = useState(false);
  const [hasMore, setHasMore] = useState(true);
  const [searchInput, setSearchInput] = useState("");

  useEffect(() => {
    const timer = setTimeout(() => setSearch(searchInput), 300);
    return () => clearTimeout(timer);
  }, [searchInput]);

  const fetchDebates = useCallback(async (cursor?: number) => {
    setLoading(!cursor);
    try {
      const params: Record<string, unknown> = { limit: 20 };
      if (cursor) params.cursor = cursor;
      if (search) params.search = search;
      if (filterModelId) params.modelId = filterModelId;
      const data = await invoke<DebateSummary[]>("get_debates", params);
      if (cursor) {
        setDebates(prev => [...prev, ...data]);
      } else {
        setDebates(data);
      }
      setHasMore(data.length === 20);
    } catch (err) {
      console.error("Failed to fetch debates:", err);
    } finally {
      setLoading(false);
    }
  }, [search, filterModelId]);

  useEffect(() => { void fetchDebates(); }, [fetchDebates]);

  useEffect(() => {
    invoke<Model[]>("list_models").then(setModels).catch(console.error);
  }, []);

  const handleExpand = async (debateId: number) => {
    if (expandedId === debateId) {
      setExpandedId(null);
      return;
    }
    setExpandedId(debateId);
    setLoadingTranscript(true);
    try {
      const data = await invoke<RoundTranscript[]>("get_debate_transcript", { debateId });
      setTranscript(data);
    } catch (err) {
      console.error("Failed to fetch transcript:", err);
    } finally {
      setLoadingTranscript(false);
    }
  };

  const handleLoadMore = () => {
    const last = debates[debates.length - 1];
    if (last) void fetchDebates(last.id);
  };

  const outcomeBadge = (d: DebateSummary) => {
    if (d.status === "abandoned") return <span className="rounded bg-slate-800 px-1.5 py-0.5 text-[10px] text-slate-500">Abandoned</span>;
    if (d.status === "voting") return <span className="rounded bg-gold-500/10 px-1.5 py-0.5 text-[10px] text-gold-400">Awaiting Vote</span>;
    if (d.winner === "draw") return <span className="rounded bg-slate-700 px-1.5 py-0.5 text-[10px] text-slate-300">Draw</span>;
    if (d.winner === "model_a") return <span className="rounded bg-emerald-500/10 px-1.5 py-0.5 text-[10px] text-emerald-400">{d.model_a_name} won</span>;
    if (d.winner === "model_b") return <span className="rounded bg-red-500/10 px-1.5 py-0.5 text-[10px] text-red-400">{d.model_b_name} won</span>;
    return null;
  };

  if (loading && debates.length === 0) {
    return (
      <div className="flex h-full items-center justify-center">
        <span className="animate-pulse text-sm text-slate-500">Loading history...</span>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col gap-6 p-6">
      <h1 className="text-xl font-black text-slate-100">Debate History</h1>

      {/* Filters */}
      <div className="flex items-center gap-4">
        <input
          type="text"
          placeholder="Search topics..."
          value={searchInput}
          onChange={(e) => setSearchInput(e.target.value)}
          className="rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-slate-200 placeholder-slate-500 outline-none focus:border-gold-500/50"
        />
        <select
          value={filterModelId ?? ""}
          onChange={(e) => setFilterModelId(e.target.value ? Number(e.target.value) : null)}
          className="rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-slate-200 outline-none focus:border-gold-500/50"
        >
          <option value="">All Models</option>
          {models.map(m => (
            <option key={m.id} value={m.id}>{m.display_name}</option>
          ))}
        </select>
      </div>

      {debates.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-3">
          <div className="text-4xl">📜</div>
          <p className="text-sm font-medium text-slate-300">No debates found</p>
          <p className="text-xs text-slate-500">
            {search || filterModelId ? "Try adjusting your filters" : "Completed debates will appear here"}
          </p>
        </div>
      ) : (
        <div className="min-h-0 flex-1 overflow-auto">
          <div className="flex flex-col gap-2">
            {debates.map(d => (
              <div key={d.id} className="rounded-xl border border-slate-800 bg-slate-900/50">
                <button
                  onClick={() => void handleExpand(d.id)}
                  className="flex w-full items-center justify-between px-5 py-3 text-left transition-colors hover:bg-slate-800/30"
                >
                  <div className="flex min-w-0 flex-1 items-center gap-4">
                    <span className="shrink-0 text-sm font-semibold text-slate-200 truncate max-w-[300px]">
                      {d.topic}
                    </span>
                    <span className="shrink-0 text-xs text-slate-500">
                      {d.model_a_name} vs {d.model_b_name}
                    </span>
                    {outcomeBadge(d)}
                  </div>
                  <div className="flex items-center gap-3">
                    <span className="text-xs text-slate-600">{d.total_rounds} rounds</span>
                    <span className="text-xs text-slate-600">
                      {new Date(d.created_at + "Z").toLocaleDateString()}
                    </span>
                    <span className={`text-xs text-slate-500 transition-transform ${expandedId === d.id ? "rotate-180" : ""}`}>
                      ▼
                    </span>
                  </div>
                </button>

                {expandedId === d.id && (
                  <div className="border-t border-slate-800 px-5 py-4">
                    {loadingTranscript ? (
                      <span className="animate-pulse text-sm text-slate-500">Loading transcript...</span>
                    ) : transcript.length === 0 ? (
                      <span className="text-sm text-slate-500">No rounds recorded</span>
                    ) : (
                      <div className="flex flex-col gap-4">
                        {transcript.map((r, i) => (
                          <div key={i} className="flex gap-3">
                            <div className="flex shrink-0 flex-col items-center gap-1">
                              <span className={`rounded px-1.5 py-0.5 text-[10px] font-bold ${
                                r.speaker === "model_a"
                                  ? "bg-emerald-500/20 text-emerald-400"
                                  : "bg-red-500/20 text-red-400"
                              }`}>
                                {r.speaker === "model_a" ? "PRO" : "CON"}
                              </span>
                              <span className="text-[10px] text-slate-600">R{r.round_number}</span>
                            </div>
                            <p className="min-w-0 flex-1 whitespace-pre-wrap text-sm leading-relaxed text-slate-300">
                              {r.content}
                            </p>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                )}
              </div>
            ))}
          </div>

          {hasMore && (
            <button
              onClick={handleLoadMore}
              className="mx-auto mt-4 block rounded-lg border border-slate-700 bg-slate-800 px-4 py-2 text-xs font-medium text-slate-300 transition-colors hover:bg-slate-700"
            >
              Load More
            </button>
          )}
        </div>
      )}
    </div>
  );
}
