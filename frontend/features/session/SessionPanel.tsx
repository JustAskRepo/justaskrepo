"use client";

import Avatar from "@/features/session/Avatar";
import IndexHalo from "@/features/session/IndexHalo";
import { accountSubtitle, accountTitle } from "@/features/session/identity";
import { STATUS_STYLE, tallyByStatus } from "@/features/repos/status";
import { describeError } from "@/lib/errors";
import { githubAppInstallUrl, logoutUrl } from "@/lib/api-client";
import type { Me, RepoSummary } from "@/types/api";

/**
 * Not a dropdown menu — a session manifest. It reads like output from the tool
 * it belongs to: who the session is, then the workspace the halo is drawing,
 * then the two things you can actually do. The status rows double as the ring's
 * legend, which is what keeps the ring from communicating by colour alone.
 *
 * The surface is opaque on purpose. `.glass` leans on `backdrop-filter`, and a
 * panel nested inside the blurred header cannot sample the page beneath it —
 * it would render as a 4%-white sheet with the dashboard showing straight
 * through. Floating overlays get a real background.
 */

function SectionLabel({ children, trailing }: { children: string; trailing?: React.ReactNode }) {
  return (
    <div className="mb-2.5 flex items-baseline justify-between">
      <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">{children}</span>
      {trailing}
    </div>
  );
}

function ActionRow({
  onClick,
  icon,
  label,
  tone = "default",
}: {
  onClick: () => void;
  icon: React.ReactNode;
  label: string;
  tone?: "default" | "danger";
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`group flex w-full items-center gap-3 rounded-lg px-2.5 py-2 text-left text-[13px] font-medium transition-colors
        focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-2 ${
          tone === "danger"
            ? "text-muted hover:bg-danger/10 hover:text-danger"
            : "text-muted hover:bg-white/[0.06] hover:text-ink"
        }`}
    >
      <span className="opacity-70 transition-opacity group-hover:opacity-100">{icon}</span>
      {label}
    </button>
  );
}

function Workspace({
  repos,
  reposError,
}: {
  repos: readonly RepoSummary[] | null;
  reposError: Error | null;
}) {
  if (repos === null) {
    if (reposError) {
      return (
        <p className="text-[13px] leading-relaxed text-warn/90">
          {describeError(reposError).inline}
        </p>
      );
    }
    return (
      <div className="space-y-2" aria-hidden="true">
        {[0, 1, 2].map((row) => (
          <div key={row} className="skeleton h-3 rounded-full" style={{ width: `${88 - row * 18}%` }} />
        ))}
      </div>
    );
  }

  if (repos.length === 0) {
    return (
      <p className="text-[13px] leading-relaxed text-muted">
        No repositories connected yet — install the GitHub App to add some.
      </p>
    );
  }

  const total = repos.length;
  return (
    <ul className="space-y-2">
      {tallyByStatus(repos).map((entry) => {
        const style = STATUS_STYLE[entry.status];
        return (
          <li key={entry.status} className="flex items-center gap-3">
            <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${style.dot}`} aria-hidden="true" />
            <span className="w-[4.75rem] shrink-0 truncate text-[13px] text-muted">{style.label}</span>
            <span className="relative h-1.5 flex-1 overflow-hidden rounded-full bg-white/[0.07]">
              <span
                className={`absolute inset-y-0 left-0 rounded-full ${style.dot}`}
                style={{ width: `${(entry.count / total) * 100}%` }}
              />
            </span>
            <span className="w-4 shrink-0 text-right font-mono text-[12px] tabular-nums text-ink">
              {entry.count}
            </span>
          </li>
        );
      })}
    </ul>
  );
}

export default function SessionPanel({
  me,
  repos,
  reposError = null,
}: {
  me: Me;
  repos: readonly RepoSummary[] | null;
  reposError?: Error | null;
}) {
  const total = repos?.length ?? null;

  return (
    <div className="panel-in w-[20rem] overflow-hidden rounded-2xl border border-white/12 bg-bg-soft shadow-[0_30px_70px_-20px_rgba(0,0,0,0.95)]">
      {/* ---------- identity ---------- */}
      <div className="flex items-center gap-3 bg-linear-to-b from-white/[0.07] to-transparent px-4 py-4">
        <IndexHalo repos={repos} size={48}>
          <Avatar me={me} size={48} />
        </IndexHalo>
        <div className="min-w-0">
          <p className="truncate text-[15px] font-semibold tracking-tight text-ink">
            {accountTitle(me)}
          </p>
          <p className="mt-0.5 truncate font-mono text-[11.5px] text-faint">{accountSubtitle(me)}</p>
        </div>
      </div>

      {/* ---------- workspace / halo legend ---------- */}
      <div className="border-t border-white/8 px-4 py-3.5">
        <SectionLabel
          trailing={
            total !== null && (
              <span className="font-mono text-[10px] text-faint">
                {total} repo{total === 1 ? "" : "s"}
              </span>
            )
          }
        >
          workspace
        </SectionLabel>
        <Workspace repos={repos} reposError={reposError} />
      </div>

      {/* ---------- actions ---------- */}
      <div className="border-t border-white/8 px-2.5 py-2.5">
        <ActionRow
          onClick={() => {
            window.location.href = githubAppInstallUrl();
          }}
          label="Add repositories"
          icon={
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="h-4 w-4 transition-transform duration-300 group-hover:rotate-90" aria-hidden="true">
              <path d="M12 5v14M5 12h14" />
            </svg>
          }
        />
        <ActionRow
          tone="danger"
          onClick={() => {
            window.location.href = logoutUrl();
          }}
          label="Sign out"
          icon={
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="h-4 w-4" aria-hidden="true">
              <path d="M15 17l5-5-5-5M20 12H9M12 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h6" />
            </svg>
          }
        />
      </div>
    </div>
  );
}
