import "server-only";

import NextAuth from "next-auth";
import GitHub from "next-auth/providers/github";

/**
 * NextAuth v5 configuration (see extras/JustAskRepo-Architecture.md §8).
 *
 * Strategy: stateless JWT session, no DB adapter — the canonical `users` row
 * is owned by the Rust backend. On first sign-in the `signIn` callback should
 * call Axum `POST /internal/users/sync` (service secret) and store the returned
 * internal `user_id` on the session.
 *
 * This is a scaffold. The provider reads GITHUB_ID / GITHUB_SECRET from env and
 * the user-sync wiring is left as a TODO until the backend endpoint exists.
 */
export const { handlers, auth, signIn, signOut } = NextAuth({
  providers: [GitHub],
  session: { strategy: "jwt" },
  callbacks: {
    async signIn() {
      // TODO: call Axum POST /internal/users/sync (X-Internal-Key) and persist
      // the returned internal user_id into the JWT via the `jwt` callback.
      return true;
    },
    async jwt({ token }) {
      // TODO: attach the backend-issued internal user_id (token.userId = ...).
      return token;
    },
    async session({ session }) {
      // TODO: expose the internal user_id on the session object.
      return session;
    },
  },
});

/**
 * Resolve the current request's internal backend user id from the session.
 * Returns null when unauthenticated. Server-only.
 */
export async function getUserId(): Promise<string | null> {
  const session = await auth();
  // TODO: read the internal user_id once the jwt/session callbacks populate it.
  return (session as unknown as { userId?: string } | null)?.userId ?? null;
}
