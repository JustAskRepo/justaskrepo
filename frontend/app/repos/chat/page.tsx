"use client";

import { Suspense } from "react";
import { useSearchParams } from "next/navigation";
import ChatWindow from "@/features/chat/ChatWindow";

/**
 * Chat page for a repository — client component. Static export can't render a
 * dynamic `[id]` segment, so the repo id arrives as `?id=` and is read on the
 * client. `useSearchParams` must sit under <Suspense>.
 *
 * TODO: create/resolve a real chat session for this repo (POST /chat/sessions)
 * and pass its id instead of the `repo-<id>` placeholder.
 */
function RepoChat() {
  const id = useSearchParams().get("id");
  if (!id) {
    return <p className="text-sm text-red-400" role="alert">Missing repo id.</p>;
  }
  const sessionId = `repo-${id}`;
  return (
    <div className="min-h-0 flex-1">
      <ChatWindow sessionId={sessionId} />
    </div>
  );
}

export default function RepoChatPage() {
  return (
    <main className="mx-auto flex h-[calc(100vh-2rem)] w-full max-w-4xl flex-col px-6 py-12">
      <h1 className="mb-6 text-xl font-semibold tracking-tight text-ink">Chat</h1>
      <Suspense fallback={<p className="text-sm text-muted">Loading…</p>}>
        <RepoChat />
      </Suspense>
    </main>
  );
}
