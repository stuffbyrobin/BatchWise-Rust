import { useState } from 'react'
import { ConfirmDialog } from '../../components/feedback/ConfirmDialog'

export type LossStatus = 'cancelled' | 'spoiled'

interface Props {
  /** The loss transition being confirmed, or null when the dialog is closed. */
  toStatus: LossStatus | null
  fromStatus: string
  busy?: boolean
  onConfirm: (reason: string) => void
  onCancel: () => void
}

/**
 * Confirms cancelling or spoiling a batch and collects the reason, which is
 * recorded in the compliance audit log. Spoiling a completed batch requires one.
 */
export function LossReasonDialog({ toStatus, fromStatus, busy = false, onConfirm, onCancel }: Props) {
  const [reason, setReason] = useState('')
  const required = fromStatus === 'completed' && toStatus === 'spoiled'
  const spoiling = toStatus === 'spoiled'

  return (
    <ConfirmDialog
      open={toStatus !== null}
      title={spoiling ? 'Mark this batch as spoiled?' : 'Cancel this batch?'}
      description="This is recorded in the compliance audit log and cannot be undone."
      confirmLabel={spoiling ? 'Mark spoiled' : 'Cancel batch'}
      cancelLabel="Keep batch"
      destructive
      busy={busy}
      confirmDisabled={required && reason.trim() === ''}
      onConfirm={() => {
        onConfirm(reason.trim())
        setReason('')
      }}
      onCancel={() => {
        setReason('')
        onCancel()
      }}
    >
      <label className="block mb-4">
        <span className="block text-sm font-medium text-[var(--color-fg)] mb-1">
          Reason{required ? ' (required)' : ' (optional)'}
        </span>
        <textarea
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          maxLength={1000}
          rows={3}
          className="w-full rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-2 py-1 text-sm text-[var(--color-fg)]"
        />
      </label>
    </ConfirmDialog>
  )
}
