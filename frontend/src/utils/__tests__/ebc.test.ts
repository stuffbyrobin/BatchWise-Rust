import { describe, expect, it } from 'vitest'
import { formatEbc } from '../ebc'

describe('formatEbc', () => {
  it('rounds to one decimal place', () => {
    expect(formatEbc(7.8812)).toBe('7.9')
    expect(formatEbc(10)).toBe('10.0')
  })

  it('uses the fallback for missing or NaN values', () => {
    expect(formatEbc(null)).toBe('-')
    expect(formatEbc(undefined, 'n/a')).toBe('n/a')
    expect(formatEbc(Number.NaN)).toBe('-')
  })
})
