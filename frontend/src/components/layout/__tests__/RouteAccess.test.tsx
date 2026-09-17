import { render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter } from 'react-router-dom'
import { RouteAccess } from '../RouteAccess'

let role: string | null = 'owner'
vi.mock('../../../auth/useAuth', () => ({ useAuth: () => ({ user: role ? { role } : null }) }))

function renderAt(path: string) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <RouteAccess>
        <p>Page content</p>
      </RouteAccess>
    </MemoryRouter>,
  )
}

describe('RouteAccess', () => {
  beforeEach(() => {
    role = 'owner'
  })

  it('shows the page without a note when the role can make changes', () => {
    role = 'brewer'
    renderAt('/recipes')
    expect(screen.getByText('Page content')).toBeInTheDocument()
    expect(screen.queryByRole('note')).toBeNull()
  })

  it('notes a page the role can only read', () => {
    role = 'viewer'
    renderAt('/batches/b1')
    expect(screen.getByText('Page content')).toBeInTheDocument()
    expect(screen.getByRole('note')).toHaveTextContent('your role (Viewer) cannot make changes here')
  })

  it('skips the note on pages that only display data', () => {
    role = 'viewer'
    renderAt('/traceability')
    expect(screen.queryByRole('note')).toBeNull()
  })

  it.each([
    ['sales', '/cost-rates'],
    ['brewer', '/duty'],
    ['brewer', '/members'],
    ['viewer', '/recipes/new'],
    ['sales', '/inventory/import'],
  ])('refuses %s on %s', (r, path) => {
    role = r
    renderAt(path)
    expect(screen.queryByText('Page content')).toBeNull()
    expect(screen.getByRole('heading', { name: 'No access' })).toBeInTheDocument()
  })
})
