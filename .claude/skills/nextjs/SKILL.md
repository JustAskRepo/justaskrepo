---
name: nextjs-tailwind
description: Next.js 16 (App Router) + Tailwind CSS v4 + TypeScript best practices for idiomatic, fast, maintainable frontend code. Use when writing Next.js pages/components, styling with Tailwind, reviewing frontend code, or migrating from Next 15 / Tailwind v3.
allowed-tools: Read, Edit, Write, Bash, Grep, Glob, Task
---

# Next.js 16 + Tailwind v4 Best Practices

Guidelines for writing idiomatic, performant, and maintainable Next.js App Router code.

**Target versions:** Next.js 16.3+, React 19.2, Tailwind CSS v4, TypeScript 5.x, Node 20.9+.
If the project is on older versions, say so and adapt — do not silently emit v16/v4 syntax into a v15/v3 codebase.

## Core Principles

1. **Server by default** — only reach for `"use client"` at the leaf that actually needs interactivity
2. **Dynamic by default, cache explicitly** — Next 16 removed implicit caching; opt in with `"use cache"`
3. **Parallelize I/O** — waterfalls cost more than any render optimization
4. **Ship less JavaScript** — bundle size is the second-largest lever after waterfalls
5. **Tokens over arbitrary values** — a design token in `@theme` beats `bg-[#1da1f2]` every time
6. **Types at the boundary** — validate external input (forms, params, APIs) with a schema, then trust it inward

## Code health

1. Prefer small components; extract when a file passes ~150 lines or grows a second responsibility
2. Colocate route-specific components under the route; promote to `components/` only on second use
3. Don't create abstractions for a single caller
4. No barrel files (`index.ts` re-export hubs) — they defeat tree-shaking

## Project Structure

```
app/
  layout.tsx              # root layout, fonts, providers
  page.tsx
  globals.css             # Tailwind entry + @theme
  (marketing)/            # route groups — no URL segment
  products/
    [id]/
      page.tsx
      loading.tsx         # Suspense fallback for the segment
      error.tsx           # must be a Client Component
      _components/        # private folder, not routable
  api/
    webhook/route.ts
components/ui/            # shared, presentational
lib/                      # pure helpers, db clients, schemas
proxy.ts                  # Next 16 replacement for middleware.ts
```

`error.tsx` and `global-error.tsx` must be Client Components. `not-found.tsx` and `loading.tsx` need not be.

---

## Server vs Client Components

### Push `"use client"` to the Leaves

```tsx
// BAD - whole page becomes client, all data fetching moves to the browser
"use client";
export default function Page() {
  const [open, setOpen] = useState(false);
  return <article>{/* lots of static content */}</article>;
}

// GOOD - server page, one small interactive island
export default async function Page() {
  const post = await getPost();
  return (
    <article>
      <PostBody html={post.html} />
      <LikeButton postId={post.id} />  {/* "use client" lives here */}
    </article>
  );
}
```

### Pass Serializable Data, and Only What's Needed

```tsx
// BAD - serializes the entire row into the RSC payload
<UserBadge user={user} />

// GOOD - send the two fields the component reads
<UserBadge name={user.name} avatarUrl={user.avatarUrl} />
```

Functions, class instances, `Date` methods, and Symbols do not cross the boundary. Server Actions are the exception — they can be passed as props.

### Slot Server Content Through Client Components

```tsx
// GOOD - Providers is a Client Component, children stay server-rendered
<ThemeProvider>
  <ServerSideDashboard />
</ThemeProvider>
```

A Client Component's `children` are not forced to become client — only its imports are.

---

## Async Request APIs (Next 16 Breaking Change)

`params`, `searchParams`, `cookies()`, `headers()`, and `draftMode()` are async. The synchronous form was removed.

```tsx
// BAD - was valid in Next 14
export default function Page({ params }: { params: { id: string } }) {
  return <div>{params.id}</div>;
}

// GOOD
export default async function Page({
  params,
  searchParams,
}: {
  params: Promise<{ id: string }>;
  searchParams: Promise<{ q?: string }>;
}) {
  const { id } = await params;
  const { q } = await searchParams;
  return <div>{id}</div>;
}
```

This applies to `generateMetadata`, `generateImageMetadata`, and Route Handlers too.

```tsx
// GOOD - Route Handler
export async function GET(
  req: Request,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const cookieStore = await cookies();
  return Response.json({ id, theme: cookieStore.get("theme")?.value });
}
```

