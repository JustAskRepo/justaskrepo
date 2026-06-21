"use client";

import { useState } from "react";
import { startIndexing } from "@/lib/api-client";

/**
 * Triggers (re)indexing of a repository via the BFF. Client component:
 * optimistic "queued" state, then the parent page polls GET status.
 */
export default function IndexButton({ repoId }: { repoId: string }) {
  const [pending, setPending] = useState(false);

  async function handleClick() {
    setPending(true);
    try {
      // TODO: surface the returned job_id / deduped state to the status poller.
      await startIndexing(repoId);
    } catch (err) {
      console.error("Failed to start indexing", err);
    } finally {
      setPending(false);
    }
  }

  return (
    <button
      type="button"
      onClick={handleClick}
      disabled={pending}
      className="rounded-full bg-accent px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
    >
      {pending ? "Queuing…" : "Index repo"}
    </button>
  );
}
