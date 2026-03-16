import { useState } from "react";

interface StarRatingProps {
  value: number | null;
  onChange: (score: number) => void;
  size?: "sm" | "md";
}

export function StarRating({ value, onChange, size = "md" }: StarRatingProps) {
  const [hovered, setHovered] = useState<number | null>(null);

  const sizeClass = size === "sm" ? "text-sm" : "text-base";
  const display = hovered ?? value;

  return (
    <div className={`flex items-center gap-0.5 ${sizeClass}`}>
      {[1, 2, 3, 4, 5].map((star) => (
        <button
          key={star}
          type="button"
          onClick={() => onChange(star)}
          onMouseEnter={() => setHovered(star)}
          onMouseLeave={() => setHovered(null)}
          className={`leading-none transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gold-500 ${
            display !== null && star <= display
              ? "text-amber-500"
              : "text-slate-600"
          }`}
          title={`Score ${star}/5`}
        >
          {display !== null && star <= display ? "★" : "☆"}
        </button>
      ))}
    </div>
  );
}
