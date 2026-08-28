# Authentication Architecture — JustAskRepo

> **Last Updated:** 2026-08-28
> **Status:** Active — login flow, session middleware (including the `Origin` check
> on mutating methods), `/api/me`, and both logout routes are implemented.

## Overview

JustAskRepo uses a **single GitHub App** for both identity and repository access, and
**server-side sessions** for authentication. The **Rust backend (Axum) is the single
source of truth** for all authentication and authorization.

The frontend is a **Next.js static export**. There is no Next.js server in production
and no BFF layer — Axum serves the exported HTML/JS/CSS at `/` and mounts the API
under `/api/*`. The browser talks to Axum **directly, same-origin**. It never sees the
session ID (the cookie is `HttpOnly`), a GitHub token, or any auth secret.

**Design stance:** opaque session cookies, no JWTs. This keeps revocation instant,
avoids token-invalidation complexity, and keeps every auth decision on the backend.

**One app, two token types.** Rather than a separate OAuth App for login plus a
GitHub App for repos, a single GitHub App provides:

| Token | Flow | Used for |
| --- | --- | --- |
| **User access token** (user-to-server) | OAuth 2.0 authorization-code flow | Login / identity — fetch profile |
| **Installation token** (server-to-server) | App JWT → installation token | Repository access (clone, index) |

The login flow is the standard OAuth authorization-code flow; only the app
*registration* differs from a plain OAuth App. The user token is used once at login
to fetch the profile and then discarded — the Valkey session is the source of truth,
not the GitHub token.

## Technology Stack

| Component | Technology |
| --- | --- |
| Frontend (UI only) | Next.js static export, served by Axum |
| Backend (auth owner) | Rust (Axum) |
| Identity + repo access | Single GitHub App |
| Session store | Valkey (with AOF persistence) |
| Persistent storage | PostgreSQL |
| Auth mechanism | Opaque session cookie |

---

## System Architecture

```text
                  ┌────────────────────────────────────────────┐
                  │                 Browser                    │
                  │   Next.js static export (HTML / JS / CSS)  │
                  └─────────────────────┬──────────────────────┘
                                        │
                    ONE ORIGIN — Cookie: __Host-session=<id>
                                        │
                     GET  /          → exported UI (ServeDir)
                     GET  /api/*     → JSON API
                                        │
                                        ▼
                  ┌────────────────────────────────────────────┐
                  │            Rust Backend (Axum)             │
                  │                                            │
                  │  ServeDir  ──────────► exported UI at /    │
                  │  /api/*    ──────────► auth middleware     │
                  │                        + module handlers   │
                  │                                            │
                  │  • GitHub App user login / callback        │
                  │  • Installation tokens for repo access     │
                  │  • Session create / validate / revoke      │
                  │  • Rate limiting, Origin checks            │
                  └────────┬─────────────┬─────────────┬───────┘
                           ▼             ▼             ▼
                      PostgreSQL      Valkey       GitHub App
                    (user records)  (sessions)   (user + install
                                                     tokens)
```

**Why one origin matters.** Because the UI and the API share an origin, there is
**no CORS layer, no preflight handling, and no cross-site cookie problem**. The
session cookie is a plain same-site cookie. This is a deliberate simplification of
the security model, not an accident of packaging — see *Same-Origin Model* below.

**Dev parity.** In development the Next dev server runs on `:3001` and rewrites
`/api/*` to Axum (`next.config.ts`), so the browser sees a single origin in dev too
and cookies behave identically. **Never open `:8080` directly in dev** — that bypasses
the proxy and splits the origin.

**Separation of concerns**

- **PostgreSQL** — durable identity. Match key is the immutable `github_id`.
- **Valkey** — ephemeral session state, keyed by session ID, TTL-expired.
- **GitHub App** — user-to-server tokens for login, installation tokens for repos.
- Losing Valkey logs everyone out (fail-closed) but loses no identity data.

