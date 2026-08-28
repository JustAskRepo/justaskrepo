"use client";

import GitHubMark from "@/components/github-mark";
import { githubAppInstallUrl } from "@/lib/api-client";

function IconPlus({ className = "" }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" className={className} aria-hidden="true">
      <path d="M12 5v14M5 12h14" />
    </svg>
  );
}

/**
 * Starts GitHub App installation with a full-page navigation to Axum, which
 * redirects on to GitHub. Same shape as sign-in: the frontend holds no GitHub
 * URL, slug or token of its own.
 */
export default function InstallAppButton({
  label = "Install GitHub App",
  size = "lg",
  variant = "primary",
  className = "",
}: {
  label?: string;
  size?: "lg" | "sm";
  variant?: "primary" | "secondary";
  className?: string;
}) {
  const handleInstall = () => {
    window.location.href = githubAppInstallUrl();
  };

  const dims = size === "lg" ? "h-13 px-7 text-base" : "h-9 px-4 text-sm";

  if (variant === "secondary") {
    return (
      <button
        type="button"
        onClick={handleInstall}
        className={`group inline-flex items-center justify-center gap-2 rounded-full border border-white/12 bg-white/[0.03] font-medium text-ink
          transition-colors duration-200 hover:border-accent/50
          focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-2 ${dims} ${className}`}
      >
        <IconPlus className="h-4 w-4 transition-transform duration-300 group-hover:rotate-90" />
        {label}
      </button>
    );
  }

  return (
    <button
      type="button"
      onClick={handleInstall}
      className={`group relative inline-flex items-center justify-center gap-2.5 rounded-full font-semibold text-white
        bg-[#161620] ring-1 ring-white/12
        transition-[transform,box-shadow,background-color] duration-300
        hover:-translate-y-0.5 hover:bg-[#1d1d2b] hover:ring-white/25
        active:translate-y-0
        focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-2
        shadow-[0_10px_30px_-10px_rgba(0,0,0,0.8)]
        shimmer-host ${dims} ${className}`}
    >
      <span className="glow-ring opacity-0 transition-opacity duration-300 group-hover:opacity-70" />
      <GitHubMark className={size === "lg" ? "h-5 w-5" : "h-4 w-4"} />
      <span>{label}</span>
      <svg
        viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round"
        aria-hidden="true"
        className="-mr-1 h-4 w-4 opacity-60 transition-transform duration-300 group-hover:translate-x-1 group-hover:opacity-100"
      >
        <path d="M5 12h14M13 6l6 6-6 6" />
      </svg>
    </button>
  );
}
