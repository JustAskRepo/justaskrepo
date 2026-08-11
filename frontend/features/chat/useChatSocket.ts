"use client";

import { useCallback, useState } from "react";
import { chatSocketUrl, getWsTicket } from "@/lib/api-client";
import type { ChatFrame } from "@/types/api";

interface ChatSocketState {
  /** Accumulated assistant tokens for the in-flight reply. */
  reply: string;
  connected: boolean;
  sendMessage: (message: string) => Promise<void>;
}

/**
 * Chat WebSocket hook (see extras/JustAskRepo-Architecture.md §2.4).
 *
 * Flow: fetch a single-use ws-ticket from Axum (`getWsTicket`), open
 * `chatSocketUrl(ticket)` (same-origin wss to Axum), send { session_id,
 * message }, and stream `ChatFrame`s back. This is a scaffold — the actual
 * socket wiring is left as a TODO.
 */
export function useChatSocket(sessionId: string): ChatSocketState {
  const [reply, setReply] = useState("");
  const [connected] = useState(false);

  const sendMessage = useCallback(
    async (message: string) => {
      setReply("");
      // TODO: const { ticket } = await getWsTicket(sessionId);
      //       const ws = new WebSocket(chatSocketUrl(ticket));
      //       ws.send({ session_id, message }); on each frame:
      //       handleFrame(frame) -> append tokens / collect citations / finish.
      void getWsTicket;
      void chatSocketUrl;
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
