import type { IndexStatus } from "@/types/api";

/**
 * One place that decides how every index state looks and reads. Colour is
 * always paired with a label here — state is never communicated by hue alone.
 * Class names are written out in full so Tailwind's scanner can see them.
 */
export interface StatusStyle {
  label: string;
  /** Marker fill. */
  dot: string;
  /** Stroke for this status' arc in the account halo. */
  arc: string;
  /** Foreground for the label. */
  text: string;
  /** Border + tint for the surrounding pill. */
  shell: string;
  /** Work is in flight — the dashboard animates and polls while any repo is here. */
  active: boolean;
}

export const STATUS_STYLE: Record<IndexStatus, StatusStyle> = {
  indexing: {
    label: "Indexing",
    dot: "bg-accent-2",
    arc: "stroke-accent-2",
    text: "text-accent-2",
    shell: "border-accent-2/30 bg-accent-2/10",
    active: true,
  },
  queued: {
    label: "Queued",
    dot: "bg-pending",
    arc: "stroke-pending",
    text: "text-pending",
    shell: "border-pending/30 bg-pending/10",
    active: true,
  },
  indexed: {
    label: "Indexed",
    dot: "bg-ok",
    arc: "stroke-ok",
    text: "text-ok",
    shell: "border-ok/30 bg-ok/10",
    active: false,
  },
  stale: {
    label: "Stale",
    dot: "bg-warn",
    arc: "stroke-warn",
    text: "text-warn",
    shell: "border-warn/30 bg-warn/10",
    active: false,
  },
  failed: {
    label: "Failed",
    dot: "bg-danger",
    arc: "stroke-danger",
    text: "text-danger",
    shell: "border-danger/30 bg-danger/10",
    active: false,
  },
  never_indexed: {
    label: "Never indexed",
    dot: "bg-muted",
    arc: "stroke-muted",
    text: "text-muted",
    shell: "border-white/10 bg-white/5",
    active: false,
  },
};

/** Display order for filters and grouping — most urgent first. */
export const STATUS_ORDER: readonly IndexStatus[] = [
  "indexing",
  "queued",
  "indexed",
  "stale",
  "failed",
  "never_indexed",
] as const;

export function isActive(status: IndexStatus): boolean {
  return STATUS_STYLE[status].active;
}

/** Indexed once, but no longer trustworthy — the user should re-run indexing. */
export function needsAttention(status: IndexStatus): boolean {
  return status === "failed" || status === "stale";
}

export interface StatusTally {
  status: IndexStatus;
  count: number;
}

/**
 * Repo counts per status in display order, with empty statuses dropped. Shared
 * by anything that needs to summarise a whole workspace at once — the account
 * halo and its legend read from this so the ring and the numbers can never
 * disagree.
 */
export function tallyByStatus(
  repos: readonly { current_index_status: IndexStatus }[],
): StatusTally[] {
  const counts = new Map<IndexStatus, number>();
  for (const repo of repos) {
    counts.set(repo.current_index_status, (counts.get(repo.current_index_status) ?? 0) + 1);
  }
  return STATUS_ORDER.map((status) => ({ status, count: counts.get(status) ?? 0 })).filter(
    (tally) => tally.count > 0,
  );
}
