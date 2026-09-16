import { describe, expect, it } from 'vitest'
import { nextSort, parseSort } from '../SortableHeader'

describe('parseSort', () => {
  it('reads the direction from a leading dash', () => {
    expect(parseSort('name')).toEqual({ col: 'name', dir: 'asc' })
    expect(parseSort('-created_at')).toEqual({ col: 'created_at', dir: 'desc' })
    expect(parseSort(undefined)).toBeNull()
    expect(parseSort('')).toBeNull()
  })
})

describe('nextSort', () => {
  it('sorts a new column ascending and flips the active one', () => {
    expect(nextSort(undefined, 'name')).toBe('name')
    expect(nextSort('-created_at', 'name')).toBe('name')
    expect(nextSort('name', 'name')).toBe('-name')
    expect(nextSort('-name', 'name')).toBe('name')
  })
})
