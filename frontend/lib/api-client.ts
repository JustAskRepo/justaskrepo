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

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    ...init,
    // Send the Axum session cookie on every call (explicit; same-origin anyway).
    credentials: "include",
    headers: { "Content-Type": "application/json", ...init?.headers },
  });
  if (!res.ok) {
    throw new Error(`API ${path} failed: ${res.status}`);
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
  return `${API_BASE}/auth/github/login`;
}
