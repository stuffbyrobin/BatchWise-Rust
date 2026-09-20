import React from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import {
  useBatch,
  useUpdateBatch,
  useDeleteBatch,
  useTransitionBatch,
  ALLOWED_NEXT,
  STATUS_LABELS,
  STATUS_COLORS,
  type BatchStatus,
} from './hooks/useBatches'
import { useCalendarEvents } from '../calendar/hooks/useCalendar'
import { useBatchCost, useComputeBatchCost } from '../reporting/hooks/useBatchCosts'
import { BatchIngredientsEditor } from './BatchIngredientsEditor'
import { APIError } from '../../api/error'
import { ConfirmDialog, useConfirm } from '../../components/feedback/ConfirmDialog'
import { useCanWrite, useIsManager } from '../../auth/useCan'
import { LossReasonDialog, type LossStatus } from './LossReasonDialog'

export function BatchDetailPage() {
  const confirm = useConfirm()
  const canWrite = useCanWrite('production')
  const isManager = useIsManager()
  const navigate = useNavigate()
  const { id } = useParams<{ id: string }>()

  const { data: batch, isLoading, isError, error, refetch } = useBatch(id ?? '')
  const { mutate: updateBatch, isPending: isUpdating } = useUpdateBatch(id ?? '')
  const { mutate: deleteBatch, isPending: isDeleting } = useDeleteBatch(id ?? '')
  const { mutate: transition, isPending: isTransitioning, error: transitionError, reset: resetTransition } = useTransitionBatch(id ?? '')
  const { data: eventsData } = useCalendarEvents({ batch_id: id, page_size: 50 })

  const [name, setName] = React.useState('')
  const [brewDate, setBrewDate] = React.useState('')
  const [packageDate, setPackageDate] = React.useState('')
  const [targetOg, setTargetOg] = React.useState<number | ''>('')
  const [actualOg, setActualOg] = React.useState<number | ''>('')
  const [targetFg, setTargetFg] = React.useState<number | ''>('')
  const [actualFg, setActualFg] = React.useState<number | ''>('')
  const [actualVolume, setActualVolume] = React.useState<number | ''>('')
  const [notes, setNotes] = React.useState('')

  const [pendingTransition, setPendingTransition] = React.useState<BatchStatus | null>(null)
  const [confirmBrewing, setConfirmBrewing] = React.useState(false)
  const [lossTransition, setLossTransition] = React.useState<LossStatus | null>(null)

  React.useEffect(() => {
    if (batch) {
      setName(batch.name ?? '')
      setBrewDate(batch.brew_date ?? '')
      setPackageDate(batch.package_date ?? '')
      setTargetOg(batch.target_og ?? '')
      setActualOg(batch.actual_og ?? '')
      setTargetFg(batch.target_fg ?? '')
      setActualFg(batch.actual_fg ?? '')
      setActualVolume(batch.actual_volume_liters ?? '')
      setNotes(batch.notes ?? '')
    }
  }, [batch])

  const handleSave = () => {
    if (!id || !name) return
    updateBatch({
      name,
      brew_date: brewDate || null,
      package_date: packageDate || null,
      target_og: targetOg === '' ? null : Number(targetOg),
      actual_og: actualOg === '' ? null : Number(actualOg),
      target_fg: targetFg === '' ? null : Number(targetFg),
      actual_fg: actualFg === '' ? null : Number(actualFg),
      actual_volume_liters: actualVolume === '' ? null : Number(actualVolume),
      notes: notes || null,
    }, { onSuccess: () => refetch() })
  }

  type TransitionStatus = 'brewing' | 'fermenting' | 'conditioning' | 'packaging' | 'completed' | 'cancelled' | 'spoiled'

  const handleTransitionClick = (toStatus: BatchStatus) => {
    if (toStatus === 'brewing') {
      setPendingTransition(toStatus)
      setConfirmBrewing(true)
    } else if (toStatus === 'cancelled' || toStatus === 'spoiled') {
      setLossTransition(toStatus)
    } else {
      resetTransition()
      transition({ to_status: toStatus as TransitionStatus }, { onSuccess: () => refetch() })
    }
  }

  const handleConfirmBrewing = () => {
    setConfirmBrewing(false)
    if (pendingTransition) {
      resetTransition()
      transition({ to_status: pendingTransition as TransitionStatus }, {
        onSuccess: () => { setPendingTransition(null); refetch() },
        onError: () => setPendingTransition(null),
      })
    }
  }

  const handleConfirmLoss = (reason: string) => {
    if (!lossTransition) return
    resetTransition()
    transition({ to_status: lossTransition, reason: reason || undefined }, {
      onSuccess: () => { setLossTransition(null); refetch() },
      onError: () => setLossTransition(null),
    })
  }

  const handleDelete = async () => {
    if (!id) return
    const ok = await confirm({ title: 'Delete this batch?', description: 'This cannot be undone.', confirmLabel: 'Delete', destructive: true })
    if (!ok) return
    deleteBatch(undefined, { onSuccess: () => navigate('/batches') })
  }

  if (isError) {
    return (
      <div className="p-4 rounded border border-[var(--color-danger)] bg-[var(--color-danger-bg)] text-[var(--color-danger)]">
        <p className="font-semibold">Failed to load batch.</p>
        <p className="text-sm mt-1">
          {error instanceof APIError ? error.message : error instanceof Error ? error.message : 'Unknown error'}
        </p>
        <button onClick={() => refetch()} className="mt-2 px-3 py-1 text-sm rounded bg-[var(--color-danger)] text-white">
          Retry
        </button>
      </div>
    )
  }

  if (isLoading || !batch) {
    return <div className="animate-pulse space-y-4 max-w-2xl"><div className="h-8 w-64 rounded" style={{ background: 'var(--color-border)' }} /></div>
  }

  const allowedNext = ALLOWED_NEXT[batch.status as BatchStatus] ?? []
  const isTerminal = allowedNext.length === 0
  // Only an Owner or Manager may spoil a completed batch.
  const transitions = allowedNext.filter((to) => isManager || !(batch.status === 'completed' && to === 'spoiled'))
  const canEdit = canWrite && !['completed', 'cancelled', 'spoiled'].includes(batch.status ?? '')
  // Once brewing starts the batch is a production record, even if cancelled.
  const canDelete = canWrite && batch.status === 'planned'
  const transitionErrorMessage = transitionError instanceof APIError
    ? transitionError.message
    : transitionError instanceof Error ? transitionError.message : null

  interface TransitionErrorDetails {
    rule?: string
    requested_amount?: number
    available_amount?: number
    shortage_amount?: number
    unit?: string
    allowed_next?: string[]
  }
  const transitionDetails = transitionError instanceof APIError
    ? (transitionError.details as TransitionErrorDetails | undefined)
    : undefined

  return (
    <div>
      <LossReasonDialog
        toStatus={lossTransition}
        fromStatus={batch.status ?? ''}
        busy={isTransitioning}
        onConfirm={handleConfirmLoss}
        onCancel={() => setLossTransition(null)}
      />
      <ConfirmDialog
        open={confirmBrewing}
        title="Start brewing?"
        description="This will deduct ingredients from inventory based on the recipe snapshot. Make sure all required lots are available."
        confirmLabel="Start brewing"
        onConfirm={handleConfirmBrewing}
        onCancel={() => {
          setConfirmBrewing(false)
          setPendingTransition(null)
        }}
      />

      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <button
            onClick={() => navigate('/batches')}
            className="text-sm text-[var(--color-muted)] hover:text-[var(--color-fg)]"
          >
            ← Batches
          </button>
          <h1 className="text-xl font-bold text-[var(--color-fg)]">{batch.name}</h1>
          <span
            className="px-2 py-0.5 rounded text-xs font-medium text-white"
            style={{ background: STATUS_COLORS[batch.status as BatchStatus] ?? 'var(--color-muted)' }}
          >
            {STATUS_LABELS[batch.status as BatchStatus] ?? batch.status}
          </span>
        </div>
        <span className="text-xs font-mono text-[var(--color-muted)]">{batch.batch_number}</span>
      </div>

      {/* Transition controls */}
      {canWrite && transitions.length > 0 && (
        <div className="mb-6 p-4 rounded border border-[var(--color-border)] bg-[var(--color-surface)]">
          <p className="text-sm font-medium text-[var(--color-fg)] mb-3">Transition to:</p>
          <div className="flex flex-wrap gap-2">
            {transitions.map((toStatus) => (
              <button
                key={toStatus}
                onClick={() => handleTransitionClick(toStatus)}
                disabled={isTransitioning}
                className="px-4 py-1.5 rounded text-sm font-medium text-white disabled:opacity-50"
                style={{ background: STATUS_COLORS[toStatus] ?? 'var(--color-muted)' }}
              >
                → {STATUS_LABELS[toStatus]}
              </button>
            ))}
          </div>

          {transitionErrorMessage && (
            <div className="mt-3 p-3 rounded border border-[var(--color-danger)] bg-[var(--color-danger-bg)] text-[var(--color-danger)]">
              <p className="text-sm font-semibold">{transitionErrorMessage}</p>
              {transitionDetails?.rule === 'insufficient_stock' && (
                <p className="text-xs mt-1">
                  Requested: {transitionDetails.requested_amount ?? ''} {transitionDetails.unit ?? ''} —
                  Available: {transitionDetails.available_amount ?? ''} {transitionDetails.unit ?? ''} —
                  Shortage: {transitionDetails.shortage_amount ?? ''} {transitionDetails.unit ?? ''}
                </p>
              )}
              {transitionDetails?.allowed_next && (
                <p className="text-xs mt-1">
                  Allowed transitions: {transitionDetails.allowed_next.join(', ') || 'none'}
                </p>
              )}
            </div>
          )}
        </div>
      )}

      {isTerminal && (
        <div className="mb-6 p-3 rounded border border-[var(--color-border)] text-sm text-[var(--color-muted)]">
          This batch is in a terminal state — no further transitions available.
        </div>
      )}

      {!canEdit && (
        <div className="mb-6 p-3 rounded border border-[var(--color-border)] text-sm text-[var(--color-muted)]">
          This batch is {batch.status} — fields are locked for duty reporting.
        </div>
      )}

      {/* Editable fields */}
      <div className="space-y-4 max-w-2xl mb-8">
        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Name</label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            readOnly={!canEdit}
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] read-only:opacity-60"
          />
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="flex flex-col gap-1">
            <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Brew Date</label>
            <input
              type="date"
              value={brewDate}
              onChange={(e) => setBrewDate(e.target.value)}
              readOnly={!canEdit}
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] read-only:opacity-60"
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Package Date</label>
            <input
              type="date"
              value={packageDate}
              onChange={(e) => setPackageDate(e.target.value)}
              readOnly={!canEdit}
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] read-only:opacity-60"
            />
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="flex flex-col gap-1">
            <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Target OG</label>
            <input
              type="number"
              value={targetOg}
              onChange={(e) => setTargetOg(e.target.value === '' ? '' : Number(e.target.value))}
              step="0.001" min="1"
              readOnly={!canEdit}
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] read-only:opacity-60"
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Actual OG</label>
            <input
              type="number"
              value={actualOg}
              onChange={(e) => setActualOg(e.target.value === '' ? '' : Number(e.target.value))}
              step="0.001" min="1"
              readOnly={!canEdit}
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] read-only:opacity-60"
            />
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="flex flex-col gap-1">
            <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Target FG</label>
            <input
              type="number"
              value={targetFg}
              onChange={(e) => setTargetFg(e.target.value === '' ? '' : Number(e.target.value))}
              step="0.001" min="1"
              readOnly={!canEdit}
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] read-only:opacity-60"
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Actual FG</label>
            <input
              type="number"
              value={actualFg}
              onChange={(e) => setActualFg(e.target.value === '' ? '' : Number(e.target.value))}
              step="0.001" min="1"
              readOnly={!canEdit}
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] read-only:opacity-60"
            />
          </div>
        </div>

        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Actual Volume (L)</label>
          <input
            type="number"
            value={actualVolume}
            onChange={(e) => setActualVolume(e.target.value === '' ? '' : Number(e.target.value))}
            step="0.1" min="0"
            readOnly={!canEdit}
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] read-only:opacity-60"
          />
        </div>

        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Notes</label>
          <textarea
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            rows={3}
            readOnly={!canEdit}
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] read-only:opacity-60"
          />
        </div>

        <div className="flex gap-2">
          {canEdit && (
            <button
              onClick={handleSave}
              disabled={isUpdating || !name}
              className="px-6 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
            >
              {isUpdating ? 'Saving…' : 'Save'}
            </button>
          )}
          {canDelete && (
            <button
              onClick={handleDelete}
              disabled={isDeleting}
              className="px-6 py-2 rounded text-sm bg-[var(--color-danger)] text-white hover:opacity-90 disabled:opacity-50"
            >
              {isDeleting ? 'Deleting…' : 'Delete batch'}
            </button>
          )}
        </div>
      </div>

      {/* Generated events */}
      {(eventsData?.items?.length ?? 0) > 0 && (
        <div className="border-t pt-6 mt-4" style={{ borderColor: 'var(--color-border)' }}>
          <h2 className="text-lg font-semibold text-[var(--color-fg)] mb-3">Calendar Events</h2>
          <div className="space-y-2">
            {eventsData?.items?.map((ev) => (
              <div
                key={ev.id}
                className="flex items-center justify-between p-3 rounded border border-[var(--color-border)] bg-[var(--color-surface)]"
              >
                <div>
                  <span className="text-sm font-medium text-[var(--color-fg)]">{ev.title}</span>
                  <span className="ml-2 text-xs text-[var(--color-muted)]">{ev.event_type}</span>
                </div>
                <div className="flex items-center gap-3 text-xs text-[var(--color-muted)]">
                  <span>{ev.start_time ? new Date(ev.start_time).toLocaleDateString() : '—'}</span>
                  <span
                    className="px-1.5 py-0.5 rounded"
                    style={{
                      background: ev.status === 'completed' ? 'var(--color-success)' : ev.status === 'skipped' ? 'var(--color-muted)' : 'var(--color-warning)',
                      color: 'white',
                    }}
                  >
                    {ev.status}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Fermentation log link */}
      <div className="border-t pt-6 mt-6" style={{ borderColor: 'var(--color-border)' }}>
        <a
          href={`/batches/${id}/fermentation`}
          className="inline-flex items-center gap-2 text-sm text-[var(--color-primary)] hover:underline"
        >
          Fermentation Log →
        </a>
      </div>

      {/* Batch cost section */}
      <div className="border-t pt-6 mt-6" style={{ borderColor: 'var(--color-border)' }}>
        <BatchCostSection batchId={id!} />
      </div>

      {/* Ingredients editor */}
      <div className="border-t pt-6 mt-6" style={{ borderColor: 'var(--color-border)' }}>
        <BatchIngredientsEditor batch={batch} canEdit={canEdit} />
      </div>
    </div>
  )
}

function BatchCostSection({ batchId }: { batchId: string }) {
  const canCompute = useCanWrite('costs')
  const { data: cost, isLoading } = useBatchCost(batchId)
  const computeMutation = useComputeBatchCost()
  const [showForm, setShowForm] = React.useState(false)
  const [energyKwh, setEnergyKwh] = React.useState('')
  const [laborHours, setLaborHours] = React.useState('')
  const [waterLiters, setWaterLiters] = React.useState('')
  const [overheadPence, setOverheadPence] = React.useState('')

  const fmt = (p: number | null | undefined) => p == null ? '-' : String.fromCharCode(163) + (p / 100).toFixed(2)

  const handleCompute = () => {
    computeMutation.mutate({
      batch_id: batchId,
      energy_kwh: energyKwh ? Number(energyKwh) : null,
      labor_hours: laborHours ? Number(laborHours) : null,
      water_liters: waterLiters ? Number(waterLiters) : null,
      overhead_pence: overheadPence ? Number(overheadPence) : null,
    }, { onSuccess: () => setShowForm(false) })
  }

  if (isLoading) return <div className="animate-pulse h-8 rounded" style={{ background: 'var(--color-border)' }} />

  return (
    <div>
      <div className="flex items-center justify-between mb-3">
        <h2 className="text-base font-semibold text-[var(--color-fg)]">Batch Cost</h2>
        {canCompute && (
          <button onClick={() => setShowForm(!showForm)} className="px-3 py-1 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90">
            {cost ? 'Recompute' : 'Compute cost'}
          </button>
        )}
      </div>
      {cost && (
        <div className="grid grid-cols-2 gap-2 text-sm mb-4">
          {[['Ingredients', cost.ingredient_cost_pence], ['Energy', cost.energy_cost_pence], ['Labor', cost.labor_cost_pence], ['Water', cost.water_cost_pence], ['Overhead', cost.overhead_cost_pence], ['Est. Duty', cost.estimated_duty_pence], ['Total', cost.total_cost_pence], ['Per Litre', cost.cost_per_liter_pence]].map(([label, val]) => (
            <div key={String(label)} className="flex justify-between border-b py-1" style={{ borderColor: 'var(--color-border)' }}>
              <span className="text-[var(--color-muted)]">{label}</span>
              <span className="font-medium">{fmt(val as number | null | undefined)}</span>
            </div>
          ))}
        </div>
      )}
      {!cost && !showForm && <p className="text-sm text-[var(--color-muted)]">No cost computed yet.</p>}
      {showForm && (
        <div className="flex flex-col gap-2 max-w-xs">
          {([['Energy (kWh)', energyKwh, setEnergyKwh], ['Labor (hours)', laborHours, setLaborHours], ['Water (L)', waterLiters, setWaterLiters], ['Overhead (pence)', overheadPence, setOverheadPence]] as [string, string, React.Dispatch<React.SetStateAction<string>>][]).map(([label, val, setter]) => (
            <div key={label} className="flex items-center gap-2">
              <label className="text-xs text-[var(--color-muted)] w-32 shrink-0">{label}</label>
              <input type="number" value={val} onChange={(e) => setter(e.target.value)} className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm w-full" />
            </div>
          ))}
          {canCompute && (
            <button onClick={handleCompute} disabled={computeMutation.isPending} className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50">
              {computeMutation.isPending ? 'Computing...' : 'Compute'}
            </button>
          )}
        </div>
      )}
    </div>
  )
}
