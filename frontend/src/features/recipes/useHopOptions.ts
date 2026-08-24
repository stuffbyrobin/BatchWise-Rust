import { useInventoryList } from '../inventory/hooks/useInventory'

export interface HopOption {
  key: string
  name: string
  label: string
  alpha_acid_pct?: number
  source: 'stock'
}

export interface HopGroup {
  label: string
  options: HopOption[]
}

/**
 * Selectable hop options for the recipe editor. There is no generic hop
 * reference library yet, so options come from in-stock inventory only (plus
 * the editor's Custom/Other entry). Auto-fills name + alpha acid %.
 */
export function useHopOptions() {
  const stock = useInventoryList({ type: 'hop', page_size: 200 })

  const agg = new Map<string, { alpha?: number; amount: number; unit: string }>()
  for (const it of stock.data?.items ?? []) {
    const cur = agg.get(it.name)
    if (cur) cur.amount += it.amount
    else agg.set(it.name, { alpha: it.alpha_acid_pct ?? undefined, amount: it.amount, unit: it.unit })
  }
  const options: HopOption[] = [...agg.entries()]
    .map(([name, v]) => ({
      key: `stock:${name}`,
      name,
      label: `${name} — ${v.amount} ${v.unit} in stock`,
      alpha_acid_pct: v.alpha,
      source: 'stock' as const,
    }))
    .sort((a, b) => a.name.localeCompare(b.name))

  const byKey = new Map(options.map((o) => [o.key, o]))
  const byName = new Map<string, HopOption>()
  for (const o of options) if (!byName.has(o.name)) byName.set(o.name, o)
  const groups: HopGroup[] = options.length ? [{ label: 'In stock', options }] : []

  return { byKey, byName, groups, loading: stock.isLoading }
}
