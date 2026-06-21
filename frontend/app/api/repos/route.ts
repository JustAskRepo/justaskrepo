import { NextResponse } from "next/server";

/**
 * BFF proxy: GET /api/repos -> Axum GET /repositories.
 *
 * TODO: wire to Axum via `callAxum("/repositories")` from @/server/axum once
 * the backend endpoint exists. Returns 501 until then.
 */
export async function GET() {
  // const res = await callAxum("/repositories");
  // return NextResponse.json(await res.json(), { status: res.status });
  return NextResponse.json(
    { error: "not_implemented" },
    { status: 501 },
  );
}
