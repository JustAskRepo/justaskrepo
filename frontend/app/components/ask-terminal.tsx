"use client";

import { useEffect, useState } from "react";

type Exchange = {
  q: string;
  a: string;
  cite: string;
};

const EXCHANGES: Exchange[] = [
  {
    q: "where is authentication handled?",
    a: "Sessions are verified in validateSession() — it checks the GitHub token and loads the user before any route runs.",
    cite: "auth/session.ts:24",
  },
  {
    q: "what does the indexing pipeline do?",
    a: "It clones the repo, splits code with Tree-sitter, embeds each chunk with OpenAI, then upserts the vectors into Qdrant.",
    cite: "modules/indexing/api.rs",
  },
  {
    q: "find every call to the payments API",
    a: "3 matches — checkout, webhook handler, and refunds. They all route through the shared Stripe client.",
    cite: "lib/stripe.ts +2",
  },
  {
    q: "explain how modules talk to each other",
    a: "They never call each other directly. Each emits past-tense domain events on the EventBus; subscribers react.",
    cite: "ADR-004-event-bus.md",
  },
];

export default function AskTerminal() {
  const [index, setIndex] = useState(0);
  const [typed, setTyped] = useState("");
  const [showAnswer, setShowAnswer] = useState(false);

  useEffect(() => {
    const current = EXCHANGES[index];
    let timeout: ReturnType<typeof setTimeout>;

    if (typed.length < current.q.length) {
      // typing the question, character by character
      timeout = setTimeout(
        () => setTyped(current.q.slice(0, typed.length + 1)),
        38 + Math.random() * 46,
      );
    } else if (!showAnswer) {
      // finished typing → brief "thinking" pause → reveal answer
      timeout = setTimeout(() => setShowAnswer(true), 520);
    } else {
      // hold the answer on screen, then advance to the next question
      timeout = setTimeout(() => {
        setShowAnswer(false);
        setTyped("");
        setIndex((i) => (i + 1) % EXCHANGES.length);
      }, 3400);
    }

    return () => clearTimeout(timeout);
  }, [typed, showAnswer, index]);

  const current = EXCHANGES[index];
  const doneTyping = typed.length === current.q.length;

  return (
    <div className="glass relative w-full max-w-xl overflow-hidden rounded-2xl shadow-[0_30px_80px_-30px_rgba(0,0,0,0.9)]">
      {/* window chrome */}
      <div className="flex items-center gap-2 border-b border-white/8 px-4 py-3">
        <span className="h-3 w-3 rounded-full bg-[#ff5f57]" />
        <span className="h-3 w-3 rounded-full bg-[#febc2e]" />
        <span className="h-3 w-3 rounded-full bg-[#28c840]" />
        <span className="ml-3 font-mono text-xs text-faint">
          justaskrepo — ~/your-codebase
        </span>
      </div>

      {/* body */}
      <div className="min-h-[230px] space-y-4 p-5 font-mono text-sm leading-relaxed">
        {/* prompt line */}
        <div className="flex items-start gap-2">
          <span className="select-none text-accent-3">repo</span>
          <span className="select-none text-faint">❯</span>
          <span className="text-ink">
            {typed}
            {!doneTyping && <span className="caret">.</span>}
          </span>
        </div>

        {/* thinking / answer */}
        {doneTyping && (
          <div
            key={index}
            className="rise flex items-start gap-3"
            style={{ animationDuration: "0.5s" }}
          >
            <span className="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-md bg-gradient-to-br from-accent to-accent-2 text-[11px] font-bold text-white">
              A
            </span>

            {showAnswer ? (
              <div className="space-y-2.5">
                <p className="font-sans text-[13.5px] leading-relaxed text-[#cdd2e6]">
                  {current.a}
                </p>
                <span className="inline-flex items-center gap-1.5 rounded-md border border-white/10 bg-white/5 px-2 py-1 text-[11px] text-accent-2">
                  <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    className="h-3 w-3"
                    aria-hidden="true"
                  >
                    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                    <path d="M14 2v6h6" />
                  </svg>
                  {current.cite}
                </span>
              </div>
            ) : (
              <span className="inline-flex items-center gap-1 pt-1 text-faint">
                searching the index
                <span className="inline-flex gap-0.5">
                  <Dot delay="0ms" />
                  <Dot delay="160ms" />
                  <Dot delay="320ms" />
                </span>
              </span>
            )}
          </div>
        )}
      </div>

      {/* progress pips */}
      <div className="flex items-center gap-1.5 border-t border-white/8 px-5 py-3">
        {EXCHANGES.map((_, i) => (
          <span
            key={i}
            className={`h-1 rounded-full transition-all duration-500 ${
              i === index ? "w-6 bg-accent-2" : "w-2 bg-white/12"
            }`}
          />
        ))}
      </div>
    </div>
  );
}

function Dot({ delay }: { delay: string }) {
  return (
    <span
      className="inline-block h-1 w-1 rounded-full bg-accent-2"
      style={{ animation: "pulse-glow 1.1s ease-in-out infinite", animationDelay: delay }}
    />
  );
}
