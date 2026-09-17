import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { LossReasonDialog } from '../LossReasonDialog'

describe('LossReasonDialog', () => {
  it('requires a reason to spoil a completed batch', async () => {
    const user = userEvent.setup()
    const onConfirm = vi.fn()
    render(<LossReasonDialog toStatus="spoiled" fromStatus="completed" onConfirm={onConfirm} onCancel={vi.fn()} />)

    const confirm = screen.getByRole('button', { name: 'Mark spoiled' })
    expect(confirm).toBeDisabled()
    await user.type(screen.getByRole('textbox', { name: /reason \(required\)/i }), '   ')
    expect(confirm).toBeDisabled()
    await user.type(screen.getByRole('textbox', { name: /reason/i }), 'Infected kegs  ')
    expect(confirm).toBeEnabled()
    await user.click(confirm)
    expect(onConfirm).toHaveBeenCalledWith('Infected kegs')
  })

  it('lets a planned batch be cancelled without a reason', async () => {
    const user = userEvent.setup()
    const onConfirm = vi.fn()
    render(<LossReasonDialog toStatus="cancelled" fromStatus="planned" onConfirm={onConfirm} onCancel={vi.fn()} />)

    expect(screen.getByRole('alertdialog', { name: 'Cancel this batch?' })).toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: /reason \(optional\)/i })).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Cancel batch' }))
    expect(onConfirm).toHaveBeenCalledWith('')
  })

  it('keeps the batch when dismissed', async () => {
    const user = userEvent.setup()
    const onCancel = vi.fn()
    render(<LossReasonDialog toStatus="spoiled" fromStatus="fermenting" onConfirm={vi.fn()} onCancel={onCancel} />)

    await user.click(screen.getByRole('button', { name: 'Keep batch' }))
    expect(onCancel).toHaveBeenCalledTimes(1)
  })

  it('renders nothing while closed', () => {
    render(<LossReasonDialog toStatus={null} fromStatus="completed" onConfirm={vi.fn()} onCancel={vi.fn()} />)
    expect(screen.queryByRole('alertdialog')).toBeNull()
  })
})
