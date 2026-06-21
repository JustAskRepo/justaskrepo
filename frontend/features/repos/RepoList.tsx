import Link from "next/link";
import type { RepoSummary } from "@/types/api";
import RepoStatusBadge from "@/features/repos/RepoStatusBadge";

/**
 * Dashboard repo list. Presentational — receives repos from a server component
 * that fetched them via the BFF (lib/api-client `getRepos`).
 */
export default function RepoList({ repos }: { repos: RepoSummary[] }) {
  if (repos.length === 0) {
    return (
      <p className="text-sm text-muted">
        No repositories yet. Install the GitHub App to get started.
      </p>
    );
  }

  return (
    <ul className="flex flex-col gap-2">
      {repos.map((repo) => (
        <li key={repo.repository_id}>
          <Link
            href={`/repos/${repo.repository_id}`}
            className="flex items-center justify-between rounded-xl border border-white/8 bg-white/[0.03] px-4 py-3 transition-colors hover:border-accent/40"
          >
            <span className="text-sm font-medium text-ink">{repo.full_name}</span>
            <RepoStatusBadge status={repo.current_index_status} />
          </Link>
        </li>
      ))}
    </ul>
  );
}
