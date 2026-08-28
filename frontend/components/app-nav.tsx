"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import Logo from "@/components/logo";

/**
 * Health of the browser's link to the Axum API, surfaced in the nav.
 *
 * "offline" means the request never reached the server. A server that answers
 * with a failure is a different situation and says so — conflating the two puts
 * a red "Offline" next to an error panel explaining what the server replied.
 */
export type Connection = "connecting" | "live" | "offline" | "error" | "signedOut";

const LINKS = [
  { href: "/", label: "Home" },
  { href: "/dashboard", label: "Dashboard" },
] as const;

const CONNECTION = {
  connecting: {
    label: "Connecting",
    hint: "Loading data from the JustAskRepo API",
    dot: "bg-muted",
    text: "text-muted",
    ping: true,
  },
  live: {
    label: "Live",
    hint: "The last request to the API succeeded",
    dot: "bg-ok",
    text: "text-ok",
    ping: true,
  },
  offline: {
    label: "Offline",
    hint: "Your browser could not reach the API",
    dot: "bg-danger",
    text: "text-danger",
    ping: false,
  },
  error: {
    label: "API error",
    hint: "The API is reachable but answered with an error",
    dot: "bg-warn",
    text: "text-warn",
    ping: false,
  },
  signedOut: {
    label: "Signed out",
    hint: "Your session has expired — sign in again",
    dot: "bg-pending",
    text: "text-pending",
    ping: false,
  },
} as const;

/** `trailingSlash: true` means pathnames arrive as "/dashboard/". */
function normalize(path: string): string {
  return path.length > 1 ? path.replace(/\/+$/, "") : path;
}

export default function AppNav({ connection = "connecting" }: { connection?: Connection }) {
  const pathname = normalize(usePathname());
  const state = CONNECTION[connection];

  return (
    <header className="sticky top-0 z-40 border-b border-white/8 bg-bg/70 backdrop-blur-xl">
      <div className="mx-auto flex w-full max-w-7xl items-center justify-between gap-4 px-6 py-3.5">
        <Link href="/" aria-label="JustAskRepo home" className="shrink-0">
          <Logo />
        </Link>

        <nav className="flex items-center gap-1" aria-label="Main">
          {LINKS.map((link) => {
            const active = pathname === link.href;
            return (
              <Link
                key={link.href}
                href={link.href}
                aria-current={active ? "page" : undefined}
                className={`relative rounded-full px-3.5 py-1.5 text-sm transition-colors ${
                  active ? "text-ink" : "text-muted hover:text-ink"
                }`}
              >
                {link.label}
                <span
                  className={`absolute inset-x-3 -bottom-px h-px bg-linear-to-r from-transparent via-accent-2 to-transparent transition-opacity duration-300 ${
                    active ? "opacity-100" : "opacity-0"
                  }`}
                />
              </Link>
            );
          })}
        </nav>

        <span
          className="flex items-center gap-2 rounded-full border border-white/10 bg-white/5 px-3 py-1.5 text-xs"
          title={state.hint}
        >
          <span className="relative flex h-2 w-2">
            {state.ping && (
              <span className={`absolute inline-flex h-full w-full animate-ping rounded-full opacity-60 ${state.dot}`} />
            )}
            <span className={`relative inline-flex h-2 w-2 rounded-full ${state.dot}`} />
          </span>
          <span className={`hidden sm:inline ${state.text}`}>{state.label}</span>
          <span className="sr-only">{state.hint}</span>
        </span>
      </div>
    </header>
  );
}
