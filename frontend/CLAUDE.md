@AGENTS.md

# JustAskRepo Frontend — Structure (enforced)

> The frontend is **thin**: Rust owns repo-domain complexity. Next.js does identity,
> presentation, and proxying. This structure mirrors `extras/JustAskRepo-Architecture.md` §8
> and MUST be maintained. The `@/*` import alias maps to the `frontend/` root (see `tsconfig.json`).

```
frontend/
  app/
    page.tsx                         # marketing landing ("/")
    (auth)/login/page.tsx            # GitHub OAuth entry
    dashboard/page.tsx               # server component: lists repos
    repos/[id]/page.tsx              # server component: repo detail + status
    repos/[id]/chat/page.tsx         # client island for chat
    api/                             # BFF route handlers (browser -> here -> Axum)
      auth/[...nextauth]/route.ts    # NextAuth v5
      repos/route.ts                 # -> Axum GET /repositories
      repos/[id]/index/route.ts      # -> POST /repositories/:id/index
      repos/[id]/status/route.ts     # -> GET status (polled)
      chat/ws-ticket/route.ts        # -> POST /chat/ws-ticket
  components/                        # dumb, reusable presentational components
  features/                          # feature-scoped UI + logic
    repos/{RepoList,RepoStatusBadge,IndexButton}.tsx
    chat/{ChatWindow.tsx,useChatSocket.ts}
  lib/                               # client-safe helpers
    api-client.ts                    # typed fetch wrapper (browser -> /api/*)
    format.ts
  server/                            # SERVER-ONLY (never bundled to client)
    axum.ts                          # server fetch to Axum + internal-JWT minting
    auth.ts                          # NextAuth config, auth() helper, getUserId()
  types/
    api.ts                           # shared DTOs mirroring the Axum API
```

## Rules

1. **Server-only boundary.** Anything that touches the Axum backend, mints the internal JWT,
   or reads a secret lives in `server/` and starts with `import "server-only"`. NEVER import a
   `server/` module from a client component (`"use client"`) or from `lib/`/`components/`.
2. **All authenticated REST goes through the BFF.** Browser → `app/api/*` route handler → Axum.
   The browser never calls Axum directly or holds the internal JWT. The single exception is the
   chat **WebSocket**, opened directly to Axum using a single-use ticket from
   `/api/chat/ws-ticket`.
3. **Where things go:**
   - Reusable, presentational, feature-agnostic UI → `components/`.
   - Feature-scoped components, hooks, and client logic → `features/<feature>/`.
   - Browser fetch calls → `lib/api-client.ts` (typed, hits `/api/*` only).
   - Shared wire/DTO types → `types/api.ts` (keep in sync with the Rust backend).
   - Pure formatting/util helpers → `lib/`.
4. **Routes follow the `app/` tree.** New pages under `app/`; new BFF proxies under `app/api/`.
   Use the Next 16 async `params` convention (`params: Promise<{...}>`, then `await params`).
5. **Server vs client.** Data fetching and the internal JWT live in server components / route
   handlers. Interactive bits (chat socket, status polling, the Index button) are client
   components.
