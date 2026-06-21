import { getRepos } from "@/lib/api-client";
import RepoList from "@/features/repos/RepoList";
import type { RepoSummary } from "@/types/api";

/**
 * Dashboard — server component. Lists the user's repositories.
 *
 * TODO: fetch via the server-side Axum client once the backend exists. For now
 * the BFF proxy is stubbed, so we render an empty list rather than calling out.
 */
export default async function DashboardPage() {
  const repos: RepoSummary[] = [];
  void getRepos; // wired through lib/api-client from client components.

  return (
    <main className="mx-auto w-full max-w-4xl px-6 py-12">
      <h1 className="mb-8 text-2xl font-semibold tracking-tight text-ink">
        Your repositories
      </h1>
      <RepoList repos={repos} />
    </main>
  );
}
