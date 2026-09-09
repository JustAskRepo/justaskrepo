# ADR-009: Repository Authorization from Login-Time Reconciliation

**Date:** 2026-09-09
**Status:** Accepted
**Deciders:** Arya Sharma

---

## Context

Logging in establishes *who someone is*. It establishes nothing about *what they may see*.
A GitHub identity grants no access to a single line of anyone's code; access comes from a
separate, deliberate act — the user installs the App on some repositories and GitHub creates
an **installation**, a scoped grant saying *this app may read these repos*.

The `installations` module owns that grant, and exists to answer one question correctly for
the rest of the system:

> Which repositories may this user see, and by what authority may we read them?

The two halves of that question have different answers, and the split runs through
everything below. *By what authority may we read them* is a server-to-server matter: an
installation access token, minted from the App JWT, never involving a human. *Which
repositories may this user see* is a statement about a person, and GitHub's only
authoritative answer to it is `GET /user/installations` — which requires a **user access
token**.

That is the collision. ADR-008 decision 3 spends the user token once at login and discards
it, precisely so that no user credential is ever stored. At setup-return time, and at every
request afterwards, there is no user token to ask with. So the module cannot simply query
GitHub for authorization; it has to decide where authorization comes from instead, and that
decision is upstream of every other choice in the module.

Three options were real, with genuinely different security and staleness properties. This
ADR records the choice and the four decisions that fall out of it.

## Decision

**1. User authorization is reconciled at login and stored. The user token is spent, never
kept.**

The login callback already holds a live user token for the few milliseconds before ADR-008
discards it. It is spent there: `GET /user/installations` (and per-installation
`GET /user/installations/{id}/repositories`), and the resulting **link** — this user may
see this installation — is written to Postgres. Later requests read the stored link and
need no token. No credential is retained.

**2. The link is established by the composition root calling `installations`, not by a
subscription to `UserAuthenticatedEvent`.**

`infrastructure/http/routes/auth.rs` calls `handle_complete_github_login`, then
`handle_link_user_installations`, then `handle_get_installation_status` to pick the
redirect. This closes the open item in `AUTHENTICATION.md` that assumed a query.

`CompleteGithubLoginResponse` grows `user_id` and `user_token: SecretString` as a
consequence — a change to `auth`'s public contract.

**3. The setup URL grants nothing. It is a navigation hint, not a writer.**

`GET /api/installations/setup` performs no writes and links no records. It redirects to the
dashboard. The two authoritative writers are the `installation.*` webhooks and login
reconciliation.

**4. The repository set is stored in Postgres, refreshed by webhook *and* by a throttled
check on the read path — and it is two sets, not one.**

*What a user may see* (`user_repositories`, written by login reconciliation from
`GET /user/installations/{id}/repositories`) is distinct from *what an installation covers*
(`installation_repositories`, written by webhooks from `GET /installation/repositories`).
Authorization reads the first. Conflating them over-grants: an organization installation can
cover fifty repositories while a given member may access three, and authorizing from the
installation's set would show that member the other forty-seven.

Webhook deltas are never applied in arrival order; a delta triggers a re-fetch of
`GET /installation/repositories` and a wholesale replacement of the set. Login reconciliation
replaces its own set the same way, for the same reason.

**5. Installation access tokens are minted on demand, cached in Valkey below their real
expiry, and never written to Postgres or held in process.**

Exposed to other modules as `GetInstallationTokenQuery` on `api.rs`, returning the token and
its expiry.

## Rationale

**Why reconcile at login (1).** The three options were:

| | Cost | Staleness |
| --- | --- | --- |
| (a) Persist the user token, encrypted | Envelope encryption, key rotation, and — because the token lives ~8 hours — the refresh-token flow too. Revises ADR-008 §3 | None. Authorization always asks GitHub |
| (b) Reconcile at login, store the link | One extra GitHub call inside a request that is already talking to GitHub | Bounded by the user's next login |
| (c) Infer from `installation.account.id` | None | None |

