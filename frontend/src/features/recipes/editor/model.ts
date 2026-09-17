import type { EmptyAs } from './useIngredientRows'

export type RecipeType = 'all_grain' | 'extract' | 'partial_mash' | 'cider' | 'mead' | 'other'
export type FermentableUnit = 'kg' | 'g'
export type HopUnit = 'g' | 'kg'
export type YeastUnit = 'g' | 'mL' | 'count'
export type HopForm = 'pellet' | 'leaf' | 'extract'
export type HopUse = 'boil' | 'whirlpool' | 'dry-hop' | 'first-wort' | 'mash'
export type MashStepType = 'infusion' | 'temperature' | 'decoction'

export type Fermentable = {
  step_order: number
  name: string
  amount: number
  unit: FermentableUnit
  color_ebc?: number | null
  potential_ppg?: number | null
  type?: string
  addition?: string
}

export type Hop = {
  step_order: number
  name: string
  amount: number
  unit: HopUnit
  alpha_acid_pct: number
  boil_time_minutes: number
  form?: HopForm
  use?: HopUse
}

export type Yeast = {
  yeast_id?: string
  name: string
  amount: number
  unit: YeastUnit
  attenuation_pct?: number | null
}

export type MashStep = {
  step_order: number
  step_type: MashStepType
  target_temp_c: number
  hold_minutes: number
  infusion_volume_liters?: number | null
}

// Numeric columns and what an emptied input stores: required values become 0,
// optional measurements null (unknown rather than a real zero).
export const FERMENTABLE_NUMERIC: Partial<Record<keyof Fermentable, EmptyAs>> = {
  step_order: 'zero',
  amount: 'zero',
  color_ebc: 'null',
  potential_ppg: 'null',
}
export const HOP_NUMERIC: Partial<Record<keyof Hop, EmptyAs>> = {
  step_order: 'zero',
  amount: 'zero',
  alpha_acid_pct: 'zero',
  boil_time_minutes: 'zero',
}
export const YEAST_NUMERIC: Partial<Record<keyof Yeast, EmptyAs>> = { amount: 'zero', attenuation_pct: 'null' }
export const MASH_NUMERIC: Partial<Record<keyof MashStep, EmptyAs>> = {
  step_order: 'zero',
  target_temp_c: 'zero',
  hold_minutes: 'zero',
  infusion_volume_liters: 'null',
}

export const newFermentable = (step_order: number): Fermentable => ({ step_order, name: '', amount: 0, unit: 'kg' })
export const newHop = (step_order: number): Hop => ({
  step_order,
  name: '',
  amount: 0,
  unit: 'g',
  alpha_acid_pct: 0,
  boil_time_minutes: 0,
})
export const newYeast = (): Yeast => ({ name: '', amount: 0, unit: 'g' })
export const newMashStep = (step_order: number): MashStep => ({
  step_order,
  step_type: 'infusion',
  target_temp_c: 0,
  hold_minutes: 0,
})

export type Choice<V extends string> = { value: V; label: string }

export const TYPE_OPTIONS: Choice<RecipeType>[] = [
  { value: 'all_grain', label: 'All Grain' },
  { value: 'extract', label: 'Extract' },
  { value: 'partial_mash', label: 'Partial Mash' },
  { value: 'cider', label: 'Cider' },
  { value: 'mead', label: 'Mead' },
  { value: 'other', label: 'Other' },
]
export const FERMENTABLE_UNITS: Choice<FermentableUnit>[] = [
  { value: 'kg', label: 'kg' },
  { value: 'g', label: 'g' },
]
export const HOP_UNITS: Choice<HopUnit>[] = [
  { value: 'g', label: 'g' },
  { value: 'kg', label: 'kg' },
]
export const YEAST_UNITS: Choice<YeastUnit>[] = [
  { value: 'g', label: 'g' },
  { value: 'mL', label: 'mL' },
  { value: 'count', label: 'count' },
]
export const MASH_STEP_TYPES: Choice<MashStepType>[] = [
  { value: 'infusion', label: 'Infusion' },
  { value: 'temperature', label: 'Temperature' },
  { value: 'decoction', label: 'Decoction' },
]
export const HOP_FORMS: Choice<HopForm>[] = [
  { value: 'pellet', label: 'Pellet' },
  { value: 'leaf', label: 'Leaf' },
  { value: 'extract', label: 'Extract' },
]
export const HOP_USES: Choice<HopUse>[] = [
  { value: 'boil', label: 'Boil' },
  { value: 'whirlpool', label: 'Whirlpool' },
  { value: 'dry-hop', label: 'Dry Hop' },
  { value: 'first-wort', label: 'First Wort' },
  { value: 'mash', label: 'Mash' },
]

/** Input styling for the recipe editor's table cells. */
export const editorInputCls =
  'border border-[var(--color-border)] rounded px-2 py-1 bg-[var(--color-surface)] text-[var(--color-fg)]'
