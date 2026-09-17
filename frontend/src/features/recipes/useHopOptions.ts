import { useMemo } from 'react'
import { useAllPages } from '../../api/allPages'
import type { components } from '../../api/generated'
import { aggregateStock } from './optionUtils'

type Ingredient = components['schemas']['Ingredient']

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
  const stock = useAllPages<Ingredient>(['inventory'], '/api/v1/inventory', { type: 'hop' })

  const derived = useMemo(() => {
    const agg = aggregateStock(stock.data ?? [], (it) => ({ alpha: it.alpha_acid_pct ?? undefined }))
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
    return { byKey, byName, groups }
  }, [stock.data])

  return { ...derived, loading: stock.isLoading }
}