**(c) is disqualified, not merely worse.** For an organization installation the account
*is* the org, and its id matches no user's `github_id`, so nobody is ever linked. It fails
closed, which means the product does not work for orgs — the case that matters most. The
obvious patch, "if the account is an org, link every member", needs the org membership API,
which needs a user token, which is option (b) wearing a disguise.

**(a) is the most correct option and it is the one to grow into, not to start with.**
ADR-008 anticipated it in as many words: a retained user token needs envelope encryption in
Postgres and a revision of that ADR, not a field on a session record. That is real work —
key management, rotation, and a refresh flow that does not currently exist — spent to close
a staleness window that (b) already bounds.

**(b) stores no credential.** It stores a *fact* derived from one: that GitHub, at a moment
when it was willing to answer, said this user could see this installation. The credential
never leaves the request that fetched it. That is the property worth protecting, and it is
why (b) does not revise ADR-008 §3 — it honours it. The token is used once, for one more
purpose than before, and dropped in the same function.

**The read path is responsible for its own freshness.** `GET /api/repositories` refreshes
the caller's installations before listing them, throttled to one GitHub request per
installation per minute. Webhook delivery is a notification, not a guarantee — it can be
missed, delayed, or (in development) never arrive — and a dashboard that lists a repository
someone has removed is an over-grant, not a cosmetic lag. So the read does not trust that a
notification arrived.

What that refresh can derive without a user access token, and what it cannot:

| | Derivable from installation-scoped data? | Why |
| --- | --- | --- |
| **Removal**, any installation | **Yes** | The installation's set is an *upper bound* on what any member reaches through it. A repository that has left is reachable by nobody through it. |
| **Addition**, personal installation | **Yes** | The account *is* the user. A personal installation covers only that account's repositories, and the owner reaches all of them, so the installation's set is exactly their set. |
| **Addition**, organization installation | **No** | Whether a given member sees a repository is answered only by `GET /user/installations/{id}/repositories`. No upper bound helps: the installation set is strictly wider than the member's. |

So a personal installation tracks GitHub in both directions within the throttle window. An
organization installation loses access immediately and gains it at the member's next login.
The asymmetry is deliberate: a missing repository is an inconvenience, a lingering one is an
over-grant.

The one thing still login-stale in the dangerous direction is a user's access to an
installation that *still exists* — someone removed from an organization. Nothing
installation-scoped can detect that, and §Consequences states it plainly rather than filing
it under "eventual consistency".

> **Corrected twice, 2026-09-09.** This paragraph first claimed the repository set was
> webhook-refreshed and near-real-time; that was true of `installation_repositories` and
> false of `user_repositories`, which is the table authorization reads — an error introduced
> when the two were split and not caught until a removed repository was seen still listed on
> the dashboard. Pruning fixed that direction. It was then found that additions had the same
> problem in reverse, which the personal-installation case above resolves. The lesson worth
> keeping: after splitting a table, re-read every claim that justified the old shape.

**Why the composition root and not a subscription (2).** Decision 1 forces it. Events carry
IDs and never credentials — ARCHITECTURE.md §6 says so, and `UserSessionsRevokedEvent`
omits `SessionId` for exactly this reason. A subscriber to `UserAuthenticatedEvent` would
arrive holding a user id and no token, and would have to fall back to (a) or (c) to do its
job. The event bus cannot carry the one thing this flow needs.

The bus is also the wrong delivery guarantee here. It is at-most-once and in-memory: a
process death between the login and the handler would leave a user signed in and staring at
an empty dashboard with nothing to retry. Inside the request, the failure is synchronous,
visible, and self-healing on the next login.

ARCHITECTURE.md §5.1 already blesses this shape and names the login callback as its
canonical example. The composition root may call several modules; a module may not.

