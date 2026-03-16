export function Skeleton({ className, count = 1 }: { className?: string; count?: number }) {
  return (
    <>
      {Array.from({ length: count }, (_, i) => (
        <div key={i} className={`animate-pulse rounded bg-slate-800 ${className ?? ""}`} />
      ))}
    </>
  );
}
