# Authentication Architecture — JustAskRepo

## Overview

JustAskRepo uses a **single GitHub App** for both identity and repository access, and
**server-side sessions** for authentication. The **Rust backend (Axum) is the single
source of truth** for all authentication and authorization. The Next.js app only
renders UI and forwards the session cookie — it never sees the session ID, GitHub
tokens, or any auth secret.

**Design stance:** opaque session cookies, no JWTs. This keeps revocation instant,
avoids token-invalidation complexity, and keeps every auth decision on the backend.

**One app, two token types.** Rather than a separate OAuth App for login plus a
GitHub App for repos, a single GitHub App provides:

| Token | Flow | Used for |
| --------------------------------- | --------------------------------------- | -------------------------------- |
| **User access token** (user-to-server) | OAuth 2.0 authorization-code flow  | Login / identity — fetch profile |
| **Installation token** (server-to-server) | App JWT → installation token     | Repository access (clone, index) |

The login flow is the standard OAuth authorization-code flow; only the app
*registration* differs from a plain OAuth App. The user token is used once at login
to fetch the profile and then discarded — the Valkey session is the source of truth,
not the GitHub token.

## Technology Stack

| Component             | Technology                    |
| --------------------- | ----------------------------- |
| Frontend (UI only)    | Next.js                       |
| Backend (auth owner)  | Rust (Axum)                   |
| Identity + repo access| Single GitHub App             |
| Session store         | Valkey (with AOF persistence) |
| Persistent storage    | PostgreSQL                    |
| Auth mechanism        | Opaque session cookie         |

---

## System Architecture

```text
┌─────────────┐   __Host-session cookie    ┌──────────────────────────┐
│   Browser   │ ─────────────────────────► │      Next.js (UI)        │
└─────────────┘                            │  • Login / Logout buttons │
      ▲                                    │  • Forwards cookie only   │
      │                                    └────────────┬─────────────┘
      │ Set-Cookie / 401                          HTTPS │ (credentials)
      │                                                 ▼
      │                          ┌────────────────────────────────────────┐
      └───────────────────────── │          Rust Backend (Axum)           │
                                 │                                        │
                                 │  Auth middleware + Auth module         │
                                 │  • GitHub App user login/callback      │
                                 │  • Installation tokens for repo access │
                                 │  • Session create / validate / revoke  │
                                 │  • Rate limiting, CSRF/origin checks   │
                                 └───────┬───────────┬───────────┬────────┘
                                         ▼           ▼           ▼
                                    PostgreSQL     Valkey     GitHub App
                                   (user records)(sessions) (user + install
                                                              tokens)
```

**Separation of concerns**

- **PostgreSQL** — durable identity. Match key is the immutable `github_id`.
- **Valkey** — ephemeral session state, keyed by session ID, TTL-expired.
- **GitHub App** — user-to-server tokens for login, installation tokens for repos.
- Losing Valkey logs everyone out (fail-closed) but loses no identity data.

---

## Login Flow

```text
User clicks "Continue with GitHub"
      │
      ▼
Next.js → GET /auth/github
      │
      ├─ Backend generates a one-time `state`, stores it in Valkey (short TTL)
      └─ 302 → GitHub App authorize URL (with state)
      │
      ▼
GitHub — user authorizes the App (user-to-server)
      │
      ▼
GET /auth/github/callback?code=…&state=…
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
      └─ Yes → 302 → Next.js dashboard
```

**Authorization vs installation.** For a GitHub App these are separate:

- **Authorization** (user-to-server) proves *who* the user is → enough to create a
  session. A user can be authorized without having installed the App.
- **Installation** grants *repository access* → required before indexing. The backend
  mints installation tokens from the App JWT; user tokens are never used for repo I/O.

The UI must handle the **authorized-but-not-installed** state: the user is logged in
but has no installation yet, so prompt them to install the App on the repos they want
indexed. This is the normal onboarding path, not an error.

**Session fixation defense:** a brand-new session ID is minted at the end of a
successful login. Any pre-login cookie value is discarded, so a planted cookie can
never become an authenticated session.

---

