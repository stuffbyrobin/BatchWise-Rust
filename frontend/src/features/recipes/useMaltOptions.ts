import { useMemo } from 'react'
import { useAllPages } from '../../api/allPages'
import type { components } from '../../api/generated'
import { midpoint, aggregateStock } from './optionUtils'

type LibraryFermentable = components['schemas']['LibraryFermentable']
type Ingredient = components['schemas']['Ingredient']

export interface MaltOption {
  /** Unique select value, e.g. "generic:<uuid>" or "stock:<name>". */
  key: string
  /** Bare malt name used for auto-fill and for matching an existing row. */
  name: string
  /** Display text (stock options append the quantity on hand). */
  label: string
  type?: string
  color_ebc?: number
  potential_ppg?: number
  source: 'stock' | 'generic'
}

export interface MaltGroup {
  label: string
  options: MaltOption[]
}

// Hot Water Extract (litre-degrees per kg) → PPG (points/lb/US gal): ÷ 8.3454.
function extractToPpg(lkg?: number | null): number | undefined {
  if (lkg == null || lkg <= 0) return undefined
  return Math.round((lkg / 8.3454) * 10) / 10
}



// Display generic malts in a sensible order; unknown types fall to the end.
const GENERIC_TYPE_ORDER = [
  'Base Malt',
  'Heritage Malt',
  'Kilned',
  'Crystal',
  'Roasted',
  'Specialty',
  'Adjunct',
  'Distilling',
]

/**
 * Merges in-stock fermentable inventory and the generic reference fermentable
 * library into selectable malt options for the recipe editor. Returns the flat
 * list plus lookup maps and a grouped view for `<optgroup>` rendering.
 */
export function useMaltOptions() {
  const generic = useAllPages<LibraryFermentable>(['library', 'fermentables'], '/api/v1/library/fermentables')
  const stock = useAllPages<Ingredient>(['inventory'], '/api/v1/inventory', { type: 'fermentable' })

  const derived = useMemo(() => {
    const genericOptions: MaltOption[] = (generic.data ?? [])
      .map((f) => ({
        key: `generic:${f.id}`,
        name: f.name,
        label: f.name,
        type: f.type ?? undefined,
        color_ebc: midpoint(f.colour_ebc_min, f.colour_ebc_max),
        potential_ppg: extractToPpg(f.extract_litres_per_kg),
        source: 'generic' as const,
      }))
      .sort((a, b) => a.name.localeCompare(b.name))

    // PPG by name, so a stock malt can borrow potential from its generic match.
    const ppgByName = new Map<string, number>()
    for (const g of genericOptions) {
      if (g.potential_ppg != null) ppgByName.set(g.name.toLowerCase(), g.potential_ppg)
    }

    // Inventory is per-lot; collapse to one option per name, summing quantity.
    const stockAgg = aggregateStock(stock.data ?? [], (it) => ({ color: it.color_ebc ?? undefined }))
    const stockOptions: MaltOption[] = [...stockAgg.entries()]
      .map(([name, v]) => ({
        key: `stock:${name}`,
        name,
        label: `${name} — ${v.amount} ${v.unit} in stock`,
        type: 'In stock',
        color_ebc: v.color,
        potential_ppg: ppgByName.get(name.toLowerCase()),
        source: 'stock' as const,
      }))
      .sort((a, b) => a.name.localeCompare(b.name))

    const options: MaltOption[] = [...stockOptions, ...genericOptions]
    const byKey = new Map(options.map((o) => [o.key, o]))

    // Match an existing row name to an option, preferring stock.
    const byName = new Map<string, MaltOption>()
    for (const o of options) if (!byName.has(o.name)) byName.set(o.name, o)

    // Grouped view: In stock first, then generic types in order.
    const groups: MaltGroup[] = []
    if (stockOptions.length) groups.push({ label: 'In stock', options: stockOptions })
    const byType = new Map<string, MaltOption[]>()
    for (const g of genericOptions) {
      const t = g.type || 'Other'
      if (!byType.has(t)) byType.set(t, [])
      byType.get(t)!.push(g)
    }
    const orderedTypes = [
      ...GENERIC_TYPE_ORDER.filter((t) => byType.has(t)),
      ...[...byType.keys()].filter((t) => !GENERIC_TYPE_ORDER.includes(t)).sort(),
    ]
    for (const t of orderedTypes) groups.push({ label: t, options: byType.get(t)! })

    return { options, byKey, byName, groups }
  }, [generic.data, stock.data])

  return { ...derived, loading: generic.isLoading || stock.isLoading }
}
