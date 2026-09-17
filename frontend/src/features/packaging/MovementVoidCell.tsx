import { useState } from 'react'
import { APIError } from '../../api/error'
import type { components } from '../../api/generated'
import { ReasonDialog } from '../../components/feedback/ReasonDialog'
import { useToast } from '../../components/feedback/Toast'
import { useVoidDistributionMovement } from './hooks/usePackaging'
import { useCanWrite } from '../../auth/useCan'

type DistributionMovement = components['schemas']['DistributionMovement']

/**
 * The Void action for a distribution movement, or the void reason once it has
 * been voided. Movements are traceability records, so they are voided with a
 * reason rather than deleted.
 */
export function MovementVoidCell({ movement }: { movement: DistributionMovement }) {
  const canWrite = useCanWrite('distribution')
  const [open, setOpen] = useState(false)
  const voidMovement = useVoidDistributionMovement()
  const { toast } = useToast()

  if (movement.voided_at) {
    return <span className="text-xs text-[var(--color-muted)]">Voided: {movement.void_reason}</span>
  }

  if (!canWrite) {
    return null
  }

  return (
    <>
      <button
        type="button"
        className="text-xs text-[var(--color-danger)] hover:underline"
        onClick={() => setOpen(true)}
      >
        Void
      </button>
      <ReasonDialog
        open={open}
        title="Void this movement?"
        description="It stays on record, marked void, and no longer counts towards stock or recalls. This is recorded in the compliance audit log."
        confirmLabel="Void movement"
        cancelLabel="Keep movement"
        required
        busy={voidMovement.isPending}
        onConfirm={async (reason) => {
          try {
            await voidMovement.mutateAsync({ id: movement.id ?? '', reason })
          } catch (e) {
            toast({
              title: 'Void failed',
              description: e instanceof APIError ? e.message : undefined,
              variant: 'destructive',
            })
          } finally {
            setOpen(false)
          }
        }}
        onCancel={() => setOpen(false)}
      />
    </>
  )
}
