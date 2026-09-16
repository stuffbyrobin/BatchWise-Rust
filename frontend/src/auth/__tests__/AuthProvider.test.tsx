import { act, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { AuthProvider } from '../AuthProvider'
import { tokenStore } from '../tokenStore'
import { useAuth } from '../useAuth'

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  })
}

type Auth = ReturnType<typeof useAuth>

function renderWithAuth(onAuth: (auth: Auth) => void) {
  function Probe() {
    const auth = useAuth()
    onAuth(auth)
    return <span data-testid="user">{auth.user?.email ?? 'none'}</span>
  }
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter>
        <AuthProvider>
          <Probe />
        </AuthProvider>
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

describe('AuthProvider', () => {
  beforeEach(() => {
    tokenStore.clear()
    sessionStorage.clear()
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('mounts without errors', () => {
    renderWithAuth(() => {})
    expect(screen.getByTestId('user')).toHaveTextContent('none')
  })

  it('logout clears the session even when the server call fails', async () => {
    tokenStore.setTokens('a1', 'r1')
    const fetchMock = vi.fn(async (input: RequestInfo | URL, _init?: RequestInit) => {
      if (String(input) === '/api/v1/auth/me') return json(200, { email: 'brewer@example.com' })
      throw new TypeError('network down')
    })
    vi.stubGlobal('fetch', fetchMock)

    let auth: Auth | null = null
    renderWithAuth((a) => {
      auth = a
    })
    expect(await screen.findByText('brewer@example.com')).toBeInTheDocument()

    await act(async () => {
      await expect(auth!.logout()).rejects.toThrow('network down')
    })

    const logoutCall = fetchMock.mock.calls.find(([url]) => String(url) === '/api/v1/auth/logout')
    expect(JSON.parse(String((logoutCall?.[1] as RequestInit).body))).toEqual({ refresh_token: 'r1' })
    expect(tokenStore.getAccessToken()).toBeNull()
    expect(tokenStore.getRefreshToken()).toBeNull()
    expect(screen.getByTestId('user')).toHaveTextContent('none')
  })

  it('drops the user when the tokens are cleared elsewhere (failed refresh)', async () => {
    tokenStore.setTokens('a1', 'r1')
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => json(200, { email: 'brewer@example.com' })),
    )
    renderWithAuth(() => {})
    expect(await screen.findByText('brewer@example.com')).toBeInTheDocument()

    act(() => {
      tokenStore.clear()
    })

    expect(screen.getByTestId('user')).toHaveTextContent('none')
  })
})
