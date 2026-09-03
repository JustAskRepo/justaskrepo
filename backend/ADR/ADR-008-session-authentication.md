# ADR-008: Opaque Valkey Sessions for GitHub App Authentication

**Date:** 2026-09-03
**Status:** Accepted
**Deciders:** Arya Sharma

---

## Context

`AUTHENTICATION.md` has described this design in full since before any of it existed, and
the `auth` module has now shipped against it: the OAuth handshake, the session store, both
revocation paths, `/api/me`, and the session middleware. That document is the operational
reference — what the system does, which key holds what, which attribute the cookie carries.

What has never been written down is which of those were *choices*. Several had a real
alternative that a future change would otherwise re-open by accident, and two are named in
`CLAUDE.md` as ADR-worthy by construction: sessions-in-Valkey versus JWTs, and auth as a
module versus auth as middleware. `AUTHENTICATION.md` has also referenced "ADR-008" by
number since August without one existing.

This is that record. Where it and `AUTHENTICATION.md` overlap, this file says *why* and
that file says *what*.

## Decision

**1. Opaque server-side sessions in Valkey. No JWT anywhere in the system.**

The cookie carries a random 256-bit identifier and nothing else. Every claim about the
caller is read from `session:<id>` on the request that needs it.

**2. `auth` is a module; the cookie, the middleware, and the routes are not.**

`modules/auth/` owns identity, sessions, and the GitHub OAuth client, and does not compile
against axum. `infrastructure/http/routes/auth.rs` builds the `Set-Cookie`,
`infrastructure/http/middleware/require_session.rs` turns a cookie into a `CurrentUser`,
and both reach the module the same way every other caller does — through `api.rs`.

**3. One GitHub App issues both token types. The user token is used once and dropped.**

Login is the standard authorization-code flow against the App; the user-to-server token
fetches the profile and is discarded in the same function. Repository access uses
installation tokens minted from the App JWT. Neither is ever stored.

**4. Two clocks. Valkey's TTL is the idle timeout; `expires_at` in the record is the
30-day absolute cap, checked in code on every load.**

The write-back that slides the idle TTL is throttled by `refresh_threshold` (5 minutes) —
a read-only request inside that window costs one `GET` and no write.

**5. Bulk revocation is a per-user session index, `user_sessions:<user_id>`. There is no
`session_version`.**

`delete_all_sessions` reads the index, deletes what it names, and `SREM`s exactly those
ids — never `DEL` of the index key.

**6. `__Host-` cookie + `SameSite=Lax` + an `Origin` check on unsafe methods, inside the
session middleware. No CSRF token.**

**7. Streaming is SSE over the ordinary cookie-authenticated `/api/*` surface. No
WebSocket, and therefore no single-use ticket.**

This supersedes the WebSocket-ticket mechanism `AUTHENTICATION.md` carried until today.

## Rationale

**Why opaque sessions (1).** A JWT moves the authority for "is this caller still allowed
in" from the server to a signature the server already made. Everything the design wants
back from that trade is expensive: revocation becomes a denylist, which is the session
store again with worse ergonomics; logout-everywhere becomes a `session_version` claim,
which needs an authoritative counterpart to compare against and therefore a read per
request; and a stolen token stays valid for its lifetime no matter what we learn about it.
The property that made JWTs attractive — validating without touching a store — buys
nothing here, because Valkey is already on the path for rate limits and the OAuth `state`,
and every session read is a single `GET` on a warm connection. What we get instead is that
deleting a key *is* the revocation.

**Why a module with the HTTP half outside it (2).** Auth is not a cross-cutting concern
wearing a module costume: it owns durable data (the `users` table), enforces rules about it
(the absolute cap, session fixation, single-use `state`), and other modules will need to
ask it questions. That is a module by ADR-001's definition. But a session cookie is an HTTP
transport detail, and `require_session` is axum-shaped by nature — putting either inside
`modules/auth/` would make the module unusable from a background job, a CLI, or the
extraction path in ARCHITECTURE.md §10. The split falls exactly where ARCHITECTURE.md §5.2
already puts it: deciding whether a session is valid is the module's job, turning that
decision into a `401` is the composition root's.

**Why one App (3).** A separate OAuth App for login plus a GitHub App for repositories
means two consent screens, two registrations, and two token models to keep straight, in
exchange for nothing — user-to-server auth on a GitHub App *is* the OAuth
authorization-code flow. Discarding the user token after the profile fetch is what keeps
this simple: no refresh-token rotation, no encrypted token column, no expiry to track. If
a feature ever needs a retained user token, it needs envelope encryption in Postgres and a
revision of this ADR, not a field on the session record.

**Why two clocks (4).** They answer different questions and neither can answer both. A
Valkey TTL that slides on activity is the only cheap way to expire idle sessions, but by
construction it can be pushed forward forever — it can never enforce "dead 30 days after
login". A single absolute TTL with no sliding would log active users out mid-session on a
fixed schedule. So the sliding one lives in Valkey where expiry is free, and the absolute
one lives in the record where the code can check it. The throttle exists because the
alternative is a Valkey write on every authenticated request, which is a write amplification
of the entire read path for a field nothing reads in real time.

