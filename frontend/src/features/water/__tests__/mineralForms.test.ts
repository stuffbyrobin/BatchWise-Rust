import { describe, expect, it } from 'vitest'
import { defaultFormFor, mineralPayload, normalizeForm } from '../mineralForms'

describe('mineral forms', () => {
  it('defaults per salt', () => {
    expect(defaultFormFor('CaCl2')).toBe('hydrate')
    expect(defaultFormFor('Na2SO4')).toBe('anhydrous')
    expect(defaultFormFor('NaCl')).toBe('hydrate')
  })

  it('normalises the legacy dihydrate value and unknown input', () => {
    expect(normalizeForm('dihydrate', 'CaCl2')).toBe('hydrate')
    expect(normalizeForm('liquid', 'CaCl2')).toBe('liquid')
    expect(normalizeForm('bogus', 'Na2SO4')).toBe('anhydrous')
    expect(normalizeForm(undefined, 'MgSO4')).toBe('hydrate')
  })

  it('attaches form only for salts with a selector, and strength only for liquids', () => {
    expect(mineralPayload({ type: 'NaCl', amount: '2' })).toEqual({ type: 'NaCl', amount: 2 })
    expect(mineralPayload({ type: 'CaSO4', amount: '1.5', form: 'anhydrous' })).toEqual({
      type: 'CaSO4',
      amount: 1.5,
      form: 'anhydrous',
    })
    expect(mineralPayload({ type: 'CaCl2', amount: '10', form: 'liquid', strength: '33' })).toEqual({
      type: 'CaCl2',
      amount: 10,
      form: 'liquid',
      strength_pct: 33,
    })
  })
})
