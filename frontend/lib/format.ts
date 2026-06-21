/**
 * Small presentation helpers. Pure, client-safe formatting utilities.
 */

/** Human-readable relative time, e.g. "3 min ago". */
export function timeAgo(iso: string | null): string {
  if (!iso) return "never";
  const then = new Date(iso).getTime();
  const seconds = Math.round((Date.now() - then) / 1000);
  if (seconds < 60) return "just now";
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  const days = Math.round(hours / 24);
  return `${days} d ago`;
}

/** Short commit sha, e.g. "a1b2c3d". */
export function shortSha(sha: string | null): string {
  return sha ? sha.slice(0, 7) : "—";
}
