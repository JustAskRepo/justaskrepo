import type { Me } from "@/types/api";

/**
 * How the signed-in user is rendered. Everything here degrades: `/api/me`
 * currently carries only ids, so the display side has to be derivable from
 * `github_id` alone and quietly improve when the backend sends the profile.
 */

/**
 * GitHub serves every account's picture from its numeric id, so the avatar is
 * available from `github_id` with no extra field and no second request. `s=`
 * asks for a square of that pixel size — pass 2x the rendered size for retina.
 */
export function avatarUrl(me: Me, size: number): string {
  const explicit = me.avatar_url?.trim();
  if (explicit) return explicit;
  return `https://avatars.githubusercontent.com/u/${me.github_id}?v=4&s=${size * 2}`;
}

/** Real name, else login. Null when `/api/me` sent neither — never a fake one. */
export function displayName(me: Me): string | null {
  return me.name?.trim() || me.username?.trim() || null;
}

/**
 * Headline in the panel. Falls back to a label rather than the raw id: an id
 * identifies, it doesn't name, and dropped into a title slot it just reads as
 * something broken.
 */
export function accountTitle(me: Me): string {
  return displayName(me) ?? "Your account";
}

/** The line beneath it — the login once we have one, the GitHub id until then. */
export function accountSubtitle(me: Me): string {
  const login = me.username?.trim();
  return login ? `@${login}` : `gh:${me.github_id}`;
}

/**
 * Drawn only if the avatar image fails. Falls back to the brand mark rather
 * than initialising a numeric id into something meaningless.
 */
export function monogram(me: Me): string {
  const source = displayName(me);
  if (!source) return "{ }";
  const parts = source.split(/[\s\-_]+/).filter(Boolean);
  const letters = parts.length > 1 ? parts[0][0] + parts[1][0] : source.slice(0, 2);
  return letters.toUpperCase();
}
