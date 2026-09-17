import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ConfirmProvider } from '../../../components/feedback/ConfirmDialog'
import { ToastProvider } from '../../../components/feedback/Toast'
import { MembersPage } from '../MembersPage'

type PatchArgs = { id: string; body: Record<string, unknown> }
const patchCalls: PatchArgs[] = []
const inviteCalls: Record<string, unknown>[] = []
const revokeCalls: string[] = []
let invitations: Record<string, unknown>[] = []
let currentUser = { user_id: 'u-owner', role: 'owner' }

const members = [
  { id: 'u-owner', email: 'owner@example.com', display_name: 'Olive Owner', role: 'owner', is_active: true, created_at: '2026-01-01T00:00:00Z' },
  { id: 'u-manager', email: 'manager@example.com', display_name: 'Max Manager', role: 'manager', is_active: true, created_at: '2026-01-02T00:00:00Z' },
  { id: 'u-brewer', email: 'brewer@example.com', display_name: 'Bea Brewer', role: 'brewer', is_active: true, created_at: '2026-01-03T00:00:00Z' },
]

vi.mock('../../../auth/useAuth', () => ({ useAuth: () => ({ user: currentUser }) }))
vi.mock('../hooks/useMembers', () => ({
  useMembers: () => ({ data: { items: members }, isLoading: false, isError: false }),
  usePatchMember: () => ({
    // A plain function rather than vi.fn (see MovementVoidCell.test.tsx).
    mutateAsync: async (args: PatchArgs) => {
      patchCalls.push(args)
    },
    isPending: false,
  }),
  useInvitations: () => ({ data: { items: invitations } }),
  useCreateInvitation: () => ({
    mutateAsync: async (body: Record<string, unknown>) => {
      inviteCalls.push(body)
      return { id: 'inv-1', email: body.email, role: body.role, expires_at: '2026-09-24T10:00:00Z', created_at: '2026-09-17T10:00:00Z', token: 'tok-1' }
    },
    isPending: false,
  }),
  useRevokeInvitation: () => ({
    mutateAsync: async (id: string) => {
      revokeCalls.push(id)
    },
    isPending: false,
  }),
}))

function renderPage() {
  return render(
    <ToastProvider>
      <ConfirmProvider>
        <MembersPage />
      </ConfirmProvider>
    </ToastProvider>,
  )
}

const row = (name: string) => screen.getByText(name).closest('tr') as HTMLElement

describe('MembersPage', () => {
  beforeEach(() => {
    patchCalls.length = 0
    inviteCalls.length = 0
    revokeCalls.length = 0
    invitations = []
    currentUser = { user_id: 'u-owner', role: 'owner' }
  })

  it('lets an Owner manage every member with every role', () => {
    renderPage()
    expect(within(row('Olive Owner')).getByText('(you)')).toBeInTheDocument()
    const managerRole = within(row('Max Manager')).getByRole('combobox', { name: 'Role for Max Manager' })
    expect(within(managerRole).getAllByRole('option').map((o) => o.textContent)).toEqual([
      'Owner', 'Manager', 'Brewer', 'Sales', 'Viewer',
    ])
  })

  it('limits a Manager to Brewer, Sales and Viewer members', () => {
    currentUser = { user_id: 'u-manager', role: 'manager' }
    renderPage()
    expect(within(row('Olive Owner')).queryByRole('combobox')).toBeNull()
    expect(within(row('Olive Owner')).queryByRole('button', { name: 'Deactivate' })).toBeNull()
    expect(within(row('Max Manager')).queryByRole('combobox')).toBeNull()
    const brewerRole = within(row('Bea Brewer')).getByRole('combobox', { name: 'Role for Bea Brewer' })
    expect(within(brewerRole).getAllByRole('option').map((o) => o.textContent)).toEqual(['Brewer', 'Sales', 'Viewer'])
  })

  it('changes a role', async () => {
    const user = userEvent.setup()
    renderPage()
    await user.selectOptions(within(row('Bea Brewer')).getByRole('combobox'), 'viewer')
    expect(patchCalls).toEqual([{ id: 'u-brewer', body: { role: 'viewer' } }])
  })

  it('confirms before deactivating a member', async () => {
    const user = userEvent.setup()
    renderPage()
    await user.click(within(row('Bea Brewer')).getByRole('button', { name: 'Deactivate' }))
    const dialog = await screen.findByRole('alertdialog', { name: 'Deactivate Bea Brewer?' })
    expect(patchCalls).toHaveLength(0)
    await user.click(within(dialog).getByRole('button', { name: 'Deactivate' }))
    await waitFor(() => expect(patchCalls).toEqual([{ id: 'u-brewer', body: { is_active: false } }]))
  })

  it('creates an invite link to share', async () => {
    const user = userEvent.setup()
    renderPage()
    await user.type(screen.getByLabelText('Email'), 'new@example.com')
    await user.selectOptions(screen.getByLabelText('Role'), 'sales')
    await user.click(screen.getByRole('button', { name: 'Create invite link' }))

    expect(inviteCalls).toEqual([{ email: 'new@example.com', role: 'sales' }])
    const link = await screen.findByLabelText('Invite link')
    expect((link as HTMLInputElement).value).toBe(`${window.location.origin}/invite#tok-1`)
  })

  it('lets a Manager invite only Brewer, Sales and Viewer members', () => {
    currentUser = { user_id: 'u-manager', role: 'manager' }
    renderPage()
    const role = screen.getByLabelText('Role')
    expect(within(role).getAllByRole('option').map((o) => o.textContent)).toEqual(['Brewer', 'Sales', 'Viewer'])
  })

  it('revokes an open invitation after confirming', async () => {
    const user = userEvent.setup()
    invitations = [{ id: 'inv-9', email: 'pending@example.com', role: 'viewer', expires_at: '2999-01-01T00:00:00Z', created_at: '2026-09-17T10:00:00Z' }]
    renderPage()
    await user.click(within(row('pending@example.com')).getByRole('button', { name: 'Revoke' }))
    const dialog = await screen.findByRole('alertdialog', { name: 'Revoke the invitation for pending@example.com?' })
    await user.click(within(dialog).getByRole('button', { name: 'Revoke' }))
    await waitFor(() => expect(revokeCalls).toEqual(['inv-9']))
  })
})
