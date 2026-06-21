import type { IndexStatus } from "@/types/api";

const LABELS: Record<IndexStatus, string> = {
  never_indexed: "Never indexed",
  queued: "Queued",
  indexing: "Indexing…",
  indexed: "Indexed",
  failed: "Failed",
  stale: "Stale",
};

/** Small status pill for a repository's current index state. */
export default function RepoStatusBadge({ status }: { status: IndexStatus }) {
  return (
    <span className="rounded-full border border-white/10 bg-white/5 px-2.5 py-0.5 text-xs text-muted">
      {LABELS[status]}
    </span>
  );
}
