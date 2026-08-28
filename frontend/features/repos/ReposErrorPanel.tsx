"use client";

import GitHubButton from "@/components/github-button";
import { describeError } from "@/lib/errors";

type IconProps = { className?: string };

function IconLock({ className }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" className={className} aria-hidden="true">
      <rect x="4" y="10" width="16" height="11" rx="2" />
      <path d="M8 10V7a4 4 0 0 1 8 0v3" />
    </svg>
  );
}
function IconOffline({ className }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" className={className} aria-hidden="true">
      <path d="M3 8.5a15 15 0 0 1 18 0M6.5 12.5a10 10 0 0 1 11 0M10 16.5a5 5 0 0 1 4 0" />
      <path d="M12 20v.2M4 4l16 16" />
    </svg>
  );
}
function IconAlert({ className }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" className={className} aria-hidden="true">
      <path d="M12 4.5 21 19H3l9-14.5Z" />
      <path d="M12 10v4M12 16.8v.2" />
    </svg>
  );
}

/**
 * Full-screen failure state for the repo list. Leads with plain language and
 * the one action worth taking; the raw transport error stays available, folded
 * away, for whoever is debugging.
 */
export default function ReposErrorPanel({
  error,
  onRetry,
}: {
  error: Error;
  onRetry: () => void;
}) {
  const friendly = describeError(error);
  const isAuth = friendly.kind === "auth";
  const isOffline = friendly.kind === "offline";

  const Icon = isAuth ? IconLock : isOffline ? IconOffline : IconAlert;
  const tone = isAuth ? "bg-accent/15 text-accent" : "bg-danger/15 text-danger";

  return (
    <div role="alert" className="glass rise relative overflow-hidden rounded-2xl px-6 py-16 text-center">
      <div className="orb animate-float h-64 w-64 left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 bg-danger/15" />

      <div className="relative flex flex-col items-center">
        <span className={`flex h-14 w-14 items-center justify-center rounded-2xl ring-1 ring-white/10 ${tone}`}>
          <Icon className="h-6 w-6" />
        </span>

        <h2 className="mt-6 text-lg font-semibold text-ink">{friendly.title}</h2>

        <p className="mt-2 max-w-md text-sm leading-relaxed text-muted">{friendly.body}</p>

        <div className="mt-7 flex flex-wrap items-center justify-center gap-3">
          {isAuth ? (
            <GitHubButton size="sm" />
          ) : (
            <button
              type="button"
              onClick={onRetry}
              className="inline-flex items-center gap-2 rounded-full border border-white/12 px-4 py-2 text-sm font-medium text-ink
                transition-colors hover:border-accent/50
                focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-2"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="h-4 w-4" aria-hidden="true">
                <path d="M21 12a9 9 0 1 1-2.64-6.36" />
                <path d="M21 3v6h-6" />
              </svg>
              Try again
            </button>
          )}
        </div>

        <details className="group mt-7 text-left">
          <summary className="cursor-pointer list-none text-center text-xs text-muted transition-colors hover:text-ink focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-2">
            <span className="inline-flex items-center gap-1.5">
              Technical details
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="h-3 w-3 transition-transform duration-200 group-open:rotate-180" aria-hidden="true">
                <path d="m6 9 6 6 6-6" />
              </svg>
            </span>
          </summary>
          <code className="mt-3 block rounded-md border border-white/8 bg-black/30 px-3 py-2 font-mono text-[11px] text-muted">
            {friendly.detail}
          </code>
        </details>
      </div>
    </div>
  );
}
