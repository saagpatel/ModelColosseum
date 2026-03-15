interface Model {
  id: number;
  name: string;
  display_name: string;
  parameter_count: number | null;
  quantization: string | null;
  family: string | null;
  elo_rating: number;
}

interface ModelSelectorProps {
  models: Model[];
  selectedId: number | null;
  onSelect: (id: number) => void;
  label?: string;
}

export function ModelSelector({
  models,
  selectedId,
  onSelect,
  label,
}: ModelSelectorProps) {
  return (
    <div>
      {label && (
        <label className="mb-2 block text-sm font-medium text-slate-400">
          {label}
        </label>
      )}
      <div className="space-y-2">
        {models.map((model) => (
          <button
            key={model.id}
            onClick={() => onSelect(model.id)}
            className={`flex w-full items-center justify-between rounded-lg border px-4 py-3 text-left transition-all ${
              selectedId === model.id
                ? "border-gold-500/50 bg-gold-500/10 text-gold-300"
                : "border-slate-700 bg-slate-800/50 text-slate-300 hover:border-slate-600 hover:bg-slate-800"
            }`}
          >
            <div className="flex items-center gap-3">
              <div
                className={`h-2 w-2 rounded-full ${
                  selectedId === model.id ? "bg-gold-400" : "bg-slate-600"
                }`}
              />
              <div>
                <span className="font-semibold">{model.display_name}</span>
                <span className="ml-2 text-xs text-slate-500">{model.name}</span>
              </div>
            </div>
            <div className="flex items-center gap-4 text-xs">
              {model.parameter_count && (
                <span className="rounded bg-slate-700 px-2 py-0.5 text-slate-400">
                  {model.parameter_count}B
                </span>
              )}
              {model.quantization && (
                <span className="rounded bg-slate-700 px-2 py-0.5 text-slate-400">
                  {model.quantization}
                </span>
              )}
              <span className="font-mono text-gold-500">
                {model.elo_rating.toFixed(0)}
              </span>
            </div>
          </button>
        ))}
      </div>
    </div>
  );
}
