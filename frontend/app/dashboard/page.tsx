"use client";

import { useEffect, useState } from "react";
import { getRepos } from "@/lib/api-client";
import RepoList from "@/features/repos/RepoList";
import type { RepoSummary } from "@/types/api";

/**
 * Dashboard — client component. Static export has no server, so the repo list
 * is fetched in the browser directly from Axum (with the session cookie).
 *
 * TanStack Query is the intended home for this fetch/cache once added; plain
 * fetch keeps the scaffold dependency-free for now.
 */
export default function DashboardPage() {
  const [repos, setRepos] = useState<RepoSummary[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let active = true;
    getRepos()
      .then((data) => active && setRepos(data))
      .catch((e: unknown) => active && setError(e instanceof Error ? e.message : "Failed to load"))
      .finally(() => active && setLoading(false));
    return () => {
      active = false;
    };
  }, []);

  return (
    <main className="mx-auto w-full max-w-4xl px-6 py-12">
      <h1 className="mb-8 text-2xl font-semibold tracking-tight text-ink">
        Your repositories
      </h1>
      {loading ? (
        <p className="text-sm text-muted">Loading…</p>
      ) : error ? (
        <p className="text-sm text-red-400" role="alert">{error}</p>
      ) : (
        <RepoList repos={repos} />
      )}
    </main>
  );
}
