/**
 * Decorative page backdrop — the grid + floating orbs used on the landing page,
 * factored out so every app screen sits on the same surface. Fixed so it does
 * not scroll away on long lists. Purely ornamental: hidden from the a11y tree.
 */
export default function AmbientBackground({ className = "" }: { className?: string }) {
  return (
    <div aria-hidden="true" className={`pointer-events-none fixed inset-0 -z-10 ${className}`}>
      <div className="bg-grid absolute inset-0" />
      <div className="orb animate-float h-[30rem] w-[30rem] -left-40 -top-48 bg-accent/25" />
      <div className="orb animate-float-x h-[24rem] w-[24rem] -right-24 -top-16 bg-accent-2/20" />
      <div className="orb animate-float-slow h-[22rem] w-[22rem] left-1/3 bottom-[-10rem] bg-accent-3/15" />
      <div className="absolute inset-x-0 top-0 h-px bg-linear-to-r from-transparent via-white/15 to-transparent" />
    </div>
  );
}
