import { describe, expect, it } from 'vitest'
import { calcHopIBU } from '../ibu'

// Reference values match the backend (src/pkg/bitterness.rs):
// 28 g of 10 % AA hops boiled 60 min in 20 L.
describe('calcHopIBU', () => {
  it('tinseth matches the backend formula', () => {
    expect(calcHopIBU('tinseth', 28, 10, 60, 20)).toBeCloseTo(32.29, 1)
  })

  it('tinseth utilisation falls as gravity rises', () => {
    expect(calcHopIBU('tinseth', 28, 10, 60, 20, 1.08)).toBeCloseTo(24.66, 1)
  })

  it('rager, with the high-gravity correction above 1.050', () => {
    expect(calcHopIBU('rager', 28, 10, 60, 20)).toBeCloseTo(43.15, 1)
    expect(calcHopIBU('rager', 28, 10, 60, 20, 1.09)).toBeCloseTo(35.96, 1)
  })

  it('is zero without weight, alpha acid or volume, or with no boil', () => {
    expect(calcHopIBU('tinseth', 0, 10, 60, 20)).toBe(0)
    expect(calcHopIBU('tinseth', 28, 0, 60, 20)).toBe(0)
    expect(calcHopIBU('tinseth', 28, 10, 60, 0)).toBe(0)
    expect(calcHopIBU('tinseth', 28, 10, 0, 20)).toBe(0)
  })
})
