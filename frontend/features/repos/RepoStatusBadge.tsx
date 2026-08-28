import type { IndexStatus } from "@/types/api";
import { STATUS_STYLE } from "@/features/repos/status";

/**
 * Status pill for a repository's index state — a coloured marker plus the
 * label, so the state reads without relying on colour perception. Active
 * states (queued / indexing) get a live pulse.
 */
export default function RepoStatusBadge({ status }: { status: IndexStatus }) {
  const style = STATUS_STYLE[status];

  return (
    <span
      className={`inline-flex shrink-0 items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs font-medium ${style.shell} ${style.text}`}
    >
      <span className="relative flex h-1.5 w-1.5">
        {style.active && (
          <span className={`absolute inline-flex h-full w-full animate-ping rounded-full opacity-70 ${style.dot}`} />
        )}
        <span className={`relative inline-flex h-1.5 w-1.5 rounded-full ${style.dot}`} />
      </span>
      {style.label}
    </span>
  );
}
