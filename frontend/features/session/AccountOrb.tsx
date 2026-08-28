"use client";

import { useEffect, useRef, useState } from "react";
import GitHubButton from "@/components/github-button";
import Avatar from "@/features/session/Avatar";
import IndexHalo, { haloSummary } from "@/features/session/IndexHalo";
import SessionPanel from "@/features/session/SessionPanel";
import { accountTitle, displayName } from "@/features/session/identity";
import { useSession } from "@/features/session/useSession";
import type { RepoSummary } from "@/types/api";

const ORB = 40;

/**
 * The account cluster in the header. Self-fetches the session (identity is
 * app-wide, so prop-drilling it through every page would buy nothing) but takes
 * `repos` from the caller, because the dashboard has already loaded and is
 * already polling that list — the halo rides along instead of doubling it.
 */
export default function AccountOrb({
  repos = null,
  reposError = null,
}: {
  repos?: readonly RepoSummary[] | null;
  /** Lets the panel say "couldn't load" instead of spinning on "loading". */
  reposError?: Error | null;
}) {
  const { status, me } = useSession();
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) return;

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      setOpen(false);
      triggerRef.current?.focus();
    };
    const onPointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };

    document.addEventListener("keydown", onKeyDown);
    document.addEventListener("pointerdown", onPointerDown);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
      document.removeEventListener("pointerdown", onPointerDown);
    };
  }, [open]);

  if (status === "loading") {
    return <span className="skeleton block rounded-full" style={{ width: ORB, height: ORB }} />;
  }

  if (status === "signedOut") {
    return <GitHubButton size="sm" />;
  }

  // "unavailable" — we don't know who this is, and the connection pill beside
  // us is already saying why. Two reports of one outage is one too many.
  if (status === "unavailable" || me === null) {
    return null;
  }

  const name = displayName(me);

  return (
    <div ref={rootRef} className="relative">
      <button
        ref={triggerRef}
        type="button"
        onClick={() => setOpen((prev) => !prev)}
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-controls="account-panel"
        aria-label={`Account — ${accountTitle(me)}. ${haloSummary(repos)}`}
        className={`flex items-center gap-2.5 rounded-full py-1 pl-1 transition-colors duration-200
          focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-2
          ${name ? "pr-3.5" : "pr-1"}
          ${open ? "bg-white/8" : "hover:bg-white/5"}`}
      >
        <IndexHalo repos={repos} size={ORB}>
          <Avatar me={me} size={ORB} />
        </IndexHalo>
        {name && (
          <span className="hidden max-w-[9rem] truncate text-sm font-medium text-ink lg:inline">
            {name}
          </span>
        )}
      </button>

      {open && (
        <div
          id="account-panel"
          role="dialog"
          aria-label="Account and workspace"
          className="absolute right-0 top-[calc(100%+0.6rem)] z-50"
        >
          <SessionPanel me={me} repos={repos} reposError={reposError} />
        </div>
      )}
    </div>
  );
}
