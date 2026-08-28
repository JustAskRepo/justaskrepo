"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { getRepos } from "@/lib/api-client";
import { isActive } from "@/features/repos/status";
import type { RepoSummary } from "@/types/api";

/** How often to re-poll while at least one repo is queued or indexing. */
const POLL_MS = 5000;

export interface ReposState {
  repos: RepoSummary[] | null;
  error: Error | null;
  /** A user-triggered refresh is in flight (drives the button spinner). */
  refreshing: boolean;
  /** Epoch ms of the last successful load. */
  lastUpdated: number | null;
  refresh: () => void;
}

/**
 * Loads the repo list from Axum in the browser (static export — no server to
 * fetch on). While any repo is mid-index the list re-polls on its own, so
 * progress lands without the user reaching for reload.
 *
 * A failed poll keeps the last good list and only sets `error`; the dashboard
 * shows a full error screen just when there is nothing to show yet.
 */
export function useRepos(): ReposState {
  const [repos, setRepos] = useState<RepoSummary[] | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [lastUpdated, setLastUpdated] = useState<number | null>(null);
  const alive = useRef(true);

  // State is committed from the promise's callbacks rather than straight-line
  // in the effect body, so mounting never cascades a synchronous re-render.
  const commit = useCallback((data: RepoSummary[]) => {
    if (!alive.current) return;
    setRepos(data);
    setError(null);
    setLastUpdated(Date.now());
  }, []);

  const fail = useCallback((err: unknown) => {
    if (!alive.current) return;
    setError(err instanceof Error ? err : new Error("Failed to load repositories"));
  }, []);

  const load = useCallback(() => getRepos().then(commit, fail), [commit, fail]);

  useEffect(() => {
    alive.current = true;
    void load();
    return () => {
      alive.current = false;
    };
  }, [load]);

  const hasActive = repos?.some((repo) => isActive(repo.current_index_status)) ?? false;

  useEffect(() => {
    if (!hasActive) return;
    const id = setInterval(() => void load(), POLL_MS);
    return () => clearInterval(id);
  }, [hasActive, load]);

  const refresh = useCallback(() => {
    setRefreshing(true);
    void load().finally(() => {
      if (alive.current) setRefreshing(false);
    });
  }, [load]);

  return { repos, error, refreshing, lastUpdated, refresh };
}
