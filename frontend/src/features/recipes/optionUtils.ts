/** Midpoint of a min/max range, rounded to 1 dp; whichever bound exists otherwise. */
export function midpoint(min?: number | null, max?: number | null): number | undefined {
  if (min != null && max != null) return Math.round(((min + max) / 2) * 10) / 10
  return min ?? max ?? undefined
}

/**
 * Collapses per-lot inventory rows into one entry per name, summing the amount.
 * `pick` extracts the extra fields kept from the first lot seen for each name.
 */
export function aggregateStock<I extends { name: string; amount: number; unit: string }, X>(
  items: readonly I[],
  pick: (item: I) => X,
): Map<string, X & { amount: number; unit: string }> {
  const agg = new Map<string, X & { amount: number; unit: string }>()
  for (const it of items) {
    const cur = agg.get(it.name)
    if (cur) cur.amount += it.amount
    else agg.set(it.name, { ...pick(it), amount: it.amount, unit: it.unit })
  }
  return agg
}
