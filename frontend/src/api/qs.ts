/**
 * Builds a `?a=1&b=x` query string from `params`, skipping `undefined`, `null`
 * and empty-string values. Returns `''` when nothing is left.
 */
export function qs(params: Record<string, unknown>): string {
  const q = Object.entries(params)
    .filter(([, v]) => v !== undefined && v !== null && v !== '')
    .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(String(v))}`)
    .join('&')
  return q ? `?${q}` : ''
}
