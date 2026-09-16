import { describe, expect, it } from 'vitest'
import { safeRedirectPath } from '../redirect'

describe('safeRedirectPath', () => {
  it.each([
    ['/app/recipes?page=2', '/app/recipes?page=2'],
    [null, '/app'],
    [undefined, '/app'],
    ['', '/app'],
    ['//evil.example/path', '/app'],
    ['/\\evil.example', '/app'],
    ['https://evil.example', '/app'],
    ['javascript:alert(1)', '/app'],
    ['app/recipes', '/app'],
  ])('%s -> %s', (from, expected) => {
    expect(safeRedirectPath(from)).toBe(expected)
  })
})
