import GitHubButton from "./components/github-button";
import AskTerminal from "./components/ask-terminal";
import Reveal from "./components/reveal";

/* ------------------------------------------------------------------ */
/*  Small inline icons                                                 */
/* ------------------------------------------------------------------ */
type IconProps = { className?: string };

function IconChunk({ className }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" className={className} aria-hidden="true">
      <path d="M8 3H5a2 2 0 0 0-2 2v3M16 3h3a2 2 0 0 1 2 2v3M8 21H5a2 2 0 0 1-2-2v-3M16 21h3a2 2 0 0 0 2-2v-3" />
      <path d="m9 9 1.5 3L9 15M14 9h2" />
    </svg>
  );
}
function IconSearch({ className }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" className={className} aria-hidden="true">
      <circle cx="11" cy="11" r="7" />
      <path d="m21 21-4.3-4.3M11 8v6M8 11h6" />
    </svg>
  );
}
function IconCite({ className }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" className={className} aria-hidden="true">
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
      <path d="M14 2v6h6M9 13l2 2 4-4" />
    </svg>
  );
}
function IconBolt({ className }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" className={className} aria-hidden="true">
      <path d="M13 2 4.5 13.5H11l-1 8.5 8.5-11.5H12z" />
    </svg>
  );
}

/* ------------------------------------------------------------------ */
/*  Data                                                               */
/* ------------------------------------------------------------------ */
const FEATURES = [
  {
    icon: IconChunk,
    title: "Code-aware chunking",
    body: "Tree-sitter parses your code by structure — functions, types, modules — so context never gets sliced in half.",
  },
  {
    icon: IconSearch,
    title: "Semantic search",
    body: "Ask in plain English. Vector search finds meaning, not just string matches, across every file in the repo.",
  },
  {
    icon: IconCite,
    title: "Answers with receipts",
    body: "Every answer points back to the exact file and line it came from. No hallucinated APIs, no guessing.",
  },
  {
    icon: IconBolt,
    title: "One-click connect",
    body: "Sign in with GitHub, pick a repo, and it indexes itself. Push new code and the index keeps up automatically.",
  },
];

const STEPS = [
  {
    n: "01",
    title: "Connect your repo",
    body: "Sign in with GitHub and choose the repository you want to talk to.",
  },
  {
    n: "02",
    title: "We index it",
    body: "Your code is chunked, embedded, and stored as vectors — usually in under a minute.",
  },
  {
    n: "03",
    title: "Just ask",
    body: "Ask anything about the codebase and get grounded answers with citations.",
  },
];

const SAMPLE_QUERIES = [
  "How does login work?",
  "Where do we call Stripe?",
  "What changed in the API last week?",
  "Explain this module to me",
  "Find all the TODOs",
  "Which tests cover checkout?",
];

/* ------------------------------------------------------------------ */
/*  Brand mark                                                         */
/* ------------------------------------------------------------------ */
function Logo() {
  return (
    <span className="flex items-center gap-2.5">
      <span className="relative flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-accent to-accent-2 font-mono text-sm font-bold text-white shadow-[0_8px_24px_-8px_rgba(139,92,246,0.8)]">
        {"{ }"}
      </span>
      <span className="text-[15px] font-semibold tracking-tight text-ink">
        JustAsk<span className="text-muted">Repo</span>
      </span>
    </span>
  );
}

