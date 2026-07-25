# ---- Next.js 16 frontend ----
# Requires `output: 'standalone'` in next.config.ts (see note below).
# Assumes npm — if you use pnpm or yarn, swap the install/build lines accordingly.

FROM node:22-alpine AS deps
WORKDIR /app
COPY package.json package-lock.json* ./
RUN npm ci

FROM node:22-alpine AS builder
WORKDIR /app
COPY --from=deps /app/node_modules ./node_modules
COPY . .
RUN npm run build

FROM node:22-alpine AS runner
WORKDIR /app
ENV NODE_ENV=production
RUN addgroup -g 1001 -S nodejs && adduser -S nextjs -u 1001
COPY --from=builder /app/public ./public
COPY --from=builder --chown=nextjs:nodejs /app/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/.next/static ./.next/static
USER nextjs
# The standalone server reads PORT/HOSTNAME — the `-p 3001` in package.json
# scripts does not apply here, so set them explicitly.
ENV PORT=3001
ENV HOSTNAME=0.0.0.0
EXPOSE 3001
CMD ["node", "server.js"]
