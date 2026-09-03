import type {
  EnqueueIndexResponse,
  Me,
  RepoStatus,
  RepoSummary,
} from "@/types/api";

/**
 * Typed browser fetch wrapper. This is a **static export** — there is no Next
 * server and no BFF. The browser calls the Axum backend directly, same-origin,
 * under `/api/*` (Axum serves the exported UI at `/` and mounts its API under
 * `/api`). Same-origin means relative paths and the session cookie is sent
 * automatically — no build-time API URL, no token handling in the client.
 *
 * Auth is a same-origin httpOnly session cookie set by Axum's GitHub OAuth flow
 * (see `githubLoginUrl`). The browser never sees or holds a token.
 */

/** Axum API prefix, same origin as the static assets. */
const API_BASE = "/api";

/**
 * A non-2xx response from Axum. Carries the status so callers can tell an
 * expired session (401) from a backend that is down or not yet serving the
 * route — the dashboard renders a different screen for each.
 */
export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly path: string,
  ) {
    super(`API ${path} failed: ${status}`);
    this.name = "ApiError";
  }

  /** No/expired session cookie — the visitor needs to sign in again. */
  get isUnauthorized(): boolean {
    return this.status === 401 || this.status === 403;
  }
}

/**
 * The request never got a response at all — offline, DNS failure, connection
 * refused, CORS. Distinct from `ApiError`, which means the server answered and
 * said no.
 */
export class NetworkError extends Error {
  constructor(
    readonly path: string,
    options?: { cause?: unknown },
  ) {
    super(`Network request to ${path} failed`, options);
    this.name = "NetworkError";
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  let res: Response;
  try {
    res = await fetch(`${API_BASE}${path}`, {
      ...init,
      // Send the Axum session cookie on every call (explicit; same-origin anyway).
      credentials: "include",
      headers: { "Content-Type": "application/json", ...init?.headers },
    });
  } catch (cause) {
    // fetch only rejects when the request never completed — a real "offline",
    // not a 4xx/5xx. Naming it here keeps that distinction all the way to the UI.
    throw new NetworkError(path, { cause });
  }
  if (!res.ok) {
    throw new ApiError(res.status, path);
  }
  return res.json() as Promise<T>;
}

/** GET /api/repositories. */
export function getRepos(): Promise<RepoSummary[]> {
  return request<RepoSummary[]>("/repositories");
}

/** GET /api/repositories/:id/status. */
export function getRepoStatus(repoId: string): Promise<RepoStatus> {
  return request<RepoStatus>(`/repositories/${repoId}/status`);
}

/** POST /api/repositories/:id/index. */
export function startIndexing(
  repoId: string,
  body: { mode?: "full" | "incremental"; force?: boolean } = {},
): Promise<EnqueueIndexResponse> {
  return request<EnqueueIndexResponse>(`/repositories/${repoId}/index`, {
    method: "POST",
    body: JSON.stringify(body),
  });
}

/*
 * Chat streaming has no helper here yet, and will not need a ticket when it does.
 * It is SSE, not a WebSocket (decided 2026-09-03 — AUTHENTICATION.md §Streaming
 * Authentication, ADR-008 §7): POST the question, then open an `EventSource` on
 * the reply stream. Both are same-origin, so both carry the session cookie by
 * themselves. The helper lands with the backend route.
 */

/**
 * URL that kicks off GitHub OAuth on the backend. Navigate the browser here
 * (a full navigation, not fetch) — Axum runs the OAuth dance, sets the session
 * cookie, and redirects back to the app.
 */
export function githubLoginUrl(): string {
  return `${API_BASE}/auth/github`;
}

/**
 * URL that starts GitHub App installation. Navigate the browser here (a full
 * navigation, not fetch) — Axum redirects on to GitHub's installation screen
 * and handles the return trip.
 *
 * The install target is a GitHub URL built from the App's slug, which is
 * backend config (see `INSTALL_URL`, ARCHITECTURE.md §5.1). A static export has
 * no env vars, so the frontend must not try to construct it: it asks Axum,
 * exactly as sign-in does.
 */
export function githubAppInstallUrl(): string {
  return `${API_BASE}/installations/new`;
}

/** GET /api/me — identity behind the session cookie. 401 when signed out. */
export function getMe(): Promise<Me> {
  return request<Me>("/me");
}

/**
 * For routes that answer `204` with no body. `request` cannot serve them — it
 * always parses the response as JSON, and there is nothing to parse.
 */
async function requestNoContent(path: string, init?: RequestInit): Promise<void> {
  let res: Response;
  try {
    res = await fetch(`${API_BASE}${path}`, { ...init, credentials: "include" });
  } catch (cause) {
    throw new NetworkError(path, { cause });
  }
  if (!res.ok) {
    throw new ApiError(res.status, path);
  }
}

/**
 * POST /api/auth/logout — revoke the current session.
 *
 * A fetch, not a navigation. AUTHENTICATION.md §Logout Flow specifies a POST,
 * and mutating routes carry an Origin check, so a full-page GET would be
 * rejected rather than sign anyone out. Axum answers `204` and returns a
 * `Max-Age=0` cookie that clears the session cookie; callers navigate on their
 * own once this resolves.
 *
 * A `401` resolves instead of throwing. The route sits behind the session
 * middleware, so an already-expired session never reaches it — and "there is no
 * session to revoke" is precisely the state the caller asked for. Surfacing it
 * as an error would fail the sign-out that needed no work, which is the one the
 * user is least able to act on.
 *
 * `403` is deliberately *not* folded in, and `isUnauthorized` is deliberately
 * not used: it covers 401 and 403 alike, and a 403 here is the Origin check
 * rejecting the request as forged. Nothing was revoked, the session is still
 * live, and reporting success would be a lie.
 */
export async function logout(): Promise<void> {
  try {
    await requestNoContent("/auth/logout", { method: "POST" });
  } catch (err) {
    if (err instanceof ApiError && err.status === 401) return;
    throw err;
  }
}

/**
 * POST /api/auth/logout/all — revoke every session for this user, this device
 * included. Same `204` and same cookie clearing as `logout`.
 *
 * No 401 exemption here, and the asymmetry is the point: a 401 means the route
 * never ran, so this user's *other* sessions are still alive. Resolving would
 * report "signed out everywhere" for a device that is still signed in.
 */
export function logoutAll(): Promise<void> {
  return requestNoContent("/auth/logout/all", { method: "POST" });
}
