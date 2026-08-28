"use client";

import { useMemo, useState } from "react";
import AmbientBackground from "@/components/ambient-background";
import AppNav, { type Connection } from "@/components/app-nav";
import InstallAppButton from "@/components/install-app-button";
import DashboardSkeleton from "@/features/repos/DashboardSkeleton";
import RepoFilters, { type RepoFilterValue } from "@/features/repos/RepoFilters";
import RepoList from "@/features/repos/RepoList";
import ReposErrorPanel from "@/features/repos/ReposErrorPanel";
import RepoStats from "@/features/repos/RepoStats";
import { STATUS_ORDER } from "@/features/repos/status";
import { describeError } from "@/lib/errors";
import { useRepos } from "@/features/repos/useRepos";
import type { IndexStatus, RepoSummary } from "@/types/api";

const DEFAULT_FILTERS: RepoFilterValue = { query: "", status: "all", sort: "recent" };

function sortRepos(repos: RepoSummary[], sort: RepoFilterValue["sort"]): RepoSummary[] {
  const out = [...repos];
  if (sort === "name") {
    out.sort((a, b) => a.full_name.localeCompare(b.full_name));
  } else if (sort === "status") {
    out.sort(
      (a, b) =>
        STATUS_ORDER.indexOf(a.current_index_status) -
        STATUS_ORDER.indexOf(b.current_index_status),
    );
  } else {
    // Most recently indexed first; never-indexed repos sink to the bottom.
    out.sort((a, b) => (b.last_indexed_at ?? "").localeCompare(a.last_indexed_at ?? ""));
  }
  return out;
}

/**
 * What the nav pill reports. Only a request that never reached the server is
 * "offline"; a server that answered with a failure gets its own state, so the
 * pill agrees with the error panel underneath it.
 */
function connectionFor(hasData: boolean, error: Error | null): Connection {
  if (!error) return hasData ? "live" : "connecting";
  const { kind } = describeError(error);
  if (kind === "offline") return "offline";
  if (kind === "auth") return "signedOut";
  return "error";
}

/**
 * Dashboard — client component. Static export has no server, so the repo list
 * is fetched in the browser directly from Axum (with the session cookie), and
 * re-polled by `useRepos` while any repo is mid-index.
 *
 * Search, status and sort are pure client-side views over that one list: no
 * extra round trips, and the filter state stays local to the page.
 */
export default function DashboardPage() {
  const { repos, error, refreshing, lastUpdated, refresh } = useRepos();
  const [filters, setFilters] = useState<RepoFilterValue>(DEFAULT_FILTERS);

  const counts = useMemo(() => {
    const acc: Partial<Record<IndexStatus, number>> = {};
    for (const repo of repos ?? []) {
      acc[repo.current_index_status] = (acc[repo.current_index_status] ?? 0) + 1;
    }
    return acc;
  }, [repos]);

  const visible = useMemo(() => {
    const query = filters.query.trim().toLowerCase();
    const matched = (repos ?? []).filter(
      (repo) =>
        (filters.status === "all" || repo.current_index_status === filters.status) &&
        (query === "" || repo.full_name.toLowerCase().includes(query)),
    );
    return sortRepos(matched, filters.sort);
  }, [repos, filters]);

  const connection = connectionFor(repos !== null, error);
  const isFiltered = filters.query.trim() !== "" || filters.status !== "all";

  return (
    // `relative isolate` mirrors the landing page: it gives the fixed ambient
    // layer a stacking context to sit behind, without falling under the canvas.
    <div className="relative isolate min-h-screen">
      <AmbientBackground />
      <AppNav connection={connection} />

      <main className="mx-auto w-full max-w-7xl px-6 py-10 sm:py-14">
        {/* ---------- page header ---------- */}
        <div className="flex flex-wrap items-end justify-between gap-6">
          <div>
            <p className="rise text-sm text-muted" style={{ animationDelay: "0ms" }}>
              Welcome back
            </p>
            <h1
              className="rise mt-2 text-4xl font-semibold tracking-tight sm:text-5xl"
              style={{ animationDelay: "70ms" }}
            >
              Your <span className="text-gradient">repositories</span>
            </h1>
            <p
              className="rise mt-3 max-w-xl text-pretty text-muted"
              style={{ animationDelay: "140ms" }}
            >
              Everything you have connected, and how current each index is. Pick
              one to start asking.
            </p>
          </div>

          <div className="rise flex items-center gap-3" style={{ animationDelay: "200ms" }}>
            {lastUpdated && (
              <span className="hidden text-xs text-muted sm:inline">
                Updated{" "}
                {new Date(lastUpdated).toLocaleTimeString([], {
                  hour: "2-digit",
                  minute: "2-digit",
                })}
              </span>
            )}
            <button
              type="button"
              onClick={refresh}
              disabled={refreshing}
              className="shimmer-host inline-flex items-center gap-2 rounded-full border border-white/12 bg-white/[0.03] px-4 py-2 text-sm font-medium text-ink
                transition-colors duration-200 hover:border-accent/50 disabled:opacity-60
                focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-2"
            >
              <svg
                viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"
                aria-hidden="true"
                className={`h-4 w-4 ${refreshing ? "animate-spin" : ""}`}
              >
                <path d="M21 12a9 9 0 1 1-2.64-6.36" />
                <path d="M21 3v6h-6" />
              </svg>
              {refreshing ? "Refreshing…" : "Refresh"}
            </button>
            <InstallAppButton label="Add repositories" size="sm" variant="secondary" />
          </div>
        </div>

        {/* ---------- body ---------- */}
        <div className="mt-10">
          {repos === null && !error ? (
            <DashboardSkeleton />
          ) : repos === null ? (
            <ReposErrorPanel error={error as Error} onRetry={refresh} />
          ) : (
            <div className="space-y-10">
              {error && (
                <p
                  role="status"
                  className="flex items-center gap-2 rounded-xl border border-warn/25 bg-warn/10 px-4 py-3 text-sm text-warn"
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" className="h-4 w-4 shrink-0" aria-hidden="true">
                    <path d="M12 4.5 21 19H3l9-14.5Z" />
                    <path d="M12 10v4M12 16.8v.2" />
                  </svg>
                  {describeError(error).inline} — showing the repositories as of
                  the last successful refresh.
                </p>
              )}

              <RepoStats repos={repos} />

              {repos.length > 0 && (
                <RepoFilters
                  value={filters}
                  onChange={setFilters}
                  counts={counts}
                  total={repos.length}
                />
              )}

              <RepoList
                key={`${filters.status}-${filters.sort}`}
                repos={visible}
                filtered={isFiltered}
                onClearFilters={() => setFilters(DEFAULT_FILTERS)}
                onQueued={refresh}
              />
            </div>
          )}
        </div>
      </main>
    </div>
  );
}