Run `npx @next/codemod@canary upgrade latest` for the mechanical parts. It misses async APIs accessed inside custom hooks or behind conditionals — find those by testing.

---

## Data Fetching

### Never Await Sequentially When Requests Are Independent

```tsx
// BAD - two round trips in series
const user = await getUser(id);
const posts = await getPosts(id);

// GOOD - one round trip
const [user, posts] = await Promise.all([getUser(id), getPosts(id)]);
```

### Start Promises Early, Await Late

```tsx
// BAD - blocks before the cheap check
const flags = await getFlags();
if (!id) return notFound();

// GOOD - cheap sync guard first, then kick off I/O
if (!id) return notFound();
const flagsPromise = getFlags();
const user = await getUser(id);
const flags = await flagsPromise;
```

### Stream Slow Sections with Suspense

```tsx
// GOOD - shell renders immediately, reviews stream in
export default async function Page({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  return (
    <>
      <ProductHeader id={id} />
      <Suspense fallback={<ReviewsSkeleton />}>
        <Reviews id={id} />
      </Suspense>
    </>
  );
}
```

### Deduplicate Per-Request Reads with `cache()`

```tsx
import { cache } from "react";

// GOOD - layout and page both call getUser(id); the DB sees one query
export const getUser = cache(async (id: string) => db.user.findUnique({ where: { id } }));
```

`React.cache()` lives for one request only. For cross-request reuse, use `"use cache"` or an LRU.

---

## Caching (Cache Components)

Next 16 caches nothing by default. Enable the model, then mark what to cache.

```ts
// next.config.ts
import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  cacheComponents: true,
  cacheLife: {
    product: { stale: 3600, revalidate: 86400, expire: 604800 },
  },
};
export default nextConfig;
```

```tsx
// GOOD - cache one component, tag it, and revalidate on write
import { cacheLife, cacheTag, revalidateTag } from "next/cache";

async function ProductCard({ id }: { id: string }) {
  "use cache";
  cacheLife("product");
  cacheTag(`product:${id}`);
  const product = await db.product.findUnique({ where: { id } });
  return <article>{product?.name}</article>;
}

// in a Server Action after mutating
revalidateTag(`product:${id}`);
```

Rules of thumb:
- `"use cache"` goes at the top of a file, component, or async function
- A cached scope may not read request-time data (`cookies()`, `headers()`, `searchParams`) — read it in the caller and pass it as an argument
- Arguments must be serializable; they become part of the cache key
- `updateTag(tag)` refreshes immediately in the same request; `revalidateTag(tag)` marks stale

---

## Server Actions

### Always Authenticate — They Are Public Endpoints

```tsx
// BAD - assumes the caller went through your UI
"use server";
export async function deleteProject(id: string) {
  await db.project.delete({ where: { id } });
}

// GOOD
"use server";
export async function deleteProject(id: string) {
  const session = await auth();
  if (!session) throw new Error("Unauthorized");

  const parsed = z.string().uuid().parse(id);
  await db.project.delete({ where: { id: parsed, ownerId: session.userId } });
  revalidateTag(`projects:${session.userId}`);
}
```

A Server Action is an unauthenticated POST route until you check the session. Validate every argument — the client controls all of them.

### Use `useActionState` for Form State

```tsx
"use client";
const [state, formAction, isPending] = useActionState(submitContact, { error: null });

return (
  <form action={formAction}>
    <input name="email" type="email" required />
    <button disabled={isPending}>{isPending ? "Sending…" : "Send"}</button>
    {state.error && <p role="alert">{state.error}</p>}
  </form>
);
```

---

## Bundle Size

### Import Directly, Not Through Barrels

```tsx
// BAD - may pull the whole library into the graph
import { Check } from "@/components/ui";

// GOOD
import { Check } from "@/components/ui/check";
```

### Lazy-Load Heavy, Below-the-Fold Components

```tsx
// GOOD
const Editor = dynamic(() => import("./editor"), {
  ssr: false,
  loading: () => <EditorSkeleton />,
});
```

### Keep Import Paths Statically Analyzable

```tsx
// BAD - bundler must include every possible match
const mod = await import(`./locales/${locale}.js`);

// GOOD
const loaders = {
  en: () => import("./locales/en.js"),
  hi: () => import("./locales/hi.js"),
} as const;
const mod = await loaders[locale]();
```

---

## Tailwind v4 Setup

