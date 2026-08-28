"use client";

import { useEffect, useState } from "react";
import { startIndexing } from "@/lib/api-client";
import { describeError } from "@/lib/errors";

type Phase = "idle" | "pending" | "queued" | "error";

const COPY: Record<Phase, string> = {
  idle: "Index",
  pending: "Queuing…",
  queued: "Queued",
  error: "Try again",
};

/**
 * Triggers (re)indexing of a repository via Axum. Reports the outcome inline
 * in plain language and hands control back to the caller through `onQueued`,
 * so the list can refresh and pick the job up in its polling loop.
 */
export default function IndexButton({
  repoId,
  onQueued,
  className = "",
}: {
  repoId: string;
  onQueued?: () => void;
  className?: string;
}) {
  const [phase, setPhase] = useState<Phase>("idle");
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    if (phase !== "queued") return;
    const id = setTimeout(() => setPhase("idle"), 2500);
    return () => clearTimeout(id);
  }, [phase]);

  async function handleClick() {
    setPhase("pending");
    setMessage(null);
    try {
      await startIndexing(repoId);
      setPhase("queued");
      onQueued?.();
    } catch (err) {
      const friendly = describeError(err);
      console.error("Failed to start indexing:", friendly.detail, err);
      setMessage(friendly.inline);
      setPhase("error");
    }
  }

  const tone =
    phase === "error"
      ? "border-danger/40 text-danger hover:border-danger/70"
      : phase === "queued"
        ? "border-ok/40 text-ok"
        : "border-white/12 text-muted hover:border-accent/50 hover:text-ink";

  return (
    <span className="relative z-10 inline-flex flex-col items-end gap-1.5">
      <button
        type="button"
        onClick={handleClick}
        disabled={phase === "pending"}
        className={`inline-flex items-center gap-1.5 rounded-full border px-3 py-1.5 text-xs font-medium
          transition-colors duration-200 disabled:opacity-60
          focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-2 ${tone} ${className}`}
      >
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
          className={`h-3.5 w-3.5 ${phase === "pending" ? "animate-spin" : ""}`}
        >
          <path d="M21 12a9 9 0 1 1-2.64-6.36" />
          <path d="M21 3v6h-6" />
        </svg>
        {COPY[phase]}
      </button>

      {phase === "error" && message && (
        <span role="status" className="max-w-[13rem] text-right text-[11px] leading-snug text-danger">
          {message}
        </span>
      )}
    </span>
  );
}