**Why the setup URL grants nothing (3).** GitHub documents that `installation_id` on the
setup redirect is attacker-supplied. A handler that links the current session to whatever id
it is handed is a one-request account-linking vulnerability — hand the victim's browser your
setup URL with your installation id, and their session is now attached to your grant.

The documented defence is to verify the id against `GET /user/installations`, which needs a
user token this design deliberately does not have at that moment. Rather than build a check
we cannot actually perform, the endpoint performs no writes at all. An unverifiable claim is
not weakly trusted; it is ignored.

**Why two repository sets (4).** This was not obvious from the endpoint names and is the
correctness trap in the whole module. GitHub exposes two listings that differ only by
credential — the user token answers "which of these repositories may *this human* reach",
the installation token answers "which repositories may *the app* reach" — and they return
different sets for any organization that scopes access per member. One table holding both
answers is a table that is wrong for one of them. Two tables with one writer each cannot
drift, and it costs a foreign key and an index.

The consequence worth stating: `installation_repositories` stays empty until webhooks land,
and nothing reads it before then. The dashboard is served entirely from `user_repositories`,
which is the set it should have been showing regardless.

**Why store the repository set (4).** The alternative — fetching from GitHub on every
dashboard load — is never stale, and puts GitHub's availability and rate limit on the
critical path of the application's front door. `GET /installation/repositories` is
per-installation and paginated, so a user with several installations costs several round
trips per page render. Storing makes it one Postgres query, survives a GitHub outage, and
gives `indexing` a stable repository id to key its work on.

The re-fetch-rather-than-apply-deltas rule is not caution, it is correctness. Webhook
delivery is at-least-once and unordered: the same `installation_repositories` event can
arrive twice, and a `.removed` can overtake the `.added` it followed. State computed by
replaying deltas in arrival order is wrong in a way that is invisible until someone notices
a repo that should not be there. Re-fetching is slower and immune to the entire class.
`repository_selection: all` makes it mandatory anyway — GitHub does not emit a delta for
every repository newly created under that setting.

**Why Valkey for the token cache (5).** Minting on every call is correct and wasteful: a
token good for an hour, discarded after one request, against a rate-limited endpoint.
In-process caching is simpler and defensible while there is one instance — and it is exactly
the assumption that breaks on the extraction path. ARCHITECTURE.md §10 names `indexing` as
the first module likely to move out, and `indexing` is the heaviest consumer of these
tokens. A cache that lives in this process is the thing that has to be rebuilt the day that
happens.

Valkey costs nothing new: ADR-008 already accepts it as a hard dependency of every
authenticated request, so this adds no coupling that is not already there, and its TTLs do
the expiry without a sweeper. Postgres is ruled out for the opposite reason — durable
storage of an hour-long bearer credential buys nothing and creates a secret at rest.

The cache TTL sits meaningfully below the token's real hour so that a token handed out is
never near death; `GetInstallationTokenQuery` returns the expiry so a long indexing run can
re-ask rather than discover the expiry mid-job. It is a Query on `api.rs` rather than a
shared helper because architecture rule 1 forbids `indexing` from reaching into this
module's `infrastructure/`, and that door is cheaper to design now than to retrofit with
`indexing` half-written.

## Consequences

- **Positive:** no user credential is retained anywhere in the system. ADR-008 decision 3
  stands unrevised, and there is still no encrypted-token column, no key rotation, and no
  refresh flow to get wrong.
- **Positive:** authorization is a Postgres join, not a GitHub call. The dashboard renders
  when GitHub is down, and it costs no third-party rate limit.
- **Positive:** correct for organization installations, which the free option (c) is not.
- **Positive:** one write path per fact, each idempotent — webhooks own the repository set,
  login owns the user link. Neither has to know whether it ran first.
- **Negative:** **a user's access to an installation is up to one session stale.** Someone
  removed from an organization keeps seeing its repositories until their next login, bounded
  by the 30-day absolute session cap in ADR-008 rather than by anything this module does.
  Uninstall is covered — that arrives as a webhook and publishes `RepoUninstalledEvent` —
  but an organization membership change is not, because GitHub does not tell the App about
  it. This is the price of (b) and the trigger for (a).