---

## Routes

All backend routes live under `/api`. The prefix is **load-bearing**: `/` is the
exported UI, and the dev proxy only forwards `/api/*` to Axum. A route registered
outside `/api` is unreachable in development.

**Public (no auth middleware)**

| Route | Purpose |
| --- | --- |
| `GET  /api/auth/github` | Start login — 302 to GitHub authorize URL |
| `GET  /api/auth/github/callback` | OAuth callback — creates the session |
| `POST /api/webhooks/github` | GitHub App webhooks (HMAC-verified, not cookie-authed) |

**Authenticated (auth middleware runs first)**

| Route | Purpose |
| --- | --- |
| `POST /api/auth/logout` | Revoke the current session |
| `POST /api/auth/logout/all` | Revoke every session for this user |
| `GET  /api/me` | Authenticated profile, or `401` |
| `GET  /api/repositories` | Repos visible to this user |
| `GET  /api/repositories/{id}/status` | Index status |
| `POST /api/repositories/{id}/index` | Enqueue indexing |
| `POST /api/chat/ws-ticket` | Mint a single-use WebSocket ticket |
| `GET  /api/ws/chat?ticket=…` | Chat WebSocket (ticket-authenticated) |

> Axum 0.8 uses `{id}` for path parameters, not `:id`, and no longer matches
> trailing slashes implicitly. `next.config.ts` sets `skipTrailingSlashRedirect`
> for exactly this reason.

---

## Login Flow

```text
User clicks "Continue with GitHub"
      │
      ▼
Browser navigates to GET /api/auth/github     ← full navigation, not fetch
      │
      ├─ Backend generates a one-time `state`, stores it in Valkey (short TTL)
      └─ 302 → GitHub App authorize URL (with state)
      │
      ▼
GitHub — user authorizes the App (user-to-server)
      │
      ▼
GET /api/auth/github/callback?code=…&state=…
      │
      ├─ Verify `state` matches the value in Valkey, then delete it (single use)
      ├─ Exchange `code` for a user access token
      ├─ Fetch GitHub profile with the user token, then discard the token
      ├─ Upsert user in PostgreSQL, matched on github_id
      ├─ Generate a fresh, cryptographically random session ID  ← rotate on every login
      ├─ Store session in Valkey, add it to the user's session index
      ├─ Set __Host-session cookie (HttpOnly, Secure, SameSite=Lax)
      │
      ▼
Has the App been installed on any repos for this user/org?
      │
      ├─ No  → 302 → GitHub App install page (select repos), then dashboard
      └─ Yes → 302 → /dashboard/
```

**Authorization vs installation.** For a GitHub App these are separate:

- **Authorization** (user-to-server) proves *who* the user is → enough to create a
  session. A user can be authorized without having installed the App.
- **Installation** grants *repository access* → required before indexing. The backend
  mints installation tokens from the App JWT; user tokens are never used for repo I/O.

The UI must handle the **authorized-but-not-installed** state: the user is logged in
but has no installation yet, so prompt them to install the App on the repos they want
indexed. This is the normal onboarding path, not an error.

> **Module boundary note.** "Has the App been installed?" is data owned by the
> `installations` module, not `auth`. The `auth` module must not query it. The HTTP
> callback handler (composition root) calls `auth`'s command, then `installations`'
> query, and decides the redirect. See `ARCHITECTURE.md` §5.

**Session fixation defense:** a brand-new session ID is minted at the end of a
successful login. Any pre-login cookie value is discarded, so a planted cookie can
never become an authenticated session.

---

## Authenticated Request Flow

```text
Browser ── Cookie: __Host-session=<id> ──► Rust auth middleware
                                              │
                                              ├─ Read cookie
                                              ├─ For mutating requests, verify Origin
                                              ├─ Look up session:<id> in Valkey
                                              ├─ Reject if missing or past expires_at
                                              ├─ Refresh idle TTL (throttled — see below)
                                              ├─ Load user, attach to request context
                                              ▼
                                        Protected route handler
```

