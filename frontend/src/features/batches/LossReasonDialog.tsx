import { ReasonDialog } from '../../components/feedback/ReasonDialog'

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
  const spoiling = toStatus === 'spoiled'
  return (
    <ReasonDialog
      open={toStatus !== null}
      title={spoiling ? 'Mark this batch as spoiled?' : 'Cancel this batch?'}
      description="This is recorded in the compliance audit log and cannot be undone."
      confirmLabel={spoiling ? 'Mark spoiled' : 'Cancel batch'}
      cancelLabel="Keep batch"
      required={fromStatus === 'completed' && spoiling}
      busy={busy}
      onConfirm={onConfirm}
      onCancel={onCancel}
    />
  )
}
