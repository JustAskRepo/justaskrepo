"use client";

import { useRef } from "react";

/**
 * Glass card that tracks the pointer and feeds `--mx` / `--my` to the
 * `.card-glow` radial highlight in globals.css, so the sheen follows the cursor
 * instead of sitting in a fixed spot. Falls back to the CSS defaults on touch
 * and keyboard, where no pointer position exists.
 */
export default function SpotlightCard({
  children,
  className = "",
  style,
}: {
  children: React.ReactNode;
  className?: string;
  style?: React.CSSProperties;
}) {
  const ref = useRef<HTMLDivElement>(null);

  function handlePointerMove(event: React.PointerEvent<HTMLDivElement>) {
    const el = ref.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    el.style.setProperty("--mx", `${event.clientX - rect.left}px`);
    el.style.setProperty("--my", `${event.clientY - rect.top}px`);
  }

  return (
    <div
      ref={ref}
      onPointerMove={handlePointerMove}
      style={style}
      className={`glass card-glow ${className}`}
    >
      {children}
    </div>
  );
}
