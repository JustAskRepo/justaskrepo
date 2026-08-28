"use client";

import GitHubMark from "@/components/github-mark";
import { githubLoginUrl } from "@/lib/api-client";

export default function GitHubButton({
  size = "lg",
  className = "",
}: {
  size?: "lg" | "sm";
  className?: string;
}) {
  const handleSignIn = () => {
    // Full-page navigation to Axum's OAuth entry — NOT a fetch. Axum runs the
    // GitHub dance, sets the session cookie, and redirects back into the app.
    window.location.href = githubLoginUrl();
  };

  const dims =
    size === "lg"
      ? "h-13 px-7 text-base"
      : "h-11 px-5 text-sm";

  return (
    <button
      type="button"
      onClick={handleSignIn}
      aria-label="Sign in with GitHub"
      className={`group relative inline-flex items-center justify-center gap-2.5 rounded-full font-semibold text-white
        bg-[#161620] ring-1 ring-white/12
        transition-[transform,box-shadow,background-color] duration-300
        hover:-translate-y-0.5 hover:bg-[#1d1d2b] hover:ring-white/25
        active:translate-y-0
        focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-2
        shadow-[0_10px_30px_-10px_rgba(0,0,0,0.8)]
        shimmer-host ${dims} ${className}`}
    >
      {/* animated glow ring on hover */}
      <span className="glow-ring opacity-0 transition-opacity duration-300 group-hover:opacity-70" />
      <GitHubMark className={size === "lg" ? "h-5 w-5" : "h-4 w-4"} />
      <span>Sign in with GitHub</span>
      <svg
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2.2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
        className="h-4 w-4 -mr-1 opacity-60 transition-transform duration-300 group-hover:translate-x-1 group-hover:opacity-100"
      >
        <path d="M5 12h14M13 6l6 6-6 6" />
      </svg>
    </button>
  );
}