A failed `Origin` check is **`403 Forbidden`** — the caller may well be who they
say they are; the request just is not trustworthy. Every other failure is
**`401 Unauthorized`**. Either way no handler runs.

---

## Logout Flow

**Single session**

```text
POST /api/auth/logout
      │
      ├─ Delete session:<id> from Valkey
      ├─ Remove id from the user's session index
      ├─ Expire the cookie (Max-Age=0)
      ▼
204 No Content
```

**All sessions ("log out everywhere")**

```text
POST /api/auth/logout/all
      │
      ├─ Read user_sessions:<user_id> (the session index)
      ├─ Delete every session:<id> in that set
      ├─ SREM exactly those ids from the index (not DEL of the key — a login
      │  racing this write would otherwise be unindexed but still alive)
      ├─ Expire the cookie on the calling device
      ▼
204 No Content
```

The per-user index is what makes this a bounded operation instead of a keyspace scan.
It is the **only** bulk-revocation mechanism — see the note under *Data Model*.

**Involuntary revocation.** Sessions are also revoked outside of user-initiated
logout:

| Trigger | Action |
| --- | --- |
| GitHub App uninstalled for the user | Revoke all sessions for that user |
| User record deleted | Revoke all sessions, then delete the index |
| Suspected theft (IP / UA anomaly) | Revoke the affected session, force re-auth |

Uninstall arrives as a webhook, so the `webhooks` module routes it and `auth`
reacts — via the event bus, never a direct call.

---

## Data Model

### PostgreSQL — `users` (durable identity only)

| Column | Description |
| --- | --- |
| id | Internal user ID (primary key) |
| github_id | GitHub user ID — **immutable match key** |
| username | GitHub username (mutable, display only) |
| email | GitHub email, if available (display only) |
| avatar_url | GitHub avatar URL |
| created_at | Account creation |
| updated_at | Last profile update |
| last_login_at | Last successful login (audit/support) |

> Users are matched **only on `github_id`**. Never match on `email` or `username` —
> both are mutable and reusable, and matching on them enables account takeover.

### Valkey — sessions (ephemeral)

**Session record** — `session:<session_id>`

```json
{
  "user_id": 123,
  "github_id": 987654,
  "created_at": "2026-07-25T12:00:00Z",
  "last_seen_at": "2026-07-25T14:30:00Z",
  "expires_at": "2026-08-24T12:00:00Z",
  "ip": "203.0.113.7",
  "user_agent": "Mozilla/5.0 …"
}
```

**Per-user session index** — `user_sessions:<user_id>` → set of session IDs
(enables "log out all devices" without scanning the keyspace).

**Login CSRF state** — `oauth_state:<state>` → short TTL (e.g. 10 min), single use.

> **No `session_version`.** An earlier draft carried a `session_version` field for
> bulk revocation. It is deliberately omitted: a version number only revokes anything
> if there is an authoritative counterpart to compare against, which would mean a
> column on `users` and therefore **a Postgres read on every authenticated request**.
> The per-user session index already gives bulk revocation with one Valkey call.
> Don't reintroduce the field.

**Expiry model**

- **Valkey TTL is authoritative** for whether a session exists.
- `expires_at` enforces a hard **30-day absolute** cap. The absolute cap is *not*
  extended by activity — a session dies 30 days after login regardless.
- `last_seen_at` + TTL refresh give a **sliding idle timeout** — active sessions stay
  alive; idle ones expire early.
- **TTL refresh is throttled.** Rewriting the session on every request means a Valkey
  write per authenticated request. Only refresh when `last_seen_at` is older than a
  threshold (start at 5 minutes). Same user-visible behaviour, a fraction of the
  writes.

> GitHub access/installation tokens are **never stored in the session record.**
> See Token Handling below.

