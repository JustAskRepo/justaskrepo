/**
 * Shared DTO types mirroring the Axum backend API (see
 * extras/JustAskRepo-Architecture.md §6 data model and §9 API design).
 *
 * These are the wire shapes the browser exchanges directly with the Rust/Axum
 * backend over `/api/*` (this is a static export — there is no BFF layer).
 * Keep them in sync with the backend's response types.
 */

export type IndexStatus =
  | "never_indexed"
  | "queued"
  | "indexing"
  | "indexed"
  | "failed"
  | "stale";

export type JobStatus =
  | "queued"
  | "running"
  | "succeeded"
  | "failed"
  | "cancelled"
  | "superseded";

export type JobStage = "cloning" | "chunking" | "embedding" | "storing";

/** A single repository as listed on the dashboard (GET /repositories). */
export interface RepoSummary {
  repository_id: string;
  owner_login: string;
  name: string;
  full_name: string;
  is_private: boolean;
  default_branch: string;
  current_index_status: IndexStatus;
  last_indexed_commit_sha: string | null;
  last_indexed_at: string | null;
}

/** Summary of an index job, embedded in repo status responses. */
export interface IndexJob {
  id: string;
  status: JobStatus;
  current_stage: JobStage | null;
  /** Live progress percentage sourced from Valkey (0–100). */
  pct?: number;
}

/** GET /repositories/:id/status. */
export interface RepoStatus {
  repository_id: string;
  current_index_status: IndexStatus;
  last_indexed_commit_sha: string | null;
  last_indexed_at: string | null;
  last_index_job: IndexJob | null;
}

/** 202 response from POST /repositories/:id/index. */
export interface EnqueueIndexResponse {
  job_id: string;
  status: JobStatus;
  repository_id: string;
  deduped: boolean;
}

/** Single-use WebSocket ticket from POST /chat/ws-ticket. */
export interface WsTicket {
  ticket: string;
}

export type ChatRole = "user" | "assistant" | "system";

export interface Citation {
  file_path: string;
  start_line: number;
  end_line: number;
  score: number;
  chunk_id?: string;
}

export interface ChatMessage {
  id: string;
  session_id: string;
  role: ChatRole;
  content: string;
  citations?: Citation[];
  created_at: string;
}

/** Server -> client WebSocket frames (see §9). */
export type ChatFrame =
  | { type: "token"; delta: string }
  | { type: "citation"; file_path: string; start_line: number; end_line: number; score: number }
  | { type: "done"; message_id: string }
  | { type: "error"; code: string; message: string };

/**
 * GET /api/me — who the current session belongs to.
 *
 * The ids come from the session record; the profile is read from `users` on
 * every call, so a GitHub rename shows up on the next request rather than
 * whenever the 30-day session finally expires.
 *
 * `name` and `avatar_url` are nullable rather than optional — the backend
 * always sends the keys, GitHub just does not always have values for them. The
 * fallbacks in `features/session/identity.ts` exist for that, not for a missing
 * field. `email` is stored but deliberately not sent: nothing renders it.
 */
export interface Me {
  user_id: number;
  github_id: number;
  username: string;
  name: string | null;
  avatar_url: string | null;
}
