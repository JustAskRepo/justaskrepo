import { NextResponse } from "next/server";

/**
 * BFF proxy: POST /api/chat/ws-ticket -> Axum POST /chat/ws-ticket.
 * Returns a single-use, short-TTL ticket the browser uses to open the chat
 * WebSocket directly to Axum.
 *
 * TODO: wire to Axum via `callAxum` from @/server/axum. Returns 501 until then.
 */
export async function POST(req: Request) {
  // const res = await callAxum("/chat/ws-ticket", {
  //   method: "POST",
  //   body: await req.text(),
  // });
  void req;
  // return NextResponse.json(await res.json(), { status: res.status });
  return NextResponse.json({ error: "not_implemented" }, { status: 501 });
}
