import type {
  EnqueueIndexResponse,
  RepoStatus,
  RepoSummary,
  WsTicket,
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

/** POST /api/chat/ws-ticket — single-use ticket to open the chat WebSocket. */
export function getWsTicket(sessionId: string): Promise<WsTicket> {
  return request<WsTicket>("/chat/ws-ticket", {
    method: "POST",
    body: JSON.stringify({ session_id: sessionId }),
  });
}

/**
 * Absolute ws(s):// URL for the chat socket, derived from the current origin so
 * it works in dev and prod without config. The ticket authenticates the socket
 * (cookies are not reliably sent on WebSocket handshakes across contexts).
 */
export function chatSocketUrl(ticket: string): string {
  const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${proto}//${window.location.host}${API_BASE}/ws/chat?ticket=${encodeURIComponent(ticket)}`;
}

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
