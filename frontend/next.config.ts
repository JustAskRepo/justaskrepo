import type { NextConfig } from "next";

const nextConfig: NextConfig = {

  output: "export",
  images: { unoptimized: true },

  trailingSlash: true,

  async rewrites() {
    const axum = process.env.AXUM_DEV_URL ?? "http://localhost:8080";
    return [{ source: "/api/:path*", destination: `${axum}/api/:path*` }];
  },
};

export default nextConfig;
