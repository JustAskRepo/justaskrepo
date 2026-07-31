import type { NextConfig } from "next";

const nextConfig: NextConfig = {

  output: "export",
  images: { unoptimized: true },

  // Emits `route/index.html` so Axum's ServeDir can resolve a hard refresh on a
  // client route. Page-file layout only — see skipTrailingSlashRedirect below.
  trailingSlash: true,

  // Without this, `trailingSlash: true` also 308s *request* paths, including the
  // /api/* rewrite below: /api/auth/github/login -> /api/auth/github/login/.
  // That breaks dev two ways. Axum 0.8 dropped implicit trailing-slash matching,
  // so a route registered at the unslashed path 404s. And prod has no Next server
  // at all (Axum serves the export directly), so prod would see the unslashed URL
  // while dev saw the slashed one — a divergence that only ever bites in dev.
  // Paths keep the href they were written with; `trailingSlash` still governs
  // the exported file layout.
  skipTrailingSlashRedirect: true,

  // Dev-only. Static export has no server, so this rewrite is absent from the
  // production build by design: there, Axum serves the export and /api from one
  // origin. In dev it recreates that single origin so session cookies behave
  // identically. Container-to-container, hence the compose service name.
  async rewrites() {
    const axum = process.env.AXUM_DEV_URL ?? "http://localhost:8080";
    return [{ source: "/api/:path*", destination: `${axum}/api/:path*` }];
  },
};

export default nextConfig;
