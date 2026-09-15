import { useMemo } from 'react'
import { useYeasts } from '../library/hooks/useLibrary'
import { useInventoryList } from '../inventory/hooks/useInventory'

export interface YeastOption {
  key: string
  /** Library yeast id; only set for generic (reference library) options. */
  id?: string
  name: string
  label: string
  attenuation_pct?: number
  /** Library strain type (ale/lager/…), used only for grouping. */
  type?: string
  source: 'stock' | 'generic'
}

export interface YeastGroup {
  label: string
  options: YeastOption[]
}

function midpoint(min?: number | null, max?: number | null): number | undefined {
  if (min != null && max != null) return Math.round(((min + max) / 2) * 10) / 10
  return min ?? max ?? undefined
}

const cap = (s: string) => (s ? s.charAt(0).toUpperCase() + s.slice(1) : s)

/**
 * Merges the generic yeast reference library and in-stock yeast inventory into
 * grouped selectable options for the recipe editor, mirroring useMaltOptions.
 */
export function useYeastOptions() {
  const generic = useYeasts({ page_size: 200 })
  const stock = useInventoryList({ type: 'yeast', page_size: 200 })

  const derived = useMemo(() => {
    const libItems = generic.data?.items ?? []
    const genericOptions: YeastOption[] = libItems
      .map((y) => ({
        key: `generic:${y.id}`,
        id: y.id,
        name: y.name,
        label: y.manufacturer ? `${y.name} (${y.manufacturer})` : y.name,
        attenuation_pct: midpoint(y.attenuation_min_pct, y.attenuation_max_pct),
        type: y.type ?? undefined,
        source: 'generic' as const,
      }))
      .sort((a, b) => a.name.localeCompare(b.name))

    // Attenuation by name, so a stock yeast can borrow it from its generic match.
    const attByName = new Map<string, number>()
    for (const g of genericOptions) {
      if (g.attenuation_pct != null) attByName.set(g.name.toLowerCase(), g.attenuation_pct)
    }

    const stockAgg = new Map<string, { att?: number; amount: number; unit: string }>()
    for (const it of stock.data?.items ?? []) {
      const cur = stockAgg.get(it.name)
      if (cur) cur.amount += it.amount
      else stockAgg.set(it.name, { att: it.attenuation_pct ?? undefined, amount: it.amount, unit: it.unit })
    }
    const stockOptions: YeastOption[] = [...stockAgg.entries()]
      .map(([name, v]) => ({
        key: `stock:${name}`,
        name,
        label: `${name} — ${v.amount} ${v.unit} in stock`,
        attenuation_pct: v.att ?? attByName.get(name.toLowerCase()),
        source: 'stock' as const,
      }))
      .sort((a, b) => a.name.localeCompare(b.name))

    const options: YeastOption[] = [...stockOptions, ...genericOptions]
    const byKey = new Map(options.map((o) => [o.key, o]))
    const byName = new Map<string, YeastOption>()
    for (const o of options) if (!byName.has(o.name)) byName.set(o.name, o)

    const groups: YeastGroup[] = []
    if (stockOptions.length) groups.push({ label: 'In stock', options: stockOptions })
    const byType = new Map<string, YeastOption[]>()
    for (const g of genericOptions) {
      const t = cap(g.type || '') || 'Other'
      if (!byType.has(t)) byType.set(t, [])
      byType.get(t)!.push(g)
    }
    for (const t of [...byType.keys()].sort()) groups.push({ label: t, options: byType.get(t)! })

    return { byKey, byName, groups }
  }, [generic.data, stock.data])

  return { ...derived, loading: generic.isLoading || stock.isLoading }
}
