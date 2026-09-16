/**
 * Returns `from` when it is a same-origin path, otherwise `fallback`.
 *
 * Guards the post-login redirect: `/login?from=//evil.example` or
 * `from=https://evil.example` must not send the user off-site.
 */
export function safeRedirectPath(from: string | null | undefined, fallback = '/app'): string {
  if (!from) return fallback
  if (!from.startsWith('/') || from.startsWith('//') || from.startsWith('/\\')) return fallback
  return from
}
