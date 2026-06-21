import { handlers } from "@/server/auth";

/** NextAuth v5 catch-all route — delegates GET/POST to the configured handlers. */
export const { GET, POST } = handlers;