---

## Session Cookie

```text
__Host-session=<random_session_id>
```

| Attribute | Value | Reason |
| --- | --- | --- |
| `__Host-` | prefix | Locks cookie to Secure + Path=/ + no Domain |
| HttpOnly | on | JavaScript cannot read it |
| Secure | on | HTTPS only |
| SameSite | Lax | Blocks cross-site cookie sends on unsafe methods |
| Path | / | Sent to all backend routes |
| Max-Age | 30 days | Matches absolute session cap |

The frontend never has access to the session ID. To know if the user is signed in,
the UI calls **`GET /api/me`**, which returns the authenticated profile or `401`.

> **Dev caveat — resolved 2026-08-23.** The `__Host-` prefix requires the `Secure`
> attribute, and dev runs over plain `http://localhost:3001`. Verified by hand in
> Chrome, Firefox, and Brave: all three treat `localhost` as a trustworthy origin
> and store the cookie. **No dev fallback is needed** — do not add one. The cookie
> name stays config-driven via `SESSION_COOKIE_NAME` regardless, so if some future
> browser or host breaks this, the fix is a different name in dev config, never a
> weakened attribute set and never `#[cfg(debug_assertions)]`.

---

## WebSocket Authentication

Cookies are not reliably attached to WebSocket handshakes across all contexts, and
the handshake cannot be rejected with a useful body. Chat therefore uses a
**single-use ticket**:

```text
POST /api/chat/ws-ticket      ← normal cookie-authenticated request
      │
      ├─ Auth middleware resolves the session from the cookie
      ├─ Mint a random ticket, store ws_ticket:<ticket> → user_id in Valkey (TTL ≤ 30s)
      ▼
{ "ticket": "…" }
      │
      ▼
GET /api/ws/chat?ticket=…
      │
      ├─ Look up and DELETE the ticket (single use)
      ├─ Reject with 401 if missing or expired
      ▼
Socket upgraded, bound to that user
```

**The ticket endpoint takes no request body.** The session comes from the cookie via
the normal middleware — the browser cannot read an `HttpOnly` cookie, so a client
that passes a session ID as an argument is by definition passing something it should
not have. The current frontend scaffold (`frontend/lib/api-client.ts`,
`getWsTicket(sessionId)`) contradicts this and must be corrected when the endpoint is
implemented.

Ticket TTL is deliberately tiny — it exists only to bridge one HTTP request to one
socket open, and it appears in a URL, which means it can land in logs.

---

## Token Handling (GitHub App)

The single GitHub App issues two token types; both stay on the backend.

- **User access token** (user-to-server) — obtained during the login callback, used
  **once** to fetch the profile, then **discarded**. It is never stored or returned
  to the frontend. GitHub App user tokens expire in ~8h with a 6-month refresh token,
  but expiry is irrelevant here since we don't retain the token. If a future feature
  ever needs to keep one, it must be **encrypted at rest** (envelope encryption via a
  KMS or libsodium/`age`) in PostgreSQL — **never** in the Valkey session JSON and
  never in a cookie.
- **Installation token** (server-to-server) — minted on demand from the App JWT for
  repository access (clone, index, GitHub API). Short-lived (~1h); cache briefly in
  memory and **never persist**. All repo I/O uses installation tokens, never user
  tokens.

---

## Security Controls

| Threat | Control |
| --- | --- |
| Login CSRF | One-time `state` in Valkey, verified and deleted on callback |
| API CSRF | Same-origin + `SameSite=Lax` **plus** Origin check on mutating routes |
| Session fixation | Fresh session ID minted on every successful login |
| Session theft (detection) | `ip` / `user_agent` recorded; anomalies can force re-auth |
| Bulk revocation | Per-user session index (`user_sessions:<user_id>`) |
| Session ID guessing | Cryptographically secure random IDs (`rand`, ≥128 bits) |
| WebSocket auth | Single-use, short-TTL ticket; never the session ID |
| Brute force / abuse | Valkey rate limiting on the auth routes and APIs |
| Token leakage | GitHub tokens not stored; encrypted only if ever persisted |
| Secrets in logs | Secrets wrapped in `SecretString` — `Debug` prints `[REDACTED]` |
| Transport | HTTPS required in production |

