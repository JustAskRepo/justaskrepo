import { NextResponse } from "next/server";

/**
 * BFF proxy: GET /api/repos/:id/status -> Axum GET /repositories/:id/status.
 * Polled (~2s) by the repo detail page while the index status is non-terminal.
 *
 * TODO: wire to Axum via `callAxum` from @/server/axum. Returns 501 until then.
 */
export async function GET(
  _req: Request,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  void id;
  // const res = await callAxum(`/repositories/${id}/status`);
  // return NextResponse.json(await res.json(), { status: res.status });
  return NextResponse.json({ error: "not_implemented" }, { status: 501 });
}
