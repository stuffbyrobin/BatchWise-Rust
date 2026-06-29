// Shared config for salts that come in more than one supplied form. Salts not
// listed here (NaCl, NaHCO₃, CaCO₃, Ca(OH)₂) have no hydrate and get no
// selector. The wire values are generic: 'anhydrous' | 'hydrate' | 'liquid'
// (the backend resolves the specific molar mass per salt). The legacy CaCl₂
// value 'dihydrate' is normalised to 'hydrate'.

export type MineralFormValue = 'anhydrous' | 'hydrate' | 'liquid'

interface SaltFormConfig {
  /** Label for the 'hydrate' option (the specific hydrate for this salt). */
  hydrateLabel: string
  /** Default form for this salt. */
  default: MineralFormValue
  /** Whether a liquid %w/w solution form is offered (CaCl₂ only). */
  liquid?: boolean
}

export const SALT_FORMS: Record<string, SaltFormConfig> = {
  CaSO4: { hydrateLabel: 'Dihydrate', default: 'hydrate' },
  CaCl2: { hydrateLabel: 'Dihydrate', default: 'hydrate', liquid: true },
  MgSO4: { hydrateLabel: 'Heptahydrate', default: 'hydrate' },
  MgCl2: { hydrateLabel: 'Hexahydrate', default: 'hydrate' },
  Na2SO4: { hydrateLabel: "Decahydrate (Glauber's)", default: 'anhydrous' },
}

export function defaultFormFor(type: string): MineralFormValue {
  return SALT_FORMS[type]?.default ?? 'hydrate'
}

/** Normalise a stored form string to a valid option for `type`. */
export function normalizeForm(form: string | undefined, type: string): MineralFormValue {
  if (form === 'dihydrate') return 'hydrate'
  if (form === 'anhydrous' || form === 'hydrate' || form === 'liquid') return form
  return defaultFormFor(type)
}

export interface MineralRowLike {
  type: string
  amount: string
  form?: string
  strength?: string
}

export interface MineralPayload {
  type: string
  amount: number
  form?: string
  strength_pct?: number
}

/** Project an editor row to the API/WASM payload, attaching form/strength for
 *  salts that have a form selector. */
export function mineralPayload(m: MineralRowLike): MineralPayload {
  const out: MineralPayload = { type: m.type, amount: Number(m.amount) }
  if (SALT_FORMS[m.type]) {
    const form = normalizeForm(m.form, m.type)
    out.form = form
    if (form === 'liquid') out.strength_pct = Number(m.strength) || 0
  }
  return out
}
