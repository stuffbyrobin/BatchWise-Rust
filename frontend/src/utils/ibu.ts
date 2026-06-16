export type IBUMethod = 'tinseth' | 'rager'

function tinsethUtilization(boilMinutes: number, og: number): number {
  const bigness = 1.65 * Math.pow(0.000125, og - 1)
  const boilFactor = (1 - Math.exp(-0.04 * boilMinutes)) / 4.15
  return bigness * boilFactor
}

function ragerUtilization(boilMinutes: number): number {
  return (18.11 + 13.86 * Math.tanh((boilMinutes - 31.32) / 18.27)) / 100
}

/**
 * Calculate IBU contribution for a single hop addition.
 * @param method  'tinseth' | 'rager'
 * @param weightG hop weight in grams
 * @param alphaPct alpha acid percentage (0–100)
 * @param boilMinutes boil time in minutes
 * @param batchVolL batch volume in litres
 * @param og wort original gravity (default 1.050)
 */
export function calcHopIBU(
  method: IBUMethod,
  weightG: number,
  alphaPct: number,
  boilMinutes: number,
  batchVolL: number,
  og = 1.050,
): number {
  if (!weightG || !alphaPct || !batchVolL) return 0
  const alphaFraction = alphaPct / 100

  if (method === 'tinseth') {
    const util = tinsethUtilization(boilMinutes, og)
    // IBU = W_g × AA × U × 10 / V_L  (derived from original oz/gallon formula)
    return (weightG * alphaFraction * util * 10) / batchVolL
  }

  // Rager
  const util = ragerUtilization(boilMinutes)
  // Gravity correction: wort above 1.050 reduces utilisation
  const gravityAdj = og > 1.050 ? 1 + (og - 1.050) / 0.2 : 1
  return (weightG * alphaFraction * util * 10) / (batchVolL * gravityAdj)
}
