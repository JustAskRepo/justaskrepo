import Link from "next/link";
import InstallAppButton from "@/components/install-app-button";
import RepoCard from "@/features/repos/RepoCard";
import type { RepoSummary } from "@/types/api";

const ONBOARDING = [
  { n: "01", title: "Install the App", body: "Choose which repositories JustAskRepo may read." },
  { n: "02", title: "We index it", body: "Code is chunked with Tree-sitter, embedded, and stored as vectors." },
  { n: "03", title: "Just ask", body: "Ask anything and get answers with citations back to the source." },
];

function EmptyShell({ children }: { children: React.ReactNode }) {
  return (
    <div className="glass rise flex flex-col items-center rounded-2xl px-6 py-16 text-center">
      {children}
    </div>
  );
}

/**
 * The repo grid. Presentational — the page fetches, filters and sorts, and
 * hands the result down. Cards stagger in on mount, and remount (so they
 * re-stagger) whenever the filtered set changes.
 */
export default function RepoList({
  repos,
  filtered = false,
  onClearFilters,
  onQueued,
}: {
  repos: RepoSummary[];
  /** True when filters are narrowing the list — changes the empty copy. */
  filtered?: boolean;
  onClearFilters?: () => void;
  onQueued?: () => void;
}) {
  if (repos.length === 0 && filtered) {
    return (
      <EmptyShell>
        <span className="flex h-12 w-12 items-center justify-center rounded-2xl bg-white/5 text-muted ring-1 ring-white/10">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" className="h-5 w-5" aria-hidden="true">
            <circle cx="11" cy="11" r="7" />
            <path d="m21 21-4.3-4.3M8.5 11h5" />
          </svg>
        </span>
        <h3 className="mt-5 text-base font-semibold text-ink">No repositories match</h3>
        <p className="mt-2 max-w-sm text-sm text-muted">
          Nothing here fits the current search and status filter.
        </p>
        {onClearFilters && (
          <button
            type="button"
            onClick={onClearFilters}
            className="mt-6 rounded-full border border-white/12 px-4 py-2 text-sm text-ink transition-colors hover:border-accent/50
              focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-2"
          >
            Clear filters
          </button>
        )}
      </EmptyShell>
    );
  }

  if (repos.length === 0) {
    return (
      <EmptyShell>
        <span className="relative flex h-14 w-14 items-center justify-center rounded-2xl bg-linear-to-br from-accent/25 to-accent-2/20 text-accent-2 ring-1 ring-white/10">
          <span className="absolute inset-0 animate-ping rounded-2xl bg-accent/10" />
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" className="relative h-6 w-6" aria-hidden="true">
            <path d="M4 19.5V5a2 2 0 0 1 2-2h13v18H6.5A2.5 2.5 0 0 1 4 19.5Z" />
            <path d="M12 8v6M9 11h6" />
          </svg>
        </span>

        <h3 className="mt-6 text-lg font-semibold text-ink">Connect your first repository</h3>
        <p className="mt-2 max-w-md text-sm leading-relaxed text-muted">
          Install the JustAskRepo GitHub App and pick the repositories it can
          read. Indexing starts on its own, and they appear here as they finish.
        </p>

        <div className="mt-7 flex flex-col items-center gap-4 sm:flex-row">
          <InstallAppButton />
          <Link
            href="/#how"
            className="group inline-flex items-center gap-1.5 rounded-full px-5 py-3 text-sm font-medium text-muted transition-colors hover:text-ink"
          >
            See how it works
            <span className="transition-transform duration-300 group-hover:translate-x-0.5">→</span>
          </Link>
        </div>

        <ol className="mt-12 grid w-full max-w-3xl gap-3 text-left sm:grid-cols-3">
          {ONBOARDING.map((step, i) => (
            <li
              key={step.n}
              className="pop rounded-xl border border-white/8 bg-white/[0.02] p-4"
              style={{ animationDelay: `${120 + i * 90}ms` }}
            >
              <span className="font-mono text-xs text-accent-2">{step.n}</span>
              <h4 className="mt-2 text-sm font-semibold text-ink">{step.title}</h4>
              <p className="mt-1 text-xs leading-relaxed text-muted">{step.body}</p>
            </li>
          ))}
        </ol>
      </EmptyShell>
    );
  }

  return (
    <ul className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
      {repos.map((repo, i) => (
        <li key={repo.repository_id} className="h-full">
          <RepoCard repo={repo} index={i} onQueued={onQueued} />
        </li>
      ))}
    </ul>
  );
}
