import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { _initTokenStore, apiClient } from '../client'
import { APIError } from '../error'

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  })
}

describe('apiClient 401 handling', () => {
  let access: string | null
  let refresh: string | null
  const clear = vi.fn(() => {
    access = null
    refresh = null
  })

  beforeEach(() => {
    access = 'expired'
    refresh = 'r1'
    clear.mockClear()
    _initTokenStore(
      () => access,
      () => refresh,
      (a, r) => {
        access = a
        refresh = r
      },
      clear,
    )
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('three parallel 401s trigger exactly one refresh, then all retry', async () => {
    let refreshCalls = 0
    vi.stubGlobal(
      'fetch',
      vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
        const url = String(input)
        if (url === '/api/v1/auth/refresh') {
          refreshCalls += 1
          await new Promise((resolve) => setTimeout(resolve, 10))
          return json(200, { access_token: 'fresh', refresh_token: 'r2' })
        }
        const auth = (init?.headers as Record<string, string> | undefined)?.Authorization
        return auth === 'Bearer fresh'
          ? json(200, { url })
          : json(401, { code: 'unauthorized', message: 'expired' })
      }),
    )

    const results = await Promise.all([
      apiClient.get('/api/v1/a'),
      apiClient.get('/api/v1/b'),
      apiClient.get('/api/v1/c'),
    ])

    expect(refreshCalls).toBe(1)
    expect(results).toEqual([{ url: '/api/v1/a' }, { url: '/api/v1/b' }, { url: '/api/v1/c' }])
    expect(refresh).toBe('r2')
    expect(clear).not.toHaveBeenCalled()
  })

  it('a failed refresh clears the tokens and rejects without navigating', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => json(401, { code: 'unauthorized', message: 'nope' })),
    )
    const before = window.location.href

    await expect(apiClient.get('/api/v1/a')).rejects.toBeInstanceOf(APIError)

    expect(clear).toHaveBeenCalledTimes(1)
    expect(window.location.href).toBe(before)
  })

  it('a 401 from login keeps the server message and never refreshes', async () => {
    const fetchMock = vi.fn(async () =>
      json(401, { code: 'unauthorized', message: 'invalid credentials', request_id: 'r' }),
    )
    vi.stubGlobal('fetch', fetchMock)

    await expect(
      apiClient.post('/api/v1/auth/login', { email: 'a@b.c', password: 'x' }),
    ).rejects.toMatchObject({ status: 401, message: 'invalid credentials' })

    expect(fetchMock).toHaveBeenCalledTimes(1)
    expect(clear).not.toHaveBeenCalled()
  })

  it('passes the caller abort signal to fetch', async () => {
    const fetchMock = vi.fn(async (_input: RequestInfo | URL, _init?: RequestInit) => json(200, {}))
    vi.stubGlobal('fetch', fetchMock)
    const controller = new AbortController()

    await apiClient.get('/api/v1/a', { signal: controller.signal })

    const init = fetchMock.mock.calls[0][1] as RequestInit
    expect(init.signal).toBe(controller.signal)
  })
})