### Same-Origin Model

The UI and the API are served from one origin (Axum serves the static export at `/`
and the API at `/api/*`; the dev proxy reproduces this). Consequences:

- **No CORS layer.** No `Access-Control-Allow-Origin`, no
  `Access-Control-Allow-Credentials`, no preflight handling. Do not add a `CorsLayer`
  — it would grant access that currently does not exist.
- **No cross-site cookie problem.** `SameSite=Lax` is a straightforward fit rather
  than a compromise; the browser has no reason to send this cookie cross-site.
- **CSRF defense is genuinely sufficient.** `SameSite=Lax` blocks cross-site sends on
  unsafe methods, and an `Origin` check on mutating routes covers the remainder. No
  CSRF token is needed. The `Origin` check runs inside the session
  middleware, so it covers exactly the cookie-authenticated routes and nothing else
  — see `ARCHITECTURE.md` §5.2.

> An earlier revision of this document described Next.js and Axum as separate origins
> requiring CORS. That described the BFF layout removed in commit `8afb83d`. If a
> future change reintroduces a separate frontend origin, this entire section — and
> the cookie attributes above — must be revisited **before** the change ships.

### Availability

Valkey is on the critical path for every authenticated request. Run it with **AOF
persistence** so sessions survive a restart, and treat a Valkey outage as
**fail-closed** — the backend returns `401` rather than serving unauthenticated
requests.

---

## Why Session-Based (not JWT)

- **Single source of truth** — every auth decision is a backend Valkey lookup.
- **Instant revocation** — delete the key; no waiting for a token to expire.
- **No token-invalidation problem** — no refresh tokens, no rotation dance, no
  stolen-JWT-still-valid window.
- **Natural fit for Valkey** — TTLs, rate limits, the OAuth `state`, and WS tickets
  all live in one fast store.

The design stays centralized and easy to extend — additional identity providers slot
in behind the same session model without touching cookies, middleware, or logout.

## Why a Single GitHub App (not OAuth App + GitHub App)

- **One consent surface** — users authorize and install one App instead of juggling a
  separate OAuth App login plus a GitHub App installation.
- **Fine-grained permissions** — per-repository selection and scoped permissions,
  rather than OAuth's coarse account-wide scopes.
- **First-class installation tokens** — backend repo access needs no long-lived user
  token; installation tokens are minted from the App JWT on demand.
- **Same login flow** — user-to-server auth is the standard OAuth authorization-code
  flow, so the session model above is unchanged.

---

## Open Items

Tracked here so they don't get lost between this document and ADR-007.

- [x] Confirm `__Host-` cookies are accepted on `http://localhost` — verified
      2026-08-23 in Chrome, Firefox, and Brave
- [x] `frontend/lib/api-client.ts` — `githubLoginUrl()` aligned to `/api/auth/github`
      (also `frontend/CLAUDE.md` rule 2, `frontend/next.config.ts` comment)
- [x] `backend/src/main.rs` — placeholder route removed; `main.rs` is now the
      composition root only, and the real routes live in
      `infrastructure/http/routes/auth.rs` as `/api/auth/github` and
      `/api/auth/github/callback`
- [ ] `frontend/lib/api-client.ts` — `getWsTicket(sessionId)` must take no argument
      (blocked on the WS-vs-SSE decision)
- [ ] Decide the rate-limit budget for `/api/auth/github` and the callback
- [ ] Decide whether `installations` reacts to `UserAuthenticatedEvent` or is queried
      directly by the callback route (this doc assumes queried)
