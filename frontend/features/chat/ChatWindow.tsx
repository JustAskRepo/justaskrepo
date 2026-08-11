"use client";

import { useState } from "react";
import { useChatSocket } from "@/features/chat/useChatSocket";

/**
 * Chat island for a repository session. Owns the input box and renders the
 * streamed assistant reply from the chat WebSocket hook.
 */
export default function ChatWindow({ sessionId }: { sessionId: string }) {
  const { reply, sendMessage } = useChatSocket(sessionId);
  const [input, setInput] = useState("");

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!input.trim()) return;
    await sendMessage(input);
    setInput("");
  }

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex-1 overflow-y-auto rounded-xl border border-white/8 bg-white/[0.03] p-4 text-sm text-ink">
        {reply || <span className="text-muted">Ask something about this repo…</span>}
      </div>
      <form onSubmit={handleSubmit} className="flex gap-2">
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="How does authentication work?"
          className="flex-1 rounded-full border border-white/10 bg-white/5 px-4 py-2 text-sm text-ink outline-none focus:border-accent/50"
        />
        <button
          type="submit"
          className="rounded-full bg-accent px-4 py-2 text-sm font-semibold text-white"
        >
          Ask
        </button>
      </form>
    </div>
  );
}
