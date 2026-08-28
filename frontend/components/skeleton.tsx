/** Shimmering placeholder block. Compose several to sketch a loading layout. */
export default function Skeleton({ className = "" }: { className?: string }) {
  return <div aria-hidden="true" className={`skeleton rounded-md ${className}`} />;
}
