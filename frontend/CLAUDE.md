@AGENTS.md

# JustAskRepo Frontend — Structure (enforced)

> The frontend is a **static export** (`output: 'export'`) — a pure client-side SPA.
> There is **no Next.js server**. Axum serves the exported `out/` and owns every bit of
> server logic (auth, indexing, retrieval, chat streaming). The browser talks to Axum
> **directly**, same-origin, under `/api/*`. This structure MUST be maintained.
> The `@/*` import alias maps to the `frontend/` root (see `tsconfig.json`).

```text
frontend/
  next.config.ts                     # output:'export', images.unoptimized, trailingSlash
  app/
    layout.tsx                       # root layout (static)
    page.tsx                         # marketing landing ("/") — static
    (auth)/login/page.tsx            # GitHub OAuth entry — static
    dashboard/page.tsx               # "use client": fetches + lists repos
    repos/page.tsx                   # "use client": repo detail, reads ?id=
    repos/chat/page.tsx              # "use client": chat island, reads ?id=
  components/                        # dumb, reusable presentational components
  features/                          # feature-scoped UI + logic
    repos/{RepoList,RepoStatusBadge,IndexButton}.tsx
    chat/{ChatWindow.tsx,useChatSocket.ts}
  lib/                               # client-only helpers
    api-client.ts                    # typed fetch -> Axum /api/*, + auth/ws URLs
    format.ts
  types/
    api.ts                           # shared DTOs mirroring the Axum API
```

## The hard constraint: zero Next server features

Static export forbids all of these, and the build errors if you add one. If you reach
for any of them, you've drifted from the plan — the answer lives in Axum, not here:

- **No** `app/api/*` route handlers, **no** server actions, **no** `middleware.ts`/`proxy.ts`.
- **No** Server Components that fetch per-request data, **no** SSR/ISR, **no** `server-only` modules.
- **No** NextAuth / BFF / internal-JWT layer. Auth is a same-origin session cookie set by Axum.
- **No** `next/image` optimizer (`images.unoptimized: true` is required), **no** dynamic route
  segments for runtime-unknown ids (see rule 4).

## Rules

1. **Browser → Axum directly.** All data access goes through `lib/api-client.ts`, which hits
   relative `/api/*` on the same origin with `credentials: "include"`. No API base URL env var —
   same-origin relative paths are frozen-safe and need no `NEXT_PUBLIC_*`. The chat WebSocket
   opens to Axum via `chatSocketUrl(ticket)` using a single-use ticket from `/api/chat/ws-ticket`.
2. **Auth belongs to Axum.** Sign-in is a full-page navigation to `githubLoginUrl()`
   (`/api/auth/github`). Axum runs the OAuth dance and sets an httpOnly session cookie;
   the browser sends it automatically on every `/api/*` call. The frontend never sees a token,
   mints nothing, and reads no secret.
3. **Where things go:**
   - Reusable, presentational, feature-agnostic UI → `components/`.
   - Feature-scoped components, hooks, and client logic → `features/<feature>/`.
   - Browser fetch calls + auth/ws URL builders → `lib/api-client.ts` (hits `/api/*` only).
   - Shared wire/DTO types → `types/api.ts` (keep in sync with the Rust backend).
   - Pure formatting/util helpers → `lib/`.
4. **Runtime-dynamic ids travel as query params, not route segments.** Export pre-renders one
   HTML file per route at build time; a `[id]` segment would require `generateStaticParams` over
   ids we can't know then. So repo detail/chat are `repos/page.tsx` + `repos/chat/page.tsx` read
   via `useSearchParams().get("id")` — wrapped in `<Suspense>` (required for `useSearchParams` in
   export). Link as `/repos?id=…` and `/repos/chat?id=…`.
5. **Client by default here — but keep pages thin.** Pages that read data or params are
   `"use client"`. Static content (landing, login, layout) stays a Server Component so it's
   pre-rendered with no JS shipped. Push `"use client"` to the smallest island that needs it.
6. **Deep-link/refresh recovery is the backend's job.** `trailingSlash: true` emits
   `route/index.html`; Axum's `ServeDir` falls back to `out/index.html` so a hard refresh on a
   client route doesn't 404. Don't try to solve routing recovery in Next.
