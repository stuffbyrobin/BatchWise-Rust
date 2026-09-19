import { describe, expect, it } from 'vitest'
import { passwordProblem, PASSWORD_MIN_LENGTH } from '../password'

describe('passwordProblem', () => {
  it('accepts a password that meets the server rules', () => {
    expect(passwordProblem('Sup3rSecret!pw')).toBeNull()
  })

  it('names the first rule that fails', () => {
    // The same cases the server checks, in the same order.
    expect(passwordProblem('Sh0rt!aa')).toBe(`Password must be at least ${PASSWORD_MIN_LENGTH} characters.`)
    expect(passwordProblem(`${'a1!'.repeat(50)}Bb`)).toBe('Password must be at most 128 characters.')
    expect(passwordProblem('sup3rsecret!pw')).toBe('Password must contain an uppercase letter.')
    expect(passwordProblem('SUP3RSECRET!PW')).toBe('Password must contain a lowercase letter.')
    expect(passwordProblem('SuperSecret!pw')).toBe('Password must contain a digit.')
    expect(passwordProblem('Sup3rSecretpw')).toBe('Password must contain a symbol.')
  })

  it('counts accented letters and non-ASCII symbols like the server does', () => {
    expect(passwordProblem('Übergäriges1§')).toBeNull()
  })
})
