# ADR-007: Rate Limiting as an HTTP Layer over a Valkey Counter

**Date:** 2026-08-29
**Status:** Accepted
**Deciders:** Arya Sharma

---

## Context

`AUTHENTICATION.md` §Security Controls has listed "Brute force / abuse → Valkey rate
limiting on the auth routes and APIs" as an existing control since the auth design was
written. No such thing existed: no middleware, no counter, no dependency. This is the
same doc-promises-what-code-does-not shape as the `Origin` gap that ARCHITECTURE.md §5.2
closed, and it is a gap rather than a decision.

The two login routes are cheap to abuse and reachable without a session:

- `GET /api/auth/github` mints a Valkey `state` key with a 10-minute TTL per hit.
- `GET /api/auth/github/callback` consumes a `state` key, and on a valid one spends two
  outbound GitHub calls (code exchange, then profile fetch).

Four questions had real alternatives, and the answers are what this ADR records.

## Decision

**1. It is composition-root HTTP machinery, not a module.**

The counter lives in `infrastructure/rate_limiter.rs` beside `db.rs` and `valkey.rs`; the
axum adapter lives in `infrastructure/http/middleware/rate_limit.rs` beside
`require_session.rs`.

**2. Fixed window** — `INCR`, `EXPIRE … NX`, `TTL`, pipelined in one `MULTI`. `NX` is
load-bearing: it sets the TTL only on the key `INCR` just created.

**3. Fail open.** `rate_limiter::check` returns a `RateLimitDecision`, never a `Result` —
it is not able to deny service because of its own failure. A Valkey error is logged at
`error!` and the request proceeds.

**4. The subject is a trusted-proxy-aware client address**, resolved by
`infrastructure/http/client_ip.rs` from `TRUSTED_PROXY_HOPS` (default `0`).

The budget is `AUTH_RATE_LIMIT_MAX_REQUESTS` per `AUTH_RATE_LIMIT_WINDOW_SECS`,
defaulting to **20 per 60 seconds**, shared by both halves of the handshake.

## Rationale

**Why not a module.** A module owns data and enforces business rules about it. A rate
limit counter is neither: nothing queries it, nothing reacts to it, and it holds no state
worth a domain event. Routing every limited request through a CQRS Command to increment
an integer would be ADR-002 ceremony protecting nothing — and it would put the limiter
*inside* one module while chat, indexing and the WebSocket ticket all need the same
counter. The split between `rate_limiter.rs` and the middleware follows §5.2's existing
rule: a budget is not an HTTP concern, but turning a refusal into `429` is.

**Why fixed window.** A caller straddling a boundary can spend up to 2× the budget across
two adjacent windows — 40 requests instead of 20. That is irrelevant against the abuse
this protects, and it costs one integer key per subject instead of one sorted-set member
per request, as a sliding log would. Revisit only if a budget gets small enough for the
doubling to matter.

**Why `EXPIRE … NX`.** A plain `EXPIRE` pushes the deadline out on every hit, so a client
that keeps retrying never sees its window close: blocked until it stops asking, which is
neither a fixed window nor what the `Retry-After` we sent it promised.

**Why fail open.** A limiter is an availability protection; one that takes the app down
with Valkey has become the outage it exists to prevent. The exposure this buys is nil
here in any case — sessions and the OAuth `state` key both live in Valkey, so a Valkey
outage already means nobody can log in, and there is nothing left to brute-force. Note
that this does not contradict "a Valkey outage is fail-closed" in `.env.example`: session
*validation* still fails closed at 401. Only the limiter fails open.

**Why the IP needs a trust model.** `X-Forwarded-For` is client-writable. Keying on it
naively is not a weak limiter, it is no limiter — an attacker rotates the header per
request and never spends a budget. The route code already knew this: `routes/auth.rs`
carried the comment *"client-spoofable … if a route ever needs a trustworthy IP, this
must become a trusted-proxy-aware parse first."* This is that route.

Treat `[forwarded…, peer]` as the observed chain. Each trusted proxy appended exactly one
entry, so stepping `TRUSTED_PROXY_HOPS` back from the end lands on what our outermost
trusted proxy actually saw; a forged prefix is simply never reached. How many proxies are
real is a fact about the deployment, not the request, so it is configuration. It defaults
to `0` — ignore the header, use the socket peer — because guessing wrong in that
direction costs accuracy, while guessing wrong in the other direction hands out an
unlimited budget. Too *high* a value falls back to the peer rather than to the forgeable
end of the chain, for the same reason.

The callback's audit `ip` now uses the same resolution. An audit field the subject of the
audit controls is worse than no field.

**Why 20/60s, shared.** A real login is two requests, so this is ten logins a minute from
one address — generous for a human fumbling the flow, and it caps abuse of the two
endpoints at 1200/hour. Splitting the budget per route doubles the configuration surface
to defend against an attacker who could just spend the other half instead. `0` is
rejected at boot: it would refuse every login, so it is a typo rather than an off switch.

## Consequences

- **Positive:** the control `AUTHENTICATION.md` has claimed since day one now exists, and
  the trusted-IP parse it needed closes a second documented gap along with it.
- **Positive:** the layer is mountable anywhere with a different policy. Chat and indexing
  get a budget by adding a field to `RateLimits` and one `route_layer` line.
- **Positive:** `429` needed no frontend work — `lib/errors.ts` already describes a
  `rateLimit` kind, and `Retry-After` rides along on the response.
- **Negative:** callers behind one NAT share a bucket. Ten logins a minute is comfortable
  for an office of tens, but it is a real ceiling, and the affected users see a `429` they
  did nothing to earn.
- **Negative:** a `429` on `/api/auth/github` renders as raw JSON, because that route is a
  browser navigation rather than a `fetch`. Correct, and ugly.
- **Negative:** the default of `0` hops means a deployment that *is* behind a proxy limits
  the proxy rather than the client until someone sets the variable — one shared bucket for
  everybody.
- **Mitigation:** the durable answer to the NAT ceiling is a per-account budget, which is
  only available *after* identity is known and so cannot cover the handshake by
  definition. If the shared bucket bites, raise the budget rather than reaching for the
  header.
- **Mitigation:** a misconfigured hop count is visible rather than silent — every session
  row shows the same `ip`, and the limiter's `warn!` names the subject it bucketed.

## Review Trigger

Revisit when any of these happens:

- A budget gets small enough (single digits per window) that the fixed-window 2× boundary
  burst stops being acceptable — switch that policy to a sliding window counter.
- A route needs a *per-account* rather than per-address budget. The policy type takes an
  arbitrary subject string already; what is missing is a decision about which identity to
  key on, and that belongs in a new ADR.
- Valkey stops being the right store for counters — for example if the process is
  horizontally scaled behind a load balancer *and* the counter becomes hot enough to
  matter, at which point per-node token buckets are the cheaper shape.
- Rate limiting is wanted on a route that is not cookie-authenticated (the public
  webhook), where the client address is the *only* subject available and a shared-NAT
  ceiling is much more likely to bite.
