import ChatWindow from "@/features/chat/ChatWindow";

/**
 * Chat page for a repository. Server component that resolves the route params
 * and mounts the client chat island. `params` is async in Next 16.
 *
 * TODO: create/resolve a chat session for this repo via the BFF and pass the
 * real session_id instead of the placeholder.
 */
export default async function RepoChatPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;
  const sessionId = `repo-${id}`; // TODO: real session id from POST /chat/sessions

  return (
    <main className="mx-auto flex h-[calc(100vh-2rem)] w-full max-w-4xl flex-col px-6 py-12">
      <h1 className="mb-6 text-xl font-semibold tracking-tight text-ink">
        Chat
      </h1>
      <div className="min-h-0 flex-1">
        <ChatWindow sessionId={sessionId} />
      </div>
    </main>
  );
}
