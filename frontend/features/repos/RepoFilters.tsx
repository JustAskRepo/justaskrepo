"use client";

import { useEffect, useRef } from "react";
import { STATUS_ORDER, STATUS_STYLE } from "@/features/repos/status";
import type { IndexStatus } from "@/types/api";

export type SortKey = "recent" | "name" | "status";
export type StatusFilter = IndexStatus | "all";

export interface RepoFilterValue {
  query: string;
  status: StatusFilter;
  sort: SortKey;
}

const SORTS: { value: SortKey; label: string }[] = [
  { value: "recent", label: "Recently indexed" },
  { value: "name", label: "Name (A–Z)" },
  { value: "status", label: "Status" },
];

/**
 * Search, status chips and sort for the repo grid. Fully controlled — the page
 * owns the filter state and does the filtering, so this stays presentational.
 * Chips only appear for statuses that actually occur in the list.
 */
export default function RepoFilters({
  value,
  onChange,
  counts,
  total,
}: {
  value: RepoFilterValue;
  onChange: (next: RepoFilterValue) => void;
  counts: Partial<Record<IndexStatus, number>>;
  total: number;
}) {
  const searchRef = useRef<HTMLInputElement>(null);

  // "/" focuses search, the way most code tools behave.
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key !== "/" || event.metaKey || event.ctrlKey) return;
      const el = event.target as HTMLElement | null;
      if (el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable)) return;
      event.preventDefault();
      searchRef.current?.focus();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const chips: { key: StatusFilter; label: string; count: number; dot: string }[] = [
    { key: "all", label: "All", count: total, dot: "bg-muted" },
    ...STATUS_ORDER.filter((status) => (counts[status] ?? 0) > 0).map((status) => ({
      key: status as StatusFilter,
      label: STATUS_STYLE[status].label,
      count: counts[status] ?? 0,
      dot: STATUS_STYLE[status].dot,
    })),
  ];

  return (
    <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
      {/* search */}
      <div className="group relative w-full lg:max-w-xs">
        <svg
          viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"
          aria-hidden="true"
          className="pointer-events-none absolute left-3.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted transition-colors group-focus-within:text-accent-2"
        >
          <circle cx="11" cy="11" r="7" />
          <path d="m21 21-4.3-4.3" />
        </svg>
        <label htmlFor="repo-search" className="sr-only">
          Filter repositories by name
        </label>
        <input
          id="repo-search"
          ref={searchRef}
          type="search"
          value={value.query}
          onChange={(e) => onChange({ ...value, query: e.target.value })}
          placeholder="Filter repositories…"
          className="h-11 w-full rounded-full border border-white/10 bg-white/[0.03] pl-10 pr-12 text-sm text-ink
            placeholder:text-muted transition-colors duration-200
            hover:border-white/20
            focus:border-accent/50 focus:bg-white/[0.05] focus:outline-none focus:ring-2 focus:ring-accent/25"
        />
        {value.query.length === 0 && (
          <kbd className="pointer-events-none absolute right-3.5 top-1/2 hidden -translate-y-1/2 rounded border border-white/12 bg-white/5 px-1.5 py-0.5 font-mono text-[10px] text-muted sm:block">
            /
          </kbd>
        )}
      </div>

      <div className="flex flex-col gap-4 sm:flex-row sm:items-center">
        {/* status chips */}
        <div className="flex flex-wrap items-center gap-2" role="group" aria-label="Filter by index status">
          {chips.map((chip) => {
            const active = value.status === chip.key;
            return (
              <button
                key={chip.key}
                type="button"
                aria-pressed={active}
                onClick={() => onChange({ ...value, status: chip.key })}
                className={`inline-flex items-center gap-1.5 rounded-full border px-3 py-1.5 text-xs font-medium
                  transition-all duration-200
                  focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-2 ${
                    active
                      ? "border-accent/50 bg-accent/15 text-ink"
                      : "border-white/10 bg-white/[0.03] text-muted hover:border-white/25 hover:text-ink"
                  }`}
              >
                <span className={`h-1.5 w-1.5 rounded-full ${chip.dot}`} aria-hidden="true" />
                {chip.label}
                <span className="font-mono text-[11px] text-muted">{chip.count}</span>
              </button>
            );
          })}
        </div>

        {/* sort */}
        <div className="flex items-center gap-2">
          <label htmlFor="repo-sort" className="sr-only">
            Sort repositories
          </label>
          <select
            id="repo-sort"
            value={value.sort}
            onChange={(e) => onChange({ ...value, sort: e.target.value as SortKey })}
            className="h-9 rounded-full border border-white/10 bg-white/[0.03] px-3 text-xs text-ink
              transition-colors hover:border-white/25
              focus:border-accent/50 focus:outline-none focus:ring-2 focus:ring-accent/25"
          >
            {SORTS.map((sort) => (
              <option key={sort.value} value={sort.value} className="bg-bg-soft">
                {sort.label}
              </option>
            ))}
          </select>
        </div>
      </div>
    </div>
  );
}
