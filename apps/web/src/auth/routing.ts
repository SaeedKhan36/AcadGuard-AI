/** Keep links from the original hash-based prototype working without bypassing auth. */
export function legacyDestination(hash: string): string | null {
  if (hash === "#/" || hash === "#") return "/";
  if (hash === "#/login") return "/auth/login";
  if (/^#\/app(?:[/?]|$)/.test(hash))
    return hash.replace(/^#\/app/, "/dashboard");
  return null;
}

/** Only return to local workspace pages, never a caller-supplied external URL. */
export function authDestination(
  search: string,
  fallback = "/dashboard",
): string {
  const requested = new URLSearchParams(search).get("redirect_url");
  if (!requested || /[\\\u0000-\u0020]/.test(requested)) return fallback;
  if (requested.startsWith("//")) return fallback;
  try {
    // Clerk can convert the relative target into an absolute URL when switching
    // between sign-in and sign-up. Discard its origin; only navigate locally.
    const url = new URL(requested, "http://app.local");
    if (!/^https?:$/.test(url.protocol)) return fallback;
    const local = url.pathname + url.search + url.hash;
    return /^\/dashboard(?:[/?#]|$)/.test(local) ? local : fallback;
  } catch {
    return fallback;
  }
}
