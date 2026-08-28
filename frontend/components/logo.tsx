/**
 * Brand mark. Presentational and feature-agnostic — shared by the marketing
 * landing page and the app chrome so the identity is defined exactly once.
 */
export default function Logo({ className = "" }: { className?: string }) {
  return (
    <span className={`flex items-center gap-2.5 ${className}`}>
      <span className="relative flex h-8 w-8 items-center justify-center rounded-lg bg-linear-to-br from-accent to-accent-2 font-mono text-sm font-bold text-white shadow-[0_8px_24px_-8px_rgba(139,92,246,0.8)]">
        {"{ }"}
      </span>
      <span className="text-[15px] font-semibold tracking-tight text-ink">
        JustAsk<span className="text-muted">Repo</span>
      </span>
    </span>
  );
}
