"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { ApiError, getMe } from "@/lib/api-client";
import type { Me } from "@/types/api";

/**
 * Four outcomes, not three. A 401 means the visitor is genuinely signed out and
 * should be offered sign-in; a network failure or a 5xx means we do not know
 * who they are, which is not the same claim. Showing "Sign in" to someone whose
 * session is fine but whose backend blipped is the bug this distinction avoids.
 */
export type SessionStatus = "loading" | "signedIn" | "signedOut" | "unavailable";

export interface SessionState {
  status: SessionStatus;
  me: Me | null;
}

/**
 * Loads `/api/me` once in the browser. Static export means there is no server
 * to resolve the session during render, so the header mounts in `loading` and
 * settles a moment later — hence the placeholder chip rather than a layout jump.
 */
export function useSession(): SessionState {
  const [state, setState] = useState<SessionState>({ status: "loading", me: null });
  const alive = useRef(true);

  const settle = useCallback((next: SessionState) => {
    if (alive.current) setState(next);
  }, []);

  useEffect(() => {
    alive.current = true;
    getMe().then(
      (me) => settle({ status: "signedIn", me }),
      (err: unknown) => {
        const signedOut = err instanceof ApiError && err.isUnauthorized;
        settle({ status: signedOut ? "signedOut" : "unavailable", me: null });
      },
    );
    return () => {
      alive.current = false;
    };
  }, [settle]);

  return state;
}