- **Negative:** the setup redirect and the `installation.created` webhook leave GitHub at
  the same moment with no ordering, and decision 3 forbids the redirect from resolving the
  race. A user can land on an empty dashboard seconds after installing.
- **Negative:** `auth`'s public contract now carries a live bearer credential in
  `CompleteGithubLoginResponse`. Nothing stores it, but it exists in a struct that crosses
  the composition root, and that struct derives `Debug`.
- **Negative:** decision 4 means the repository set is only as fresh as webhook delivery, and
  the two halves recover differently. A missed webhook leaves `user_repositories` stale only
  until that user's next login, which rewrites it wholesale. It leaves the `installations`
  row and `installation_repositories` stale **permanently**: login reconciliation upserts
  installation records and unlinks the user, but never deletes an installation, because one
  legitimately exists with nobody linked to it (someone can install the App and never sign
  in) — so "no links" is not evidence of death. Only GitHub knows, and only the
  `installation.deleted` webhook currently asks. A missed one is an orphaned row with no
  authorization consequence and no cleanup path; the durable fix is a periodic reconcile
  against `GET /app/installations`, which is a scheduled job rather than something login can
  safely infer.
- **Negative:** installing the App does not populate the dashboard on its own. The webhook
  writes `installation_repositories` and the dashboard reads `user_repositories`, so the new
  grant is invisible until a user access token exists to link it. That is why the Setup URL
  redirects through `/api/auth/github` rather than to the dashboard — the silent
  re-authorization is what produces the token. Without that hop the flow appears to succeed
  and shows nothing, which is how this was found.
- **Negative:** the same repository's name and visibility are stored in two tables written by
  two sources. They can disagree between a webhook and a login. Nothing joins them, so the
  disagreement is invisible rather than incorrect — but a future reader who joins them will
  find it.
- **Mitigation:** for both staleness and the setup race, the cheap fix short of option (a)
  is to obtain a fresh user token on demand by bouncing through `/api/auth/github` — for an
  already-authorized App this is a redirect rather than a consent screen. Worth reaching for
  if the race proves visible or a "refresh my repositories" action is asked for; not worth
  building before either happens.
- **Mitigation:** `user_token` is `SecretString`, not `String`, so a `tracing` call that
  formats the response prints `[REDACTED]`. Reviewers should treat any widening of that type
  as a contract change.
- **Mitigation:** linking failure must not fail the login. A user who is signed in but sees
  no repositories can retry; a user who cannot sign in because GitHub's installations
  endpoint was briefly unavailable has a much worse day.
- **Mitigation:** the idempotency claims are the module's highest-value tests — apply the
  same webhook twice and assert the second is a no-op. That is where the bugs will be.

## Review Trigger

Revisit when any of these happens:

- **One-session-stale authorization stops being good enough** — a compliance requirement, or
  an incident where a removed organization member kept access. That is the trigger for
  option (a), and it brings envelope encryption in Postgres, the refresh-token flow, and a
  revision of ADR-008 decision 3 with it.
- **`indexing` moves out of this process.** The Valkey token cache is already shaped for
  that; what needs rechecking is whether anything has grown an in-process assumption around
  it since.
- **The setup-redirect race becomes a real support burden** rather than a theoretical one.
  The mitigation above is the answer, and it is a small change.
- **Orphaned installation rows start to matter** — most likely once `indexing` reads
  `installation_repositories`, where a dead installation means a job whose token mint 404s.
  That is the trigger for the periodic `GET /app/installations` reconcile named above.
- **A second identity source appears** — a user who is not a GitHub user. The whole
  user-to-installation link assumes exactly one GitHub identity per user row.
- **GitHub adds an app-level way to ask which users may see an installation.** That removes
  the need for a user token entirely and makes this ADR obsolete rather than upgraded.
