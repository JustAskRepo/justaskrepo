import Link from "next/link";
import SpotlightCard from "@/components/spotlight-card";
import RepoStatusBadge from "@/features/repos/RepoStatusBadge";
import IndexButton from "@/features/repos/IndexButton";
import { isActive } from "@/features/repos/status";
import { shortSha, timeAgo } from "@/lib/format";
import type { RepoSummary } from "@/types/api";

type IconProps = { className?: string };

function IconLock({ className }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" className={className} aria-hidden="true">
      <rect x="4" y="10" width="16" height="11" rx="2" />
      <path d="M8 10V7a4 4 0 0 1 8 0v3" />
    </svg>
  );
}
function IconGlobe({ className }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" className={className} aria-hidden="true">
      <circle cx="12" cy="12" r="9" />
      <path d="M3 12h18M12 3a15 15 0 0 1 0 18 15 15 0 0 1 0-18Z" />
    </svg>
  );
}
function IconBranch({ className }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" className={className} aria-hidden="true">
      <circle cx="6" cy="5" r="2.5" />
      <circle cx="6" cy="19" r="2.5" />
      <circle cx="18" cy="9" r="2.5" />
      <path d="M6 7.5v9M18 11.5a5 5 0 0 1-5 5H6" />
    </svg>
  );
}
function IconCommit({ className }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" className={className} aria-hidden="true">
      <circle cx="12" cy="12" r="3.5" />
      <path d="M2 12h6.5M15.5 12H22" />
    </svg>
  );
}

/** Two-letter monogram from the repo name, e.g. "just-ask-repo" -> "JA". */
function monogram(name: string): string {
  const parts = name.split(/[^a-zA-Z0-9]+/).filter(Boolean);
  const letters = parts.length > 1 ? parts[0][0] + parts[1][0] : name.slice(0, 2);
  return letters.toUpperCase();
}

/**
 * A single repository on the dashboard. The card body is a stretched link into
 * the repo detail page; the ask link and index button sit above it on their own
 * layer so they stay independently clickable.
 */
export default function RepoCard({
  repo,
  index = 0,
  onQueued,
}: {
  repo: RepoSummary;
  /** Position in the grid — drives the entrance stagger. */
  index?: number;
  onQueued?: () => void;
}) {
  const busy = isActive(repo.current_index_status);

  return (
    // The entrance animation sits on a wrapper: a filled animation outranks
    // `:hover` in the cascade, so animating the card itself would freeze out
    // `.card-glow`'s hover lift once the entrance finished.
    <div className="pop h-full" style={{ animationDelay: `${Math.min(index, 11) * 45}ms` }}>
      <SpotlightCard className="group relative flex h-full flex-col rounded-2xl p-5">
        <div className="flex items-start gap-3">
          <span
            aria-hidden="true"
            className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-linear-to-br from-accent/30 to-accent-2/25 font-mono text-xs font-bold text-ink ring-1 ring-white/10"
          >
            {monogram(repo.name)}
          </span>

          <div className="min-w-0 flex-1">
            <h3 className="truncate text-[15px] font-semibold text-ink">
              <Link
                href={`/repos?id=${repo.repository_id}`}
                className="rounded-sm after:absolute after:inset-0 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-2"
              >
                <span className="text-muted">{repo.owner_login}/</span>
                {repo.name}
              </Link>
            </h3>
            <p className="mt-1 flex items-center gap-1.5 text-xs text-muted">
              {repo.is_private ? <IconLock className="h-3.5 w-3.5" /> : <IconGlobe className="h-3.5 w-3.5" />}
              {repo.is_private ? "Private" : "Public"}
            </p>
          </div>

          <RepoStatusBadge status={repo.current_index_status} />
        </div>

        <dl className="mt-5 grid grid-cols-2 gap-3 text-xs">
          <div className="min-w-0">
            <dt className="flex items-center gap-1.5 text-muted">
              <IconBranch className="h-3.5 w-3.5" />
              Branch
            </dt>
            <dd className="mt-1 truncate font-mono text-ink">{repo.default_branch}</dd>
          </div>
          <div className="min-w-0">
            <dt className="flex items-center gap-1.5 text-muted">
              <IconCommit className="h-3.5 w-3.5" />
              Commit
            </dt>
            <dd className="mt-1 truncate font-mono text-ink">{shortSha(repo.last_indexed_commit_sha)}</dd>
          </div>
        </dl>

        {busy ? (
          <div className="mt-5">
            <p className="text-xs text-muted">
              {repo.current_index_status === "indexing"
                ? "Chunking and embedding…"
                : "Waiting for a worker…"}
            </p>
            <div className="progress-track mt-2 h-1" role="progressbar" aria-label="Indexing in progress" />
          </div>
        ) : (
          <p className="mt-5 text-xs text-muted">
            Last indexed <span className="text-ink">{timeAgo(repo.last_indexed_at)}</span>
          </p>
        )}

        <div className="mt-5 flex items-center justify-between gap-2 border-t border-white/8 pt-4">
          <Link
            href={`/repos/chat?id=${repo.repository_id}`}
            className="relative z-10 inline-flex items-center gap-1.5 rounded-full border border-white/12 px-3 py-1.5 text-xs font-medium text-ink
              transition-colors duration-200 hover:border-accent-2/50 hover:text-accent-2
              focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-2"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" className="h-3.5 w-3.5" aria-hidden="true">
              <path d="M21 12a8 8 0 0 1-8 8H7l-4 3V12a8 8 0 0 1 8-8h2a8 8 0 0 1 8 8Z" />
            </svg>
            Ask
          </Link>

          <IndexButton repoId={repo.repository_id} onQueued={onQueued} />
        </div>
      </SpotlightCard>
    </div>
  );
}
