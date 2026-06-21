import type {
  EnqueueIndexResponse,
  RepoStatus,
  RepoSummary,
  WsTicket,
} from "@/types/api";

/**
 * Typed browser-side fetch wrapper (see extras/JustAskRepo-Architecture.md §8).
 *
 * The browser only ever talks to the Next.js BFF route handlers under
 * `/api/*` — never directly to Axum (except the chat WebSocket via a
 * single-use ticket). The route handlers proxy to Axum with the internal JWT.
 */

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    ...init,
    headers: { "Content-Type": "application/json", ...init?.headers },
  });
  if (!res.ok) {
    throw new Error(`API ${path} failed: ${res.status}`);
  }
  return res.json() as Promise<T>;
}

/** GET /api/repos -> Axum GET /repositories. */
export function getRepos(): Promise<RepoSummary[]> {
  return request<RepoSummary[]>("/api/repos");
}

/** GET /api/repos/:id/status -> Axum GET /repositories/:id/status. */
export function getRepoStatus(repoId: string): Promise<RepoStatus> {
  return request<RepoStatus>(`/api/repos/${repoId}/status`);
}

/** POST /api/repos/:id/index -> Axum POST /repositories/:id/index. */
export function startIndexing(
  repoId: string,
  body: { mode?: "full" | "incremental"; force?: boolean } = {},
): Promise<EnqueueIndexResponse> {
  return request<EnqueueIndexResponse>(`/api/repos/${repoId}/index`, {
    method: "POST",
    body: JSON.stringify(body),
  });
}

/** POST /api/chat/ws-ticket -> Axum POST /chat/ws-ticket. */
export function getWsTicket(sessionId: string): Promise<WsTicket> {
  return request<WsTicket>("/api/chat/ws-ticket", {
    method: "POST",
    body: JSON.stringify({ session_id: sessionId }),
  });
}
