/**
 * Format an EBC colour value for display, rounded to one decimal place.
 * Computed colours (and some imported grain colours) carry many decimals;
 * brewers only care about ~1 dp. Returns `fallback` for null/undefined/NaN.
 */
export function formatEbc(ebc: number | null | undefined, fallback = '-'): string {
  if (ebc == null || Number.isNaN(ebc)) return fallback
  return ebc.toFixed(1)
}
