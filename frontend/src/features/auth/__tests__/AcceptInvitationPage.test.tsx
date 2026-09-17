import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { APIError } from '../../../api/error'
import { AcceptInvitationPage } from '../AcceptInvitationPage'

const preview = { tenant_name: 'Hop Hill Brewery', email: 'bea@example.com', role: 'brewer', expires_at: '2026-09-24T10:00:00Z' }
let previewImpl: () => Promise<unknown> = async () => preview
const acceptCalls: unknown[][] = []

// Plain functions rather than vi.fn (see MovementVoidCell.test.tsx).
vi.mock('../../../api/client', () => ({
  apiClient: { post: () => previewImpl() },
}))
vi.mock('../../../auth/useAuth', () => ({
  useAuth: () => ({
    acceptInvitation: async (...args: unknown[]) => {
      acceptCalls.push(args)
    },
  }),
}))

function renderAt(url: string) {
  return render(
    <MemoryRouter initialEntries={[url]}>
      <Routes>
        <Route path="/invite" element={<AcceptInvitationPage />} />
        <Route path="/app" element={<p>Dashboard</p>} />
      </Routes>
    </MemoryRouter>,
  )
}

describe('AcceptInvitationPage', () => {
  beforeEach(() => {
    previewImpl = async () => preview
    acceptCalls.length = 0
  })

  it('shows the invitation and joins the brewery', async () => {
    const user = userEvent.setup()
    renderAt('/invite#tok-123')
    expect(await screen.findByText('Hop Hill Brewery')).toBeInTheDocument()
    expect(screen.getByText('Brewer')).toBeInTheDocument()

    await user.type(screen.getByLabelText('Your Name'), 'Bea')
    await user.type(screen.getByLabelText('Password'), 'Sup3rSecret!pw')
    await user.type(screen.getByLabelText('Confirm password'), 'Sup3rSecret!pw')
    await user.click(screen.getByRole('button', { name: 'Create account and join' }))

    await waitFor(() => expect(screen.getByText('Dashboard')).toBeInTheDocument())
    expect(acceptCalls).toEqual([['tok-123', 'Bea', 'Sup3rSecret!pw']])
  })

  it('checks that the passwords match', async () => {
    const user = userEvent.setup()
    renderAt('/invite#tok-123')
    await screen.findByText('Hop Hill Brewery')
    await user.type(screen.getByLabelText('Your Name'), 'Bea')
    await user.type(screen.getByLabelText('Password'), 'Sup3rSecret!pw')
    await user.type(screen.getByLabelText('Confirm password'), 'Different!pw12')
    await user.click(screen.getByRole('button', { name: 'Create account and join' }))

    expect(screen.getByRole('alert')).toHaveTextContent('The passwords do not match.')
    expect(acceptCalls).toHaveLength(0)
  })

  it('explains an expired invitation', async () => {
    previewImpl = async () => {
      throw new APIError(422, 'business_rule_violation', 'This invitation has expired. Ask for a new one.', 'r1')
    }
    renderAt('/invite#tok-123')
    expect(await screen.findByRole('alert')).toHaveTextContent('This invitation has expired. Ask for a new one.')
    expect(screen.queryByRole('button', { name: 'Create account and join' })).toBeNull()
  })

  it('stays vague about unknown or used invitations', async () => {
    previewImpl = async () => {
      throw new APIError(404, 'not_found', 'invitation not found', 'r1')
    }
    renderAt('/invite#tok-123')
    expect(await screen.findByRole('alert')).toHaveTextContent('This invitation is not valid.')
  })

  it('reports a link without a token', () => {
    renderAt('/invite')
    expect(screen.getByRole('alert')).toHaveTextContent('This invite link is incomplete.')
  })
})