**Why an index rather than a version (5).** A `session_version` on `users` revokes nothing
unless every request compares against it, and that comparison is a Postgres read per
request — the exact cost the session design exists to avoid. The index makes
logout-everywhere one `SMEMBERS` plus one pipeline, bounded by the user's own session
count instead of the keyspace. `SCAN` over `session:*` was the third option and is
disqualified at any real size.

The `SREM`-not-`DEL` detail is the part worth protecting: a login landing between the read
and the write adds its id to the same set, and dropping the key wholesale would unindex
that brand-new session while leaving its record alive — a working credential that no future
revoke-all can see. Removing only the ids we read leaves the newcomer indexed.

**Why no CSRF token (6).** The UI and the API share an origin, so `SameSite=Lax` already
withholds the cookie from cross-site sub-resource requests. The remaining gap is the
top-level cross-site navigation, which is always a `GET` — so an `Origin` check on unsafe
methods closes it exactly, using a header the browser sets itself and page script cannot
forge. A CSRF token would add a second secret, a rotation story, and a failure mode
(expired token on a long-open tab) to defend against what those two already cover. The
check lives inside the session middleware rather than on the router because cookie
authentication is what makes CSRF possible in the first place: a route that authenticates
some other way has nothing to defend, and one that authenticates this way must never be
able to opt out.

**Why SSE, and why that deletes the ticket (7).** The ticket existed for one reason: a
WebSocket handshake does not reliably carry cookies across contexts, and it cannot be
refused with a useful body — so authentication had to be smuggled through a query
parameter, which meant minting a second short-lived credential, storing it, expiring it,
and accepting that it lands in access logs.

SSE is an ordinary `GET`. Same-origin means the session cookie rides along untouched,
`require_session` runs on it exactly as it runs on `/api/me`, and a rejection is a plain
`401` with a body the client can read. The entire ticket mechanism — endpoint, Valkey key
type, TTL, single-use semantics, and its share of the rate-limit budget — stops existing
rather than being implemented.

What SSE gives up is client-to-server streaming, which chat does not need: a question is
one request and the answer is the stream. The message goes up as a `POST` (covered by the
`Origin` check) and the reply comes back on a `GET` stream. A cross-site page cannot open
that stream against us twice over — `SameSite=Lax` withholds the cookie from a cross-site
`EventSource`, and with no CORS headers the browser refuses the response regardless.

Two consequences come with it, both acceptable. SSE reconnects on its own, and each
reconnect is a fresh cookie-authenticated request — so an expired session ends the stream
with a `401` instead of a socket that silently stops producing. And a browser allows about
six concurrent HTTP/1.1 connections per origin, so a long-lived stream per tab can starve
other requests on HTTP/1.1; under HTTP/2 the streams are multiplexed and it is a non-issue.

## Consequences

- **Positive:** revocation is instant and total, and it is the same operation in all four
  cases — logout, logout-everywhere, uninstall, and account deletion all end at "delete the
  key".
- **Positive:** no token-invalidation machinery exists to get wrong. No refresh rotation,
  no denylist, no clock skew, no key rollover.
- **Positive:** additional identity providers slot in behind the same session model without
  touching the cookie, the middleware, or logout — only the callback that mints the session
  is provider-shaped.
- **Positive:** dropping the ticket removes a whole credential type from the system, and
  with it the endpoint, the Valkey key, and the "ticket in a URL in a log" exposure.
- **Negative:** Valkey is on the critical path for every authenticated request, and a
  Valkey outage logs everyone out. This is fail-closed on purpose (the rate limiter is the
  one deliberate exception, ADR-007), but it is a hard availability coupling.
- **Negative:** sessions are process-external state, so the backend is only as horizontally
  scalable as its Valkey. A JWT design would have had no such coupling.
- **Negative:** the absolute cap costs a comparison on every session load that the TTL
  cannot do for us, and the throttle means `last_seen_at` is accurate only to within
  `refresh_threshold`. Anything wanting exact last-seen data needs a different field.
- **Mitigation:** run Valkey with AOF persistence so a restart does not log the world out.
  A lost Valkey costs sessions and nothing else — identity lives in Postgres and the worst
  case is that everyone signs in again.
- **Mitigation:** the invariants that are easy to break silently now have tests —
  the two clocks and the throttle as unit tests in `domain/session.rs`, and the
  `SREM`-not-`DEL` race as an `#[ignore]`d probe in `infrastructure/session_store.rs`.

## Review Trigger

Revisit when any of these happens:

- A second backend service needs to authenticate the same user without calling this one.
  That is the first real argument for a signed token, and it is an argument for a *short*
  internal one issued from a session — not for putting a JWT in the browser.
- Chat needs the client to stream *to* the server (live audio, collaborative editing).
  Decision 7 is scoped to request/response with a streamed reply; a duplex requirement
  reopens WebSocket and brings the ticket back with it.
- A user token has to be retained (acting on a user's behalf outside an installation's
  scope). That needs envelope encryption in Postgres and revises decision 3.
- The session read becomes hot enough to matter, at which point the answer is a short
  in-process cache with a TTL below `refresh_threshold` — and the cost is that revocation
  stops being instant, which is the property this whole ADR is built on.
- The UI stops being served from the API's origin. That invalidates decision 6 entirely and
  must be settled *before* such a change ships.
