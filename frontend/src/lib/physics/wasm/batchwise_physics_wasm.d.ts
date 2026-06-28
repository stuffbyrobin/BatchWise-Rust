/* tslint:disable */
/* eslint-disable */
/**
 * Computes the treated-water profile + predicted mash pH from a JSON payload.
 */
export function computeWaterTreatment(input_json: string): WaterTreatment;
/**
 * Computes OG/FG/ABV/IBU/colour from a recipe-form JSON payload.
 */
export function computeRecipeCalcs(input_json: string): RecipeCalcs;
/**
 * Apparent attenuation % from original and final gravity.
 */
export function calculateAttenuation(og: number, fg: number): number;
/**
 * Energy (kcal) per 100 ml from ABV %.
 */
export function energyKcalPer100ml(abv_pct: number): number;
/**
 * Specific gravity → degrees Plato.
 */
export function sgToPlato(sg: number): number;
/**
 * Degrees Plato → specific gravity.
 */
export function platoToSg(plato: number): number;
/**
 * UK beer duty in **pence** for a volume (litres) at a given ABV %.
 */
export function calculateBeerDutyGbPence(volume_liters: number, abv_pct: number): number;
/**
 * Small Producer Relief rate (0.0–1.0) for an annual production in hl/year.
 */
export function sprReliefRate(annual_production_hl_pa: number): number;
/**
 * Energy (kJ) per 100 ml from ABV %.
 */
export function energyKjPer100ml(abv_pct: number): number;
/**
 * EBC → SRM.
 */
export function ebcToSrm(ebc: number): number;
/**
 * Degrees Lovibond → EBC.
 */
export function lovibondToEbc(lovibond: number): number;
/**
 * ABV % from original and final gravity (e.g. `1.050, 1.010` → `5.25`).
 */
export function calculateAbv(og: number, fg: number): number;
/**
 * SRM → EBC.
 */
export function srmToEbc(srm: number): number;
/**
 * UK alcohol units for a serving (ABV % and volume in ml).
 */
export function alcoholUnits(abv_pct: number, volume_ml: number): number;
/**
 * Estimated calories per 12 oz from original and final gravity.
 */
export function calculateCalories(og: number, fg: number): number;
/**
 * The computed recipe values, surfaced to JS with camelCase getters.
 */
export class RecipeCalcs {
  private constructor();
  free(): void;
  readonly calc_abv_pct: number;
  readonly calc_color_ebc: number;
  readonly calc_fg: number;
  readonly calc_og: number;
  readonly calc_ibu: number;
}
/**
 * The computed water-treatment values, surfaced to JS with snake_case getters.
 */
export class WaterTreatment {
  private constructor();
  free(): void;
  readonly alkalinity: number;
  readonly sodium_ppm: number;
  readonly calcium_ppm: number;
  readonly sulfate_ppm: number;
  readonly chloride_ppm: number;
  readonly residual_alk: number;
  readonly magnesium_ppm: number;
  readonly bicarbonate_ppm: number;
  readonly sulfate_to_chloride: number;
  readonly mash_ph: number;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_recipecalcs_free: (a: number, b: number) => void;
  readonly __wbg_watertreatment_free: (a: number, b: number) => void;
  readonly calculateAbv: (a: number, b: number, c: number) => void;
  readonly calculateAttenuation: (a: number, b: number, c: number) => void;
  readonly calculateBeerDutyGbPence: (a: number, b: number) => number;
  readonly calculateCalories: (a: number, b: number, c: number) => void;
  readonly computeRecipeCalcs: (a: number, b: number, c: number) => void;
  readonly computeWaterTreatment: (a: number, b: number, c: number) => void;
  readonly platoToSg: (a: number) => number;
  readonly recipecalcs_calc_abv_pct: (a: number) => number;
  readonly recipecalcs_calc_color_ebc: (a: number) => number;
  readonly recipecalcs_calc_fg: (a: number) => number;
  readonly recipecalcs_calc_ibu: (a: number) => number;
  readonly recipecalcs_calc_og: (a: number) => number;
  readonly watertreatment_alkalinity: (a: number) => number;
  readonly watertreatment_bicarbonate_ppm: (a: number) => number;
  readonly watertreatment_calcium_ppm: (a: number) => number;
  readonly watertreatment_chloride_ppm: (a: number) => number;
  readonly watertreatment_magnesium_ppm: (a: number) => number;
  readonly watertreatment_mash_ph: (a: number) => number;
  readonly watertreatment_residual_alk: (a: number) => number;
  readonly watertreatment_sodium_ppm: (a: number) => number;
  readonly watertreatment_sulfate_ppm: (a: number) => number;
  readonly watertreatment_sulfate_to_chloride: (a: number) => number;
  readonly alcoholUnits: (a: number, b: number) => number;
  readonly srmToEbc: (a: number) => number;
  readonly energyKcalPer100ml: (a: number) => number;
  readonly sgToPlato: (a: number) => number;
  readonly lovibondToEbc: (a: number) => number;
  readonly energyKjPer100ml: (a: number) => number;
  readonly sprReliefRate: (a: number) => number;
  readonly ebcToSrm: (a: number) => number;
  readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
  readonly __wbindgen_export_0: (a: number, b: number) => number;
  readonly __wbindgen_export_1: (a: number, b: number, c: number, d: number) => number;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