One import replaces the three v3 directives. There is no `tailwind.config.js`.

```css
/* app/globals.css */
@import "tailwindcss";

@custom-variant dark (&:where(.dark, .dark *));

@theme {
  --color-brand-50:  oklch(0.97 0.02 250);
  --color-brand-500: oklch(0.62 0.19 250);
  --color-brand-900: oklch(0.32 0.11 250);

  --font-sans: "Inter Variable", ui-sans-serif, system-ui, sans-serif;
  --radius-card: 0.75rem;
  --spacing-section: 6rem;
}

@utility tab-4 {
  tab-size: 4;
}
```

Defining `--color-brand-500` generates `bg-brand-500`, `text-brand-500`, `border-brand-500`, `ring-brand-500` and so on, automatically. Defining `--spacing-section` generates `py-section`, `mt-section`, `gap-section`.

For Next.js, install the PostCSS plugin:

```js
// postcss.config.mjs
export default { plugins: { "@tailwindcss/postcss": {} } };
```

Content is detected automatically and respects `.gitignore` — no `content` array. Use `@source "../packages/ui";` only for templates outside the project tree.

### v3 Syntax Models Still Emit — Watch For It

```css
/* BAD - v3, does not work in v4 */
@tailwind base;
@tailwind components;
@tailwind utilities;

/* GOOD */
@import "tailwindcss";
```

| v3 (wrong in v4) | v4 |
|---|---|
| `tailwind.config.js` | `@theme` in CSS |
| `bg-gradient-to-r` | `bg-linear-to-r` |
| `bg-opacity-50` | `bg-black/50` |
| `darkMode: "class"` config | `@custom-variant dark (...)` |
| `plugin()` in JS config | `@plugin "…"` / `@utility` |
| `w-[var(--x)]` | `w-(--x)` |
| `ring` = 3px | `ring` = 1px, use `ring-3` |
| default border = gray-200 | default border = `currentColor` |

Legacy configs can be bridged with `@config "./tailwind.config.js";` during migration, but treat that as temporary.

---

## Tailwind Class Composition

### Merge Conditional Classes with `cn`

```ts
// lib/utils.ts
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

```tsx
// BAD - template strings; later class doesn't win, specificity is a coin flip
<div className={`p-4 ${isActive ? "bg-brand-500" : ""} ${className}`} />

// GOOD - conflicts resolved, caller can override
<div className={cn("p-4 bg-white", isActive && "bg-brand-500", className)} />
```

### Use CVA for Variants, Not Boolean Prop Soup

```ts
// GOOD
const button = cva(
  "inline-flex items-center justify-center rounded-card font-medium transition-colors focus-visible:ring-2 focus-visible:ring-brand-500 disabled:opacity-50",
  {
    variants: {
      intent: {
        primary: "bg-brand-500 text-white hover:bg-brand-900",
        ghost: "bg-transparent hover:bg-brand-50",
      },
      size: { sm: "h-8 px-3 text-sm", md: "h-10 px-4" },
    },
    defaultVariants: { intent: "primary", size: "md" },
  },
);

type ButtonProps = React.ComponentProps<"button"> & VariantProps<typeof button>;
```

### Never Build Class Names by Concatenation

```tsx
// BAD - Tailwind's scanner sees "text-" and generates nothing
<p className={`text-${color}-500`} />

// GOOD - full class names, statically visible
const tone = { danger: "text-red-500", ok: "text-green-500" } as const;
<p className={tone[status]} />
```

### Reach for `@apply` Rarely

`@apply` reintroduces the naming problem Tailwind exists to remove. Extract a component instead. Legitimate uses: third-party markup you can't touch, and `prose`-style content overrides.

---

## Responsive, Dark Mode, Container Queries

```tsx
// GOOD - mobile-first; unprefixed is the small screen
<div className="flex flex-col gap-4 md:flex-row md:gap-8" />

// GOOD - pair every color with its dark counterpart at the point of use
<div className="bg-white text-gray-900 dark:bg-gray-950 dark:text-gray-50" />

// GOOD - container queries are built in; component adapts to its slot, not the viewport
<div className="@container">
  <article className="grid grid-cols-1 @md:grid-cols-2" />
</div>
```

Prefer semantic tokens (`bg-surface`, `text-muted`) defined once in `@theme` over repeating `dark:` pairs across dozens of components.

---

## TypeScript

### Type Props from the DOM Element

```tsx
// BAD - loses every native button attribute
type Props = { onClick: () => void; children: ReactNode };

