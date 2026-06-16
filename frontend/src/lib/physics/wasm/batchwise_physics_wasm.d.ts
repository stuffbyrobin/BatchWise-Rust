/* tslint:disable */
/* eslint-disable */
/**
 * Energy (kcal) per 100 ml from ABV %.
 */
export function energyKcalPer100ml(abv_pct: number): number;
/**
 * UK beer duty in **pence** for a volume (litres) at a given ABV %.
 */
export function calculateBeerDutyGbPence(volume_liters: number, abv_pct: number): number;
/**
 * Apparent attenuation % from original and final gravity.
 */
export function calculateAttenuation(og: number, fg: number): number;
/**
 * UK alcohol units for a serving (ABV % and volume in ml).
 */
export function alcoholUnits(abv_pct: number, volume_ml: number): number;
/**
 * Estimated calories per 12 oz from original and final gravity.
 */
export function calculateCalories(og: number, fg: number): number;
/**
 * EBC → SRM.
 */
export function ebcToSrm(ebc: number): number;
/**
 * Specific gravity → degrees Plato.
 */
export function sgToPlato(sg: number): number;
/**
 * Degrees Plato → specific gravity.
 */
export function platoToSg(plato: number): number;
/**
 * Small Producer Relief rate (0.0–1.0) for an annual production in hl/year.
 */
export function sprReliefRate(annual_production_hl_pa: number): number;
/**
 * Energy (kJ) per 100 ml from ABV %.
 */
export function energyKjPer100ml(abv_pct: number): number;
/**
 * ABV % from original and final gravity (e.g. `1.050, 1.010` → `5.25`).
 */
export function calculateAbv(og: number, fg: number): number;
/**
 * SRM → EBC.
 */
export function srmToEbc(srm: number): number;
/**
 * Degrees Lovibond → EBC.
 */
export function lovibondToEbc(lovibond: number): number;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly calculateAbv: (a: number, b: number, c: number) => void;
  readonly calculateAttenuation: (a: number, b: number, c: number) => void;
  readonly calculateBeerDutyGbPence: (a: number, b: number) => number;
  readonly calculateCalories: (a: number, b: number, c: number) => void;
  readonly platoToSg: (a: number) => number;
  readonly alcoholUnits: (a: number, b: number) => number;
  readonly sprReliefRate: (a: number) => number;
  readonly srmToEbc: (a: number) => number;
  readonly energyKcalPer100ml: (a: number) => number;
  readonly sgToPlato: (a: number) => number;
  readonly lovibondToEbc: (a: number) => number;
  readonly energyKjPer100ml: (a: number) => number;
  readonly ebcToSrm: (a: number) => number;
  readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
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
