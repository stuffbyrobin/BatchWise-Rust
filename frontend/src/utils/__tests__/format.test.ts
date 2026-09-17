import { describe, expect, it } from 'vitest'
import { EMPTY, fmtDate, fmtDateTime, fmtPence } from '../format'

describe('format helpers', () => {
  it('fmtDate keeps the date part', () => {
    expect(fmtDate('2026-09-16T12:34:56Z')).toBe('2026-09-16')
    expect(fmtDate('2026-09-16')).toBe('2026-09-16')
    expect(fmtDate(null)).toBe(EMPTY)
    expect(fmtDate(undefined)).toBe(EMPTY)
  })

  it('fmtDateTime is empty for missing values', () => {
    expect(fmtDateTime(undefined)).toBe(EMPTY)
    expect(fmtDateTime('2026-09-16T12:34:56Z')).not.toBe(EMPTY)
  })

  it('fmtPence formats pounds with two decimals', () => {
    expect(fmtPence(12345)).toBe('£123.45')
    expect(fmtPence(0)).toBe('£0.00')
    expect(fmtPence(null)).toBe(EMPTY)
  })
})
