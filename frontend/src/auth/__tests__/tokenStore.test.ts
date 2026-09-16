import { beforeEach, describe, expect, it } from 'vitest'
import { tokenStore } from '../tokenStore'

describe('tokenStore persistence', () => {
  beforeEach(() => {
    tokenStore.clear()
    sessionStorage.clear()
  })

  it('persists only the refresh token', () => {
    tokenStore.setTokens('access-secret', 'refresh-1')

    const saved = JSON.parse(sessionStorage.getItem('batchwise-auth') ?? '{}')
    expect(saved.state).toEqual({ refreshToken: 'refresh-1' })
    expect(JSON.stringify(saved)).not.toContain('access-secret')
  })
})
