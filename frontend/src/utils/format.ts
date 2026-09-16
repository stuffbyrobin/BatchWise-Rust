/** Placeholder shown for a missing value. */
export const EMPTY = '—'

/** The date part (`YYYY-MM-DD`) of an ISO date or timestamp. */
export function fmtDate(s: string | null | undefined): string {
  if (!s) return EMPTY
  return String(s).slice(0, 10)
}

/** A timestamp as a short local date and time. */
export function fmtDateTime(s: string | null | undefined): string {
  if (!s) return EMPTY
  return new Date(s).toLocaleString(undefined, { dateStyle: 'short', timeStyle: 'short' })
}

/** An amount in pence as pounds, e.g. `12345` -> `£123.45`. */
export function fmtPence(p: number | null | undefined): string {
  if (p == null) return EMPTY
  return '£' + (p / 100).toFixed(2)
}
