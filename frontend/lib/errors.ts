import { ApiError, NetworkError } from "@/lib/api-client";

/**
 * Turns a thrown error into copy a person can act on. Transport details
 * (`API /repositories failed: 404`) are kept, but demoted to `detail` so the UI
 * can tuck them away behind a disclosure instead of leading with them.
 *
 * Wording stays subject-free so callers can use it for any failed action.
 */
export type ErrorKind = "auth" | "offline" | "outdated" | "rateLimit" | "server" | "unknown";

export interface FriendlyError {
  kind: ErrorKind;
  /** Headline for a full error panel. */
  title: string;
  /** What happened, and what to do about it. */
  body: string;
  /** Short phrase for a one-line inline notice. */
  inline: string;
  /** The original message, for a collapsed "technical details" view. */
  detail: string;
}

export function describeError(error: unknown): FriendlyError {
  const detail = error instanceof Error ? error.message : String(error);

  if (error instanceof NetworkError) {
    return {
      kind: "offline",
      title: "No connection to JustAskRepo",
      body: "Your browser couldn’t reach the server. Check your internet connection, then try again.",
      inline: "You appear to be offline",
      detail,
    };
  }

  if (error instanceof ApiError) {
    if (error.isUnauthorized) {
      return {
        kind: "auth",
        title: "Your session has expired",
        body: "Sign in with GitHub again to pick up where you left off.",
        inline: "Your session has expired",
        detail,
      };
    }
    if (error.status === 404) {
      return {
        kind: "outdated",
        title: "The server didn’t recognise that request",
        body: "This part of the API may not be available yet. If the app was updated recently, reloading the page can help.",
        inline: "The server didn’t recognise the request",
        detail,
      };
    }
    if (error.status === 429) {
      return {
        kind: "rateLimit",
        title: "Too many requests",
        body: "A lot of requests went out in a short space of time. Wait a moment, then try again.",
        inline: "Too many requests — give it a moment",
        detail,
      };
    }
    if (error.status >= 500) {
      return {
        kind: "server",
        title: "Something went wrong on our side",
        body: "The server ran into a problem handling that. It is usually temporary — try again in a moment.",
        inline: "The server ran into a problem",
        detail,
      };
    }
  }

  return {
    kind: "unknown",
    title: "Something went wrong",
    body: "Something unexpected got in the way. Try again — if it keeps happening, the technical details below will help track it down.",
    inline: "Something unexpected got in the way",
    detail,
  };
}
