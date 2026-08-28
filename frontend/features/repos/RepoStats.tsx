import CountUp from "@/components/count-up";
import SpotlightCard from "@/components/spotlight-card";
import { isActive, needsAttention } from "@/features/repos/status";
import { timeAgo } from "@/lib/format";
import type { RepoSummary } from "@/types/api";

type IconProps = { className?: string };

function IconRepo({ className }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" className={className} aria-hidden="true">
      <path d="M4 19.5V5a2 2 0 0 1 2-2h13v18H6.5A2.5 2.5 0 0 1 4 19.5Z" />
      <path d="M4 17.5h15" />
    </svg>
  );
}
function IconCheck({ className }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" className={className} aria-hidden="true">
      <circle cx="12" cy="12" r="9" />
      <path d="m8.5 12.2 2.4 2.4 4.6-4.9" />
    </svg>
  );
}
function IconPulse({ className }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" className={className} aria-hidden="true">
      <path d="M3 12h4l2.5-6 4 12 2.5-6H21" />
    </svg>
  );
}
function IconAlert({ className }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" className={className} aria-hidden="true">
      <path d="M12 4.5 21 19H3l9-14.5Z" />
      <path d="M12 10v4M12 16.8v.2" />
    </svg>
  );
}

/**
 * Headline numbers for the repo list. Every figure is derived from the repos
 * already on screen — nothing here is fetched or estimated. Each tile carries
 * an icon and a label, so a tile is never identified by colour alone.
 */
export default function RepoStats({ repos }: { repos: RepoSummary[] }) {
  const total = repos.length;
  const indexed = repos.filter(
    (r) => r.current_index_status === "indexed",
  ).length;
  const running = repos.filter((r) => isActive(r.current_index_status)).length;
  const attention = repos.filter((r) =>
    needsAttention(r.current_index_status),
  ).length;
  const priv = repos.filter((r) => r.is_private).length;
  const coverage = total === 0 ? 0 : Math.round((indexed / total) * 100);

  const latest =
    repos
      .map((r) => r.last_indexed_at)
      .filter((at): at is string => Boolean(at))
      .sort()
      .at(-1) ?? null;

  const tiles = [
    {
      key: "total",
      label: "Repositories",
      value: total,
      tone: "text-ink",
      icon: IconRepo,
      iconTone: "text-accent-2",
      caption:
        total === 0
          ? "None connected yet"
          : `${priv} private · ${total - priv} public`,
    },
    {
      key: "indexed",
      label: "Indexed",
      value: indexed,
      tone: "text-ok",
      icon: IconCheck,
      iconTone: "text-ok",
      caption: latest ? `Last activity ${timeAgo(latest)}` : "Nothing indexed yet",
      meter: coverage,
    },
    {
      key: "running",
      label: "In progress",
      value: running,
      tone: running > 0 ? "text-accent-2" : "text-ink",
      icon: IconPulse,
      iconTone: running > 0 ? "text-accent-2" : "text-muted",
      caption: running > 0 ? "Refreshing automatically" : "Queue is clear",
    },
    {
      key: "attention",
      label: "Needs attention",
      value: attention,
      tone: attention > 0 ? "text-warn" : "text-ink",
      icon: IconAlert,
      iconTone: attention > 0 ? "text-warn" : "text-muted",
      caption: attention > 0 ? "Failed or stale — re-index" : "All healthy",
    },
  ] as const;

  return (
    <section aria-label="Repository overview" className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
      {tiles.map((tile, i) => {
        const Icon = tile.icon;
        return (
          // Entrance on a wrapper so the filled animation does not outrank
          // `.card-glow`'s hover transform (see RepoCard).
          <div key={tile.key} className="rise h-full" style={{ animationDelay: `${i * 70}ms` }}>
            <SpotlightCard className="h-full rounded-2xl p-5">
              <div className="flex items-center justify-between">
                <span className="text-xs font-medium uppercase tracking-wide text-muted">
                  {tile.label}
                </span>
                <Icon className={`h-4 w-4 ${tile.iconTone}`} />
              </div>

              <p className={`mt-3 text-4xl font-semibold tracking-tight ${tile.tone}`}>
                <CountUp value={tile.value} />
              </p>

              {"meter" in tile && (
                <div className="mt-3">
                  <div className="h-1 overflow-hidden rounded-full bg-white/8">
                    <div
                      className="h-full rounded-full bg-linear-to-r from-ok to-accent-2 transition-[width] duration-700 ease-out"
                      style={{ width: `${tile.meter}%` }}
                    />
                  </div>
                  <p className="mt-1.5 text-[11px] text-muted">{tile.meter}% index coverage</p>
                </div>
              )}

              <p className="mt-2 text-xs text-muted">{tile.caption}</p>
            </SpotlightCard>
          </div>
        );
      })}
    </section>
  );
}
