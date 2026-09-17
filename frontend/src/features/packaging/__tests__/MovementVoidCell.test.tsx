import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ToastProvider } from '../../../components/feedback/Toast'
import { MovementVoidCell } from '../MovementVoidCell'

type VoidArgs = { id: string; reason: string }
let authRole = 'sales'

vi.mock('../../../auth/useAuth', () => ({ useAuth: () => ({ user: { role: authRole } }) }))
const voidCalls: VoidArgs[] = []
let voidImpl: (args: VoidArgs) => Promise<unknown> = async () => undefined

vi.mock('../hooks/usePackaging', () => ({
  useVoidDistributionMovement: () => ({
    // A plain function rather than vi.fn: with vi.fn, Vitest failed the test on
    // a rejection that the component catches.
    mutateAsync: (args: VoidArgs) => {
      voidCalls.push(args)
      return voidImpl(args)
    },
    isPending: false,
  }),
}))

const movement = { id: 'm1', movement_type: 'sample', quantity: 10, to_location: 'Taproom' }

function renderCell(m: Record<string, unknown>) {
  return render(
    <ToastProvider>
      <MovementVoidCell movement={m} />
    </ToastProvider>,
  )
}

describe('MovementVoidCell', () => {
  beforeEach(() => {
    voidCalls.length = 0
    voidImpl = async () => undefined
    authRole = 'sales'
  })

  it('shows why a voided movement was voided, with no Void action', () => {
    renderCell({ ...movement, voided_at: '2026-09-17T10:00:00Z', void_reason: 'Wrong run' })
    expect(screen.getByText('Voided: Wrong run')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Void' })).toBeNull()
  })

  it('voids a movement once a reason is given', async () => {
    const user = userEvent.setup()
    voidImpl = async () => ({ ...movement, voided_at: '2026-09-17T10:00:00Z' })
    renderCell(movement)

    await user.click(screen.getByRole('button', { name: 'Void' }))
    const confirm = await screen.findByRole('button', { name: 'Void movement' })
    expect(confirm).toBeDisabled()
    await user.type(screen.getByRole('textbox', { name: /reason \(required\)/i }), ' Wrong run ')
    await user.click(confirm)

    expect(voidCalls).toEqual([{ id: 'm1', reason: 'Wrong run' }])
    await waitFor(() => expect(screen.queryByRole('alertdialog')).toBeNull())
  })

  it('shows a toast when voiding is refused', async () => {
    const user = userEvent.setup()
    voidImpl = async () => {
      throw new Error('refused')
    }
    renderCell(movement)

    await user.click(screen.getByRole('button', { name: 'Void' }))
    await user.type(await screen.findByRole('textbox', { name: /reason/i }), 'Duplicate')
    await user.click(screen.getByRole('button', { name: 'Void movement' }))

    expect(await screen.findByText('Void failed')).toBeInTheDocument()
    expect(voidCalls).toHaveLength(1)
  })

  it('hides the Void action from a role that can only read', () => {
    authRole = 'viewer'
    const { unmount } = renderCell(movement)
    expect(screen.queryByRole('button', { name: 'Void' })).toBeNull()
    unmount()
    renderCell({ ...movement, voided_at: '2026-09-17T10:00:00Z', void_reason: 'Wrong run' })
    expect(screen.getByText('Voided: Wrong run')).toBeInTheDocument()
  })
})
