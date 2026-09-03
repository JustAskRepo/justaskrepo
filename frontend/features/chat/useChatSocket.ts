"use client";

import { useCallback, useState } from "react";
import type { ChatFrame } from "@/types/api";

interface ChatSocketState {
  /** Accumulated assistant tokens for the in-flight reply. */
  reply: string;
  connected: boolean;
  sendMessage: (message: string) => Promise<void>;
}

/**
 * Chat streaming hook — still a scaffold, and still a WebSocket-shaped one.
 *
 * The transport is SSE as of 2026-09-03 (AUTHENTICATION.md §Streaming
 * Authentication, ADR-008 §7): POST the question, then read `ChatFrame`s off an
 * `EventSource` on the reply stream. Both requests are same-origin and carry the
 * session cookie themselves, so there is no ticket to fetch first — and the
 * `sessionId` argument below is a leftover of the ticket design. It goes away
 * with the rewrite: the cookie is `HttpOnly`, so the browser can never hold that
 * value in the first place.
 */
export function useChatSocket(sessionId: string): ChatSocketState {
  const [reply, setReply] = useState("");
  const [connected] = useState(false);

  const sendMessage = useCallback(
    async (message: string) => {
      setReply("");
      // TODO: POST the question to the chat endpoint, then
      //       new EventSource("/api/chat/…/stream") and fold each frame in with
      //       applyFrame -> append tokens / collect citations / finish.
      void sessionId;
      void message;
    },
    [sessionId],
  );

  return { reply, connected, sendMessage };
}

/** Reducer-friendly frame handler kept here so transport stays one place. */
export function applyFrame(reply: string, frame: ChatFrame): string {
  return frame.type === "token" ? reply + frame.delta : reply;
}
