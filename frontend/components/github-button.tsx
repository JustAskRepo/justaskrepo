"use client";

function GitHubMark({ className = "" }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      aria-hidden="true"
      fill="currentColor"
      className={className}
    >
      <path d="M12 .5C5.73.5.5 5.74.5 12.02c0 5.1 3.29 9.42 7.86 10.95.58.1.79-.25.79-.56 0-.27-.01-1-.02-1.96-3.2.7-3.88-1.54-3.88-1.54-.52-1.34-1.28-1.7-1.28-1.7-1.05-.72.08-.71.08-.71 1.16.08 1.77 1.2 1.77 1.2 1.03 1.77 2.7 1.26 3.36.96.1-.75.4-1.26.73-1.55-2.55-.29-5.24-1.28-5.24-5.7 0-1.26.45-2.29 1.19-3.1-.12-.29-.52-1.46.11-3.05 0 0 .97-.31 3.18 1.18a11 11 0 0 1 5.79 0c2.2-1.49 3.17-1.18 3.17-1.18.63 1.59.23 2.76.11 3.05.74.81 1.19 1.84 1.19 3.1 0 4.43-2.69 5.41-5.25 5.69.41.36.78 1.07.78 2.16 0 1.56-.02 2.82-.02 3.2 0 .31.21.67.8.56A10.53 10.53 0 0 0 23.5 12.02C23.5 5.74 18.27.5 12 .5Z" />
    </svg>
  );
}

export default function GitHubButton({
  size = "lg",
  className = "",
}: {
  size?: "lg" | "sm";
  className?: string;
}) {
  const handleSignIn = () => {
    // Auth is intentionally not wired yet — backend stays untouched for now.
    // This is where the GitHub OAuth flow will kick off later.
    if (typeof window !== "undefined") {
      console.info("[JustAskRepo] GitHub sign-in coming soon.");
    }
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