/* ------------------------------------------------------------------ */
/*  Page                                                               */
/* ------------------------------------------------------------------ */
export default function Home() {
  return (
    <div className="relative isolate min-h-screen overflow-hidden">
      {/* ---------- animated background ---------- */}
      <div className="pointer-events-none absolute inset-0 -z-10">
        <div className="bg-grid absolute inset-0" />
        <div className="orb animate-float h-[34rem] w-[34rem] -left-40 -top-40 bg-accent/30" />
        <div className="orb animate-float-x h-[28rem] w-[28rem] right-[-8rem] top-10 bg-accent-2/25" />
        <div className="orb animate-float-slow h-[26rem] w-[26rem] left-1/3 top-[28rem] bg-accent-3/20" />
        <div className="absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-white/15 to-transparent" />
      </div>

      {/* ---------- nav ---------- */}
      <header className="mx-auto flex w-full max-w-6xl items-center justify-between px-6 py-5">
        <Logo />
        <nav className="hidden items-center gap-8 text-sm text-muted sm:flex">
          <a href="#features" className="transition-colors hover:text-ink">Features</a>
          <a href="#how" className="transition-colors hover:text-ink">How it works</a>
          <a
            href="https://github.com"
            target="_blank"
            rel="noreferrer"
            className="transition-colors hover:text-ink"
          >
            GitHub
          </a>
        </nav>
        <GitHubButton size="sm" className="hidden sm:inline-flex" />
      </header>

      {/* ---------- hero ---------- */}
      <main>
        <section className="mx-auto flex w-full max-w-4xl flex-col items-center px-6 pt-16 text-center sm:pt-24">
          <div
            className="rise inline-flex items-center gap-2 rounded-full border border-white/10 bg-white/5 px-4 py-1.5 text-xs text-muted backdrop-blur"
            style={{ animationDelay: "0ms" }}
          >
            <span className="relative flex h-2 w-2">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-accent-3 opacity-70" />
              <span className="relative inline-flex h-2 w-2 rounded-full bg-accent-3" />
            </span>
            Semantic understanding for your whole codebase
          </div>

          <h1
            className="rise mt-7 text-balance text-5xl font-semibold leading-[1.05] tracking-tight sm:text-7xl"
            style={{ animationDelay: "80ms" }}
          >
            Just <span className="text-gradient">ask</span> your repo.
          </h1>

          <p
            className="rise mt-6 max-w-2xl text-pretty text-lg leading-relaxed text-muted sm:text-xl"
            style={{ animationDelay: "160ms" }}
          >
            Stop grepping. Start asking. JustAskRepo turns your entire codebase
            into a conversation — semantic search, instant answers, and every
            response backed by the exact file it came from.
          </p>

          <div
            className="rise mt-9 flex flex-col items-center gap-4 sm:flex-row"
            style={{ animationDelay: "240ms" }}
          >
            <GitHubButton size="lg" />
            <a
              href="#how"
              className="group inline-flex items-center gap-1.5 rounded-full px-5 py-3 text-sm font-medium text-muted transition-colors hover:text-ink"
            >
              See how it works
              <span className="transition-transform duration-300 group-hover:translate-y-0.5">↓</span>
            </a>
          </div>

          <p
            className="rise mt-5 text-xs text-faint"
            style={{ animationDelay: "300ms" }}
          >
            Free to connect your first repo · No credit card
          </p>

          {/* animated terminal */}
          <div
            className="rise mt-16 flex w-full justify-center"
            style={{ animationDelay: "380ms" }}
          >
            <AskTerminal />
          </div>

          {/* sample query chips */}
          <div
            className="rise mt-10 flex flex-wrap items-center justify-center gap-2.5"
            style={{ animationDelay: "460ms" }}
          >
            {SAMPLE_QUERIES.map((q) => (
              <span
                key={q}
                className="rounded-full border border-white/8 bg-white/[0.03] px-3.5 py-1.5 font-mono text-xs text-muted transition-colors hover:border-accent/40 hover:text-ink"
              >
                {q}
              </span>
            ))}
          </div>
        </section>

        {/* ---------- features ---------- */}
        <section id="features" className="mx-auto w-full max-w-6xl px-6 py-28 sm:py-36">
          <Reveal className="mx-auto max-w-2xl text-center">
            <h2 className="text-3xl font-semibold tracking-tight sm:text-4xl">
              Your codebase, finally <span className="text-gradient">answerable</span>.
            </h2>
            <p className="mt-4 text-muted">
              Built for the questions you actually ask — not the keywords you
              hope to remember.
            </p>
          </Reveal>

          <div className="mt-14 grid gap-5 sm:grid-cols-2 lg:grid-cols-4">
            {FEATURES.map((f, i) => {
              const Icon = f.icon;
              return (
                <Reveal key={f.title} delay={i * 90}>
                  <div className="glass card-glow h-full rounded-2xl p-6">
                    <div className="flex h-11 w-11 items-center justify-center rounded-xl bg-gradient-to-br from-accent/25 to-accent-2/20 text-accent-2 ring-1 ring-white/10">
                      <Icon className="h-5 w-5" />
                    </div>
                    <h3 className="mt-5 text-base font-semibold text-ink">{f.title}</h3>
                    <p className="mt-2 text-sm leading-relaxed text-muted">{f.body}</p>
                  </div>
                </Reveal>
              );
            })}
          </div>
        </section>

        {/* ---------- how it works ---------- */}
        <section id="how" className="mx-auto w-full max-w-6xl px-6 pb-28 sm:pb-36">
          <Reveal className="mx-auto max-w-2xl text-center">
            <h2 className="text-3xl font-semibold tracking-tight sm:text-4xl">
              From repo to answers in three steps
            </h2>
          </Reveal>

          <div className="relative mt-14 grid gap-5 md:grid-cols-3">
            {/* connecting line */}
            <div className="absolute left-0 right-0 top-12 hidden h-px bg-gradient-to-r from-transparent via-white/10 to-transparent md:block" />
            {STEPS.map((s, i) => (
              <Reveal key={s.n} delay={i * 120}>
                <div className="glass relative h-full rounded-2xl p-7">
                  <span className="font-mono text-sm text-accent-2">{s.n}</span>
                  <h3 className="mt-3 text-lg font-semibold text-ink">{s.title}</h3>
                  <p className="mt-2 text-sm leading-relaxed text-muted">{s.body}</p>
                </div>
              </Reveal>
            ))}
          </div>
        </section>

        {/* ---------- final CTA ---------- */}
        <section className="mx-auto w-full max-w-6xl px-6 pb-32">
          <Reveal>
            <div className="glass relative overflow-hidden rounded-3xl px-8 py-16 text-center sm:px-16 sm:py-20">
              <div className="orb animate-float h-72 w-72 left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 bg-accent/25" />
              <h2 className="relative text-balance text-4xl font-semibold tracking-tight sm:text-5xl">
                Stop grepping. <span className="text-gradient">Start asking.</span>
              </h2>
              <p className="relative mx-auto mt-5 max-w-xl text-muted">
                Connect a repository and have your first conversation with your
                codebase in under a minute.
              </p>
              <div className="relative mt-9 flex justify-center">
                <GitHubButton size="lg" />
              </div>
            </div>
          </Reveal>
        </section>
      </main>

      {/* ---------- footer ---------- */}
      <footer className="border-t border-white/8">
        <div className="mx-auto flex w-full max-w-6xl flex-col items-center justify-between gap-4 px-6 py-8 text-sm text-faint sm:flex-row">
          <Logo />
          <p>© {new Date().getFullYear()} JustAskRepo. Ask away.</p>
        </div>
      </footer>
    </div>
  );
}
