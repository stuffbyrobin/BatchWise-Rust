import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import { MemoryRouter } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { AuthProvider } from '../AuthProvider'

describe('AuthProvider', () => {
  it('mounts without errors', () => {
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    render(
      <QueryClientProvider client={qc}>
        <MemoryRouter>
          <AuthProvider>
            <div data-testid="child">hello</div>
          </AuthProvider>
        </MemoryRouter>
      </QueryClientProvider>,
    )
    expect(screen.getByTestId('child')).toBeInTheDocument()
  })
})
