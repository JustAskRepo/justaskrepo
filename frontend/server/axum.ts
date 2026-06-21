import "server-only";

import { getUserId } from "@/server/auth";

/**
 * Server-only client for the Rust/Axum backend (see
 * extras/JustAskRepo-Architecture.md §8, Zone B / BFF).
 *
 * This is the trust boundary: the GitHub OAuth token never crosses into the
 * backend. Each call mints a short-lived (~60s) internal JWT (`sub=user_id`)
 * and sends it as `Bearer`. NEVER import this module from a client component.
 */

const AXUM_BASE_URL = process.env.AXUM_BASE_URL ?? "http://localhost:8080";

/**
 * Mint a short-lived internal JWT for the given backend user id.
 * TODO: sign with the shared internal secret (e.g. via `jose`) — 60s expiry,
 * `sub = userId`. Stubbed until the backend's JWT verification is in place.
 */
async function mintInternalJwt(userId: string): Promise<string> {
  // TODO: implement JWT signing — sign { sub: userId } with the internal secret.
  void userId;
  return "TODO_INTERNAL_JWT";
}

/**
 * Authenticated server-side fetch to Axum. Resolves the current user, mints an
 * internal JWT, and attaches it as a Bearer token. Throws if unauthenticated.
 */
export async function callAxum(
  path: string,
  init: RequestInit = {},
): Promise<Response> {
  const userId = await getUserId();
  if (!userId) {
    throw new Error("callAxum: no authenticated user");
  }

  const token = await mintInternalJwt(userId);

  return fetch(`${AXUM_BASE_URL}${path}`, {
    ...init,
    headers: {
      ...init.headers,
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
    },
    cache: "no-store",
  });
}
