"use client";

import { Suspense, useEffect, useState } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import RepoStatusBadge from "@/features/repos/RepoStatusBadge";
import IndexButton from "@/features/repos/IndexButton";
import { getRepoStatus } from "@/lib/api-client";
import type { IndexStatus } from "@/types/api";

/**
 * Repo detail — client component. In a static export there are no server-
 * rendered dynamic segments (`/repos/[id]` would need `generateStaticParams`
 * over ids we can't know at build time), so the repo id travels as `?id=` and
 * is read client-side. `useSearchParams` must sit under <Suspense>.
 */
function RepoDetail() {
  const id = useSearchParams().get("id");
  const [status, setStatus] = useState<IndexStatus>("never_indexed");

  useEffect(() => {
    if (!id) return;
    let active = true;
    getRepoStatus(id)
      .then((s) => active && setStatus(s.current_index_status))
      .catch(() => {
        /* TODO: surface load errors; poll while non-terminal */
      });
    return () => {
      active = false;
    };
  }, [id]);

  if (!id) {
    return <p className="text-sm text-red-400" role="alert">Missing repo id.</p>;
  }

  return (
    <div className="flex items-center justify-between">
      <div className="flex items-center gap-3">
        <h1 className="text-2xl font-semibold tracking-tight text-ink">Repository</h1>
        <RepoStatusBadge status={status} />
      </div>
      <div className="flex items-center gap-3">
        <IndexButton repoId={id} />
        <Link
          href={`/repos/chat?id=${id}`}
          className="rounded-full border border-white/10 px-4 py-2 text-sm text-ink hover:border-accent/40"
        >
          Chat
        </Link>
      </div>
    </div>
  );
}

export default function RepoDetailPage() {
  return (
    <main className="mx-auto w-full max-w-4xl px-6 py-12">
      <Suspense fallback={<p className="text-sm text-muted">Loading…</p>}>
        <RepoDetail />
      </Suspense>
    </main>
  );
}
