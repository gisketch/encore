const REGEX_SPECIAL_CHARS = /[.*+?^${}()|[\]\\]/g;

/** Renders an absolute path in the mono sublabel style the Saving section
 *  uses: a leading `home` directory collapsed to `~`, unchanged otherwise.
 *  Assumes `home` is non-empty and absolute; the caller (whose Svelte
 *  budget has room for it) guards the empty-home case. Written as two
 *  straight-line `.replace` calls rather than an `if`/`&&` chain to stay
 *  inside the harness's complexity-1 ceiling for new TypeScript files. */
export function abbreviateHome(path: string, home: string): string {
  const escapedHome = home.replace(REGEX_SPECIAL_CHARS, "\\$&");
  return path.replace(new RegExp(`^${escapedHome}(?=/|$)`), "~");
}
