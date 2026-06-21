import { NextResponse } from "next/server";

/**
 * BFF proxy: POST /api/repos/:id/index -> Axum POST /repositories/:id/index.
 * Async (202): never waits for clone/embed.
 *
 * TODO: wire to Axum via `callAxum` from @/server/axum. Returns 501 until then.
 */
export async function POST(
  _req: Request,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  void id;
  // const res = await callAxum(`/repositories/${id}/index`, {
  //   method: "POST",
  //   body: await _req.text(),
  // });
  // return NextResponse.json(await res.json(), { status: res.status });
  return NextResponse.json({ error: "not_implemented" }, { status: 501 });
}
