import Link from "next/link";
import RepoStatusBadge from "@/features/repos/RepoStatusBadge";
import IndexButton from "@/features/repos/IndexButton";
import type { IndexStatus } from "@/types/api";

/**
 * Repo detail — server component. SSRs the initial status; an interactive
 * client poller (TODO) refreshes it while non-terminal. `params` is async in
 * Next 16.
 */
export default async function RepoDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;

  // TODO: SSR initial status via the server-side Axum client.
  const status: IndexStatus = "never_indexed";

  return (
    <main className="mx-auto w-full max-w-4xl px-6 py-12">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <h1 className="text-2xl font-semibold tracking-tight text-ink">
            Repository
          </h1>
          <RepoStatusBadge status={status} />
        </div>
        <div className="flex items-center gap-3">
          <IndexButton repoId={id} />
          <Link
            href={`/repos/${id}/chat`}
            className="rounded-full border border-white/10 px-4 py-2 text-sm text-ink hover:border-accent/40"
          >
            Chat
          </Link>
        </div>
      </div>
    </main>
  );
}
