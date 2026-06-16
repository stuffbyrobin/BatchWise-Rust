// Loader + typed facade for the brewing-physics WASM module.
//
// The WASM is compiled from the Rust backend's `src/pkg/*` (see
// BatchWise-Rust/wasm), so these run the *exact same* calculations the server
// runs. `init()` is memoised: the module is fetched and instantiated once.

import init, * as wasm from './wasm/batchwise_physics_wasm.js'
// Vite resolves `?url` to the hashed asset URL for the .wasm binary.
import wasmUrl from './wasm/batchwise_physics_wasm_bg.wasm?url'

/** The pure physics functions exposed by the WASM bundle. */
export interface Physics {
  /** ABV % from original/final gravity (e.g. 1.050, 1.010 → 5.25). */
  calculateAbv(og: number, fg: number): number
  /** Apparent attenuation % from original/final gravity. */
  calculateAttenuation(og: number, fg: number): number
  /** Estimated calories per 12 oz from original/final gravity. */
  calculateCalories(og: number, fg: number): number
  /** Specific gravity → degrees Plato. */
  sgToPlato(sg: number): number
  /** Degrees Plato → specific gravity. */
  platoToSg(plato: number): number
  /** SRM → EBC. */
  srmToEbc(srm: number): number
  /** EBC → SRM. */
  ebcToSrm(ebc: number): number
  /** Degrees Lovibond → EBC. */
  lovibondToEbc(lovibond: number): number
  /** Energy (kJ) per 100 ml from ABV %. */
  energyKjPer100ml(abvPct: number): number
  /** Energy (kcal) per 100 ml from ABV %. */
  energyKcalPer100ml(abvPct: number): number
  /** UK alcohol units for a serving (ABV %, volume ml). */
  alcoholUnits(abvPct: number, volumeMl: number): number
  /** UK beer duty in pence for a volume (litres) at a given ABV %. */
  calculateBeerDutyGbPence(volumeLiters: number, abvPct: number): number
  /** Small Producer Relief rate (0–1) for annual production hl/year. */
  sprReliefRate(annualProductionHlPa: number): number
  /** Live recipe OG/FG/ABV/IBU/colour from the current grain & hop bill. */
  computeRecipeCalcs(input: RecipeCalcInput): RecipeCalcResult
}

/** Input to {@link Physics.computeRecipeCalcs}. */
export interface RecipeCalcInput {
  batch_size_liters: number
  efficiency_pct?: number | null
  /** First yeast's attenuation %, if any. */
  attenuation_pct?: number | null
  fermentables: {
    amount: number
    unit: string
    potential_ppg?: number | null
    color_ebc?: number | null
  }[]
  hops: {
    amount: number
    unit: string
    alpha_acid_pct?: number
    boil_time_minutes?: number
    form?: string | null
    use?: string | null
  }[]
}

/** Computed recipe values (camelCased plain object). */
export interface RecipeCalcResult {
  calcOg: number
  calcFg: number
  calcAbvPct: number
  calcIbu: number
  calcColorEbc: number
}

const facade: Physics = {
  calculateAbv: wasm.calculateAbv,
  calculateAttenuation: wasm.calculateAttenuation,
  calculateCalories: wasm.calculateCalories,
  sgToPlato: wasm.sgToPlato,
  platoToSg: wasm.platoToSg,
  srmToEbc: wasm.srmToEbc,
  ebcToSrm: wasm.ebcToSrm,
  lovibondToEbc: wasm.lovibondToEbc,
  energyKjPer100ml: wasm.energyKjPer100ml,
  energyKcalPer100ml: wasm.energyKcalPer100ml,
  alcoholUnits: wasm.alcoholUnits,
  calculateBeerDutyGbPence: wasm.calculateBeerDutyGbPence,
  sprReliefRate: wasm.sprReliefRate,
  computeRecipeCalcs(input: RecipeCalcInput): RecipeCalcResult {
    const c = wasm.computeRecipeCalcs(JSON.stringify(input))
    try {
      return {
        calcOg: c.calc_og,
        calcFg: c.calc_fg,
        calcAbvPct: c.calc_abv_pct,
        calcIbu: c.calc_ibu,
        calcColorEbc: c.calc_color_ebc,
      }
    } finally {
      c.free() // release the wasm-allocated struct
    }
  },
}

let ready: Promise<Physics> | null = null

/** Initialises the WASM module once and resolves the physics facade. */
export function loadPhysics(): Promise<Physics> {
  if (!ready) {
    ready = init({ module_or_path: wasmUrl }).then(() => facade)
  }
  return ready
}