// GOOD
type Props = React.ComponentProps<"button"> & { intent?: "primary" | "ghost" };
```

### Validate at the Boundary

```ts
// GOOD - parse untrusted input once, then the type is real
const SearchParams = z.object({
  q: z.string().trim().min(1).optional(),
  page: z.coerce.number().int().positive().default(1),
});

const { q, page } = SearchParams.parse(await searchParams);
```

### Avoid `any` and Non-Null Assertions

```ts
// BAD
const user = data! as any;

// GOOD
const user = UserSchema.parse(data);
```

---

## Images, Fonts, Metadata

```tsx
// GOOD - next/font self-hosts, no layout shift, no external request
import { Inter } from "next/font/google";
const inter = Inter({ subsets: ["latin"], variable: "--font-sans" });

// GOOD - always give width/height or fill; mark the LCP image priority
<Image src={hero} alt="Team at work" width={1200} height={630} priority />
```

Next 16 narrowed `images.qualities` to `[75]` by default — declare others explicitly in `next.config.ts` if you use them.

```ts
// GOOD - static metadata is hoisted and cached
export const metadata: Metadata = {
  title: "Products",
  openGraph: { images: ["/og.png"] },
};
```

Use `generateMetadata` only when the values depend on request data; it blocks the response.

---

## Accessibility

```tsx
// BAD
<div onClick={handleClick}>Delete</div>

// GOOD
<button type="button" onClick={handleClick}>Delete</button>

// GOOD - icon-only controls need an accessible name
<button aria-label="Close dialog"><X aria-hidden="true" /></button>
```

- Never remove focus rings; restyle with `focus-visible:ring-2`
- Every input needs a `<label htmlFor>`, not just a placeholder
- Body text at 4.5:1 contrast minimum — check both themes
- Announce async errors with `role="alert"`

---

## Testing

```tsx
// GOOD - query the way a user would, not by test id or class
test("submits the contact form", async () => {
  render(<ContactForm />);
  await userEvent.type(screen.getByLabelText(/email/i), "a@b.com");
  await userEvent.click(screen.getByRole("button", { name: /send/i }));
  expect(await screen.findByRole("alert")).toHaveTextContent(/thanks/i);
});
```

- Vitest + Testing Library for components; Playwright for flows that cross routes or auth
- Async Server Components are not renderable by Testing Library — test their data functions directly and E2E the page
- Assert on rendered output, never on Tailwind class strings

---

## Anti-Patterns to Avoid

| Anti-Pattern | Better Approach |
|---|---|
| `"use client"` at the top of a page | Push it to the interactive leaf |
| Sequential `await` on independent data | `Promise.all()` |
| `useEffect` to fetch on mount | Fetch in a Server Component |
| `useEffect` to derive state | Compute during render |
| Unauthenticated Server Action | Check session + validate args first |
| Barrel `index.ts` re-exports | Import from the source file |
| `` className={`text-${c}-500`} `` | Map to full class names |
| Template-string class merging | `cn()` with `tailwind-merge` |
| `bg-[#1da1f2]` scattered around | Token in `@theme` |
| `@apply` for component styling | Extract a React component |
| Boolean props (`isPrimary`, `isSmall`) | CVA variants |
| `<div onClick>` | `<button>` |
| `params.id` without `await` | `const { id } = await params` |
| `middleware.ts` | `proxy.ts` (Next 16) |
| `any` on API responses | Parse with a schema |

---

## Quick Reference

```bash
# Quality gates
npx tsc --noEmit && npx next lint && npx vitest run && npx next build

# Scaffold
npx create-next-app@latest --typescript --tailwind --app --eslint

# Upgrades
npx @next/codemod@canary upgrade latest   # Next 15 -> 16
npx @tailwindcss/upgrade                  # Tailwind v3 -> v4

# Agent context (Next 16.3+ writes AGENTS.md / CLAUDE.md automatically)
npx @next/codemod@canary agents-md        # for 16.1 and earlier

# Dev / build
next dev                                  # Turbopack is the default bundler
next build --webpack                      # escape hatch if a webpack plugin blocks you
ANALYZE=true next build                   # with @next/bundle-analyzer
```

**Migration order that minimizes pain:** upgrade React → run the Next codemod → fix async APIs the codemod missed → rename `middleware.ts` to `proxy.ts` → enable `cacheComponents` last, once the app builds clean.
