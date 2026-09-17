import { render, screen, within, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { ConfirmDialog, ConfirmProvider, useConfirm } from '../ConfirmDialog'

describe('ConfirmDialog', () => {
  function Harness({ onResult }: { onResult: (ok: boolean) => void }) {
    const confirm = useConfirm()
    return (
      <button onClick={async () => onResult(await confirm({ title: 'Delete this batch?', description: 'This cannot be undone.', confirmLabel: 'Delete', destructive: true }))}>
        Delete batch
      </button>
    )
  }

  const renderHarness = (onResult: (ok: boolean) => void) =>
    render(
      <ConfirmProvider>
        <Harness onResult={onResult} />
      </ConfirmProvider>,
    )

  it('resolves true when confirmed', async () => {
    const onResult = vi.fn()
    renderHarness(onResult)

    await userEvent.click(screen.getByText('Delete batch'))
    const dialog = await screen.findByRole('alertdialog', { name: 'Delete this batch?' })
    expect(within(dialog).getByText('This cannot be undone.')).toBeInTheDocument()
    await userEvent.click(within(dialog).getByRole('button', { name: 'Delete' }))
    await waitFor(() => expect(onResult).toHaveBeenCalledWith(true))
    expect(screen.queryByRole('alertdialog')).toBeNull()
  })

  it('resolves false when cancelled', async () => {
    const onResult = vi.fn()
    renderHarness(onResult)

    await userEvent.click(screen.getByText('Delete batch'))
    const dialog = await screen.findByRole('alertdialog', { name: 'Delete this batch?' })
    await userEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }))
    await waitFor(() => expect(onResult).toHaveBeenCalledWith(false))
    expect(screen.queryByRole('alertdialog')).toBeNull()
  })

  it('Escape cancels', async () => {
    const onResult = vi.fn()
    renderHarness(onResult)

    await userEvent.click(screen.getByText('Delete batch'))
    await screen.findByRole('alertdialog', { name: 'Delete this batch?' })
    await userEvent.keyboard('{Escape}')
    await waitFor(() => expect(onResult).toHaveBeenCalledWith(false))
  })

  it('focuses Cancel first', async () => {
    renderHarness(vi.fn())

    await userEvent.click(screen.getByText('Delete batch'))
    await screen.findByRole('alertdialog', { name: 'Delete this batch?' })
    await waitFor(() => expect(screen.getByRole('button', { name: 'Cancel' })).toHaveFocus())
  })

  it('a busy dialog disables its buttons and ignores Escape', async () => {
    const onCancel = vi.fn()
    render(
      <ConfirmDialog
        open
        busy
        title="Start brewing?"
        confirmLabel="Starting…"
        onConfirm={vi.fn()}
        onCancel={onCancel}
      />,
    )

    expect(screen.getByRole('button', { name: 'Cancel' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Starting…' })).toBeDisabled()
    await userEvent.keyboard('{Escape}')
    expect(onCancel).not.toHaveBeenCalled()
  })

  it('renders extra content and can hold the confirm button disabled', () => {
    render(
      <ConfirmDialog open title="Mark spoiled?" confirmLabel="Mark spoiled" confirmDisabled onConfirm={vi.fn()} onCancel={vi.fn()}>
        <p>Reason field</p>
      </ConfirmDialog>,
    )
    expect(screen.getByText('Reason field')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Mark spoiled' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeEnabled()
  })

  it('useConfirm requires the provider', () => {
    function Bare() {
      useConfirm()
      return null
    }

    const origError = console.error
    vi.spyOn(console, 'error').mockImplementation(() => {})
    expect(() => render(<Bare />)).toThrow('useConfirm must be used within ConfirmProvider')
    console.error = origError
  })
})