## Authenticated Request Flow

```text
Browser ── Cookie: __Host-session=<id> ──► Rust auth middleware
                                              │
                                              ├─ Read cookie
                                              ├─ For mutating requests, verify Origin/Referer
                                              ├─ Look up session:<id> in Valkey
                                              ├─ Reject if missing or past expires_at
                                              ├─ Refresh idle TTL (sliding window)
                                              ├─ Load user, attach to request context
                                              ▼
                                        Protected route handler
```

Any failure → **`401 Unauthorized`**, no handler runs.

---

## Logout Flow

**Single session**

```text
POST /auth/logout
      │
      ├─ Delete session:<id> from Valkey
      ├─ Remove id from the user's session index
      ├─ Expire the cookie (Max-Age=0)
      ▼
200 OK
```

---

## Data Model

### PostgreSQL — `users` (durable identity only)

| Column        | Description                                  |
| ------------- | -------------------------------------------- |
| id            | Internal user ID (primary key)               |
| github_id     | GitHub user ID — **immutable match key**     |
| username      | GitHub username (immutable, display only)      |
| email         | GitHub email, if available (display only)    |
| avatar_url    | GitHub avatar URL                            |
| created_at    | Account creation                             |
| updated_at    | Last profile update                          |
| last_login_at | Last successful login (audit/support)        |

> Users are matched **only on `github_id`**. Never match on `email` or `username` —
> both are mutable and reusable, and matching on them enables account takeover.

### Valkey — sessions (ephemeral)

**Session record** — `session:<session_id>`

```json
{
  "user_id": 123,
  "github_id": 987654,
  "session_version": 1,
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

**Expiry model**

- **Valkey TTL is authoritative** for whether a session exists.
- `expires_at` enforces a hard **30-day absolute** cap.
- `last_seen_at` + TTL refresh give a **sliding idle timeout** — active sessions stay
  alive; idle ones expire early.

> GitHub access/installation tokens are **never stored in the session record.**
> See Token Handling below.

---

## Session Cookie

```
__Host-session=<random_session_id>
```

| Attribute  | Value    | Reason                                              |
| ---------- | -------- | --------------------------------------------------- |
| `__Host-`  | prefix   | Locks cookie to Secure + Path=/ + no Domain         |
| HttpOnly   | on       | JavaScript cannot read it                           |
| Secure     | on       | HTTPS only                                           |
| SameSite   | Lax      | Blocks cross-site cookie sends on unsafe methods    |
| Path       | /        | Sent to all backend routes                          |
| Max-Age    | 30 days  | Matches absolute session cap                         |

The frontend never has access to the session ID. To know if the user is signed in,
the UI calls **`GET /api/me`**, which returns the authenticated profile or `401`.

---

## Protected Routes

Every business route runs the auth middleware first:

```
GET  /api/me
GET  /api/projects
GET  /api/repositories
POST /api/chat
POST /api/search
```

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

| Threat                     | Control                                                        |
| -------------------------- | ------------------------------------------------------------- |
| Login CSRF                 | One-time `state` in Valkey, verified and deleted on callback  |
| API CSRF                   | `SameSite=Lax` **plus** Origin/Referer check on mutating routes|
| Session fixation           | Fresh session ID minted on every successful login             |
| Session theft (detection)  | `ip` / `user_agent` recorded; anomalies can force re-auth     |
| Bulk revocation            | `session_version` bump + per-user session index               |
| Session ID guessing        | Cryptographically secure random IDs                           |
| Brute force / abuse        | Valkey rate limiting on `/auth/github`, callback, and APIs    |
| Token leakage              | GitHub tokens not stored; encrypted only if ever persisted    |
| Transport                  | HTTPS required in production                                   |

### CORS

Next.js and the Rust backend are separate origins, so cross-origin cookie auth
requires:

- `Access-Control-Allow-Credentials: true`
- `Access-Control-Allow-Origin` set to a **strict allow-list** (never `*` with credentials)
- Preflight handling for the mutating API routes

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
- **Natural fit for Valkey** — TTLs, rate limits, and the OAuth `state` all live in
  one fast store.

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
