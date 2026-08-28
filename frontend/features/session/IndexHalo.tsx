"use client";

import { useId } from "react";
import { STATUS_STYLE, isActive, tallyByStatus } from "@/features/repos/status";
import type { IndexStatus, RepoSummary } from "@/types/api";

/**
 * The ring around the avatar is the workspace, not decoration: one arc per
 * connected repository, coloured by that repo's index state and ordered by
 * urgency. A glance at the header answers "is my index healthy?" without
 * opening anything, and the arcs re-draw as the dashboard polls.
 *
 * Colour alone never carries the meaning — `SessionPanel` spells out the same
 * numbers as a labelled legend, and the ring exposes them to assistive tech as
 * its accessible name.
 */

/** Past this many repos an arc would be thinner than the gap beside it, so the
 *  ring switches from one-tick-per-repo to one-arc-per-status. */
const MAX_TICKS = 16;

const BOX = 52;
const CENTER = BOX / 2;
/** Data ring. */
const R = 21;
const CIRC = 2 * Math.PI * R;
const WIDTH = 3;
/** Track left showing between arcs, in path units (~3px at the header size). */
const GAP = 3.5;
/** The activity comet orbits outside the data, never on top of it. */
const R_SCAN = R + 3;

interface Arc {
  status: IndexStatus;
  length: number;
  offset: number;
}

function arcsFor(repos: readonly RepoSummary[]): Arc[] {
  const tally = tallyByStatus(repos);
  const total = repos.length;

  const ticks =
    total <= MAX_TICKS
      ? tally.flatMap((t) => Array.from({ length: t.count }, () => ({ status: t.status, weight: 1 })))
      : tally.map((t) => ({ status: t.status, weight: t.count }));

  const arcs: Arc[] = [];
  let cursor = 0;
  for (const tick of ticks) {
    const span = (tick.weight / total) * CIRC;
    arcs.push({ status: tick.status, length: Math.max(span - GAP, 1.5), offset: cursor });
    cursor += span;
  }
  return arcs;
}

export function haloSummary(repos: readonly RepoSummary[] | null): string {
  if (repos === null) return "Workspace status unavailable";
  if (repos.length === 0) return "No repositories connected";
  const parts = tallyByStatus(repos).map(
    (t) => `${t.count} ${STATUS_STYLE[t.status].label.toLowerCase()}`,
  );
  return `Index health: ${parts.join(", ")}`;
}

export default function IndexHalo({
  repos,
  size,
  children,
}: {
  repos: readonly RepoSummary[] | null;
  /** Outer diameter in px; everything inside scales from it. */
  size: number;
  children?: React.ReactNode;
}) {
  const gradientId = useId();
  const arcs = repos && repos.length > 0 ? arcsFor(repos) : [];
  const live = repos?.some((repo) => isActive(repo.current_index_status)) ?? false;
  const inner = Math.round((size * 32) / BOX);

  // With no list to draw, the ring rests on the brand gradient rather than a
  // near-invisible grey — an unknown workspace should still look deliberate.
  const resting = repos === null;

  return (
    <span
      className="relative inline-flex shrink-0 items-center justify-center"
      style={{ width: size, height: size }}
    >
      <svg
        viewBox={`0 0 ${BOX} ${BOX}`}
        className="absolute inset-0 h-full w-full -rotate-90"
        role="img"
        aria-label={haloSummary(repos)}
      >
        <defs>
          <linearGradient id={gradientId} x1="0" y1="0" x2="1" y2="1">
            <stop offset="0%" stopColor="var(--color-accent)" />
            <stop offset="100%" stopColor="var(--color-accent-2)" />
          </linearGradient>
        </defs>

        <circle
          cx={CENTER} cy={CENTER} r={R}
          fill="none" strokeWidth={WIDTH}
          stroke={resting ? `url(#${gradientId})` : undefined}
          className={resting ? "opacity-45" : "stroke-white/10"}
        />
        {arcs.map((arc, i) => (
          <circle
            key={`${arc.status}-${i}`}
            cx={CENTER} cy={CENTER} r={R}
            fill="none" strokeWidth={WIDTH} strokeLinecap="butt"
            strokeDasharray={`${arc.length} ${CIRC - arc.length}`}
            strokeDashoffset={-arc.offset}
            className={STATUS_STYLE[arc.status].arc}
          />
        ))}
        {live && (
          <circle
            cx={CENTER} cy={CENTER} r={R_SCAN}
            fill="none" strokeWidth={1.5} strokeLinecap="round"
            strokeDasharray={`${CIRC * 0.12} ${CIRC}`}
            className="animate-halo-scan stroke-accent-2 opacity-70"
            style={{ transformOrigin: "center" }}
          />
        )}
      </svg>

      {/* Rounded square, not a disc — it echoes the `{ }` tile in the logo and
          keeps the circular gauge legible as a separate object. */}
      <span
        className="relative overflow-hidden rounded-[32%] bg-white/5 ring-1 ring-white/10"
        style={{ width: inner, height: inner }}
      >
        {children}
      </span>
    </span>
  );
}
