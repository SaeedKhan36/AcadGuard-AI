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
  return /^\/dashboard(?:[/?#]|$)/.test(requested) ? requested : fallback;
}
