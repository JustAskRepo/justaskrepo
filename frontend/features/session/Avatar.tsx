"use client";

import { useState } from "react";
import { avatarUrl, monogram } from "@/features/session/identity";
import type { Me } from "@/types/api";

/**
 * A plain <img>, deliberately: this is a static export with
 * `images.unoptimized`, so `next/image` would buy nothing for a 44px picture it
 * cannot optimise, while adding a remote-pattern entry to `next.config.ts`.
 *
 * `alt=""` because every caller already carries the accessible name (the
 * trigger button, or the name printed beside it in the panel) — a second
 * announcement of the same person is noise.
 */
export default function Avatar({ me, size }: { me: Me; size: number }) {
  const [broken, setBroken] = useState(false);

  if (broken) {
    return (
      <span className="flex h-full w-full items-center justify-center bg-linear-to-br from-accent to-accent-2 font-mono text-[11px] font-bold text-white">
        {monogram(me)}
      </span>
    );
  }

  return (
    // eslint-disable-next-line @next/next/no-img-element -- see note above
    <img
      src={avatarUrl(me, size)}
      alt=""
      width={size}
      height={size}
      decoding="async"
      onError={() => setBroken(true)}
      className="h-full w-full object-cover"
    />
  );
}
