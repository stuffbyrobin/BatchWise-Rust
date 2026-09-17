import { useState, type ReactNode } from 'react'
import { ConfirmDialog } from './ConfirmDialog'

interface Props {
  open: boolean
  title: string
  description?: ReactNode
  confirmLabel: string
  cancelLabel?: string
  /** A non-blank reason is needed before confirming. */
  required?: boolean
  busy?: boolean
  /** Receives the trimmed reason, or '' when an optional reason is left blank. */
  onConfirm: (reason: string) => void
  onCancel: () => void
}

/** A destructive confirmation that asks why, for actions recorded in the audit log. */
export function ReasonDialog({
  open,
  title,
  description,
  confirmLabel,
  cancelLabel,
  required = false,
  busy = false,
  onConfirm,
  onCancel,
}: Props) {
  const [reason, setReason] = useState('')

  return (
    <ConfirmDialog
      open={open}
      title={title}
      description={description}
      confirmLabel={confirmLabel}
      cancelLabel={cancelLabel}
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
