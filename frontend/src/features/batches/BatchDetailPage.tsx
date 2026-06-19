import React from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import {
  useBatch,
  useUpdateBatch,
  useDeleteBatch,
  useTransitionBatch,
  usePatchBatchIngredients,
  ALLOWED_NEXT,
  STATUS_LABELS,
  STATUS_COLORS,
  type BatchStatus,
  type Batch,
} from './hooks/useBatches'
import { useCalendarEvents } from '../calendar/hooks/useCalendar'
import { useBatchCost, useComputeBatchCost } from '../reporting/hooks/useBatchCosts'
import { useInventoryList } from '../inventory/hooks/useInventory'
import { useTenant } from '../account/hooks/useTenant'
import type { components } from '../../api/generated'
import { calcHopIBU, type IBUMethod } from '../../utils/ibu'
import { formatEbc } from '../../utils/ebc'
import { APIError } from '../../api/error'

type InventoryLot = components['schemas']['Ingredient']

export function BatchDetailPage() {
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

  const handleDelete = () => {
    if (!id) return
    if (window.confirm('Delete this batch? This cannot be undone.')) {
      deleteBatch(undefined, { onSuccess: () => navigate('/batches') })
    }
  }

  if (isError) {
    return (
      <div className="p-4 rounded border border-[var(--color-danger)] bg-red-50 text-[var(--color-danger)]">
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
  const canEdit = !['completed', 'cancelled', 'spoiled'].includes(batch.status ?? '')
  const canDelete = ['planned', 'cancelled'].includes(batch.status ?? '')
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
      {/* Confirmation dialog for brewing transition */}
      {confirmBrewing && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
          <div className="bg-[var(--color-surface)] rounded-lg border border-[var(--color-border)] p-6 max-w-sm w-full mx-4 shadow-lg">
            <h2 className="text-lg font-semibold text-[var(--color-fg)] mb-2">Start brewing?</h2>
            <p className="text-sm text-[var(--color-muted)] mb-4">
              This will deduct ingredients from inventory based on the recipe snapshot. Make sure all required lots are available.
            </p>
            <div className="flex gap-3 justify-end">
              <button
                onClick={() => { setConfirmBrewing(false); setPendingTransition(null) }}
                className="px-4 py-2 rounded text-sm border border-[var(--color-border)] text-[var(--color-fg)]"
              >
                Cancel
              </button>
              <button
                onClick={handleConfirmBrewing}
                disabled={isTransitioning}
                className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white disabled:opacity-50"
              >
                {isTransitioning ? 'Starting…' : 'Start brewing'}
              </button>
            </div>
          </div>
        </div>
      )}

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
      {!isTerminal && (
        <div className="mb-6 p-4 rounded border border-[var(--color-border)] bg-[var(--color-surface)]">
          <p className="text-sm font-medium text-[var(--color-fg)] mb-3">Transition to:</p>
          <div className="flex flex-wrap gap-2">
            {allowedNext.map((toStatus) => (
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
            <div className="mt-3 p-3 rounded border border-[var(--color-danger)] bg-red-50 text-[var(--color-danger)]">
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
        <IngredientsEditorWrapper batch={batch} canEdit={canEdit} />
      </div>
    </div>
  )
}

type AnyRow = Record<string, unknown>

function IngredientsEditorWrapper({ batch, canEdit }: { batch: Batch; canEdit: boolean }) {
  const { data: tenant } = useTenant()
  const ibuMethod = (tenant?.ibu_method ?? 'tinseth') as IBUMethod
  const batchOg = typeof batch.target_og === 'number' ? batch.target_og : 1.050
  const snap = batch.batch_recipe_snapshot as Record<string, unknown> | null
  const batchVolL = (typeof snap?.batch_size_liters === 'number' && snap.batch_size_liters > 0)
    ? snap.batch_size_liters
    : (typeof batch.actual_volume_liters === 'number' && batch.actual_volume_liters > 0)
      ? batch.actual_volume_liters
      : 20
  return <IngredientsEditor batch={batch} canEdit={canEdit} ibuMethod={ibuMethod} batchOg={batchOg} batchVolL={batchVolL} />
}

function LotPicker({
  name,
  invType,
  value,
  onChange,
  disabled,
}: {
  name: string
  invType: string
  value: string
  onChange: (lotId: string, lot: InventoryLot | null) => void
  disabled: boolean
}) {
  const { data } = useInventoryList({ name, type: invType, page_size: 50 })
  const lots = data?.items ?? []

  // When lots data loads, sync analytical fields for the already-selected lot.
  // Uses a ref so it only fires once per selected lot, not on every re-render.
  const syncedLotRef = React.useRef<string | null>(null)
  React.useEffect(() => {
    if (!value || lots.length === 0) return
    if (syncedLotRef.current === value) return
    const lot = lots.find((l) => l.id === value) ?? null
    if (lot) {
      syncedLotRef.current = value
      onChange(value, lot)
    }
  // onChange is a new closure each render — intentionally excluded to avoid loops
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [value, lots])

  if (lots.length === 0) {
    return <span className="text-xs text-[var(--color-muted)] px-1">No stock</span>
  }

  return (
    <select
      value={value}
      disabled={disabled}
      onChange={(e) => {
        syncedLotRef.current = e.target.value  // mark as synced so useEffect won't re-fire
        const lot = lots.find((l) => l.id === e.target.value) ?? null
        onChange(e.target.value, lot)
      }}
      className="w-full bg-transparent rounded px-1 py-0.5 text-xs text-[var(--color-fg)] border border-transparent focus:outline-none focus:border-[var(--color-accent)] hover:border-[var(--color-border)] disabled:opacity-50"
    >
      <option value="">— FIFO —</option>
      {lots.map((lot) => (
        <option key={lot.id} value={lot.id}>
          {lot.lot_number} ({lot.amount} {lot.unit})
          {invType === 'hop' && lot.alpha_acid_pct != null ? ` · AA: ${lot.alpha_acid_pct}%` : ''}
          {invType === 'fermentable' && lot.color_ebc != null ? ` · ${formatEbc(lot.color_ebc)} EBC` : ''}
          {invType === 'yeast' && lot.attenuation_pct != null ? ` · ${lot.attenuation_pct}% att` : ''}
        </option>
      ))}
    </select>
  )
}

function cellCls(disabled: boolean) {
  return `w-full bg-transparent rounded px-1 py-0.5 text-sm text-[var(--color-fg)] focus:outline-none border border-transparent focus:border-[var(--color-accent)] hover:border-[var(--color-border)] ${disabled ? 'opacity-50 cursor-not-allowed' : ''}`
}

function IngredientsEditor({ batch, canEdit, ibuMethod, batchOg, batchVolL }: {
  batch: Batch; canEdit: boolean; ibuMethod: IBUMethod; batchOg: number; batchVolL: number
}) {
  const snap = batch.batch_recipe_snapshot
  const patchMut = usePatchBatchIngredients(batch.id ?? '')

  const [fermentables, setFermentables] = React.useState<AnyRow[]>((snap?.fermentables ?? []) as AnyRow[])
  const [hops, setHops]                 = React.useState<AnyRow[]>((snap?.hops ?? []) as AnyRow[])
  const [yeasts, setYeasts]             = React.useState<AnyRow[]>((snap?.yeasts ?? []) as AnyRow[])
  const [saved, setSaved]               = React.useState(false)
  const [saveError, setSaveError]       = React.useState<string | null>(null)

  const updateRow = (setter: React.Dispatch<React.SetStateAction<AnyRow[]>>, idx: number, field: string, value: unknown) =>
    setter((prev) => prev.map((r, i) => i === idx ? { ...r, [field]: value } : r))

  const addRow = (setter: React.Dispatch<React.SetStateAction<AnyRow[]>>, template: AnyRow) =>
    setter((prev) => [...prev, { ...template }])

  const removeRow = (setter: React.Dispatch<React.SetStateAction<AnyRow[]>>, idx: number) =>
    setter((prev) => prev.filter((_, i) => i !== idx))

  const handleSave = () => {
    setSaveError(null)
    patchMut.mutate(
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      { fermentables: fermentables as any, hops: hops as any, yeasts: yeasts as any },
      {
        onSuccess: () => { setSaved(true); setTimeout(() => setSaved(false), 2000) },
        onError: (err) => setSaveError(err instanceof Error ? err.message : 'Save failed'),
      },
    )
  }

  const inputCls = (disabled: boolean) => cellCls(disabled)

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-base font-semibold text-[var(--color-fg)]">Ingredients</h2>
        {canEdit && (
          <div className="flex items-center gap-3">
            {saveError && <span className="text-xs text-[var(--color-danger)]">{saveError}</span>}
            {saved && <span className="text-xs text-green-600">Saved</span>}
            <button
              onClick={handleSave}
              disabled={patchMut.isPending}
              className="px-4 py-1.5 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
            >
              {patchMut.isPending ? 'Saving…' : 'Save ingredients'}
            </button>
          </div>
        )}
        {!canEdit && (
          <span className="text-xs text-[var(--color-muted)]">Locked — terminal status</span>
        )}
      </div>

      {/* Fermentables */}
      <details open className="mb-4">
        <summary className="cursor-pointer text-sm font-medium text-[var(--color-muted)] hover:text-[var(--color-fg)] select-none mb-2">
          Fermentables ({fermentables.length})
        </summary>
        <div className="overflow-x-auto border rounded-lg mt-2" style={{ borderColor: 'var(--color-border)', background: 'var(--color-surface)' }}>
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-[var(--color-muted)] uppercase tracking-wide border-b" style={{ borderColor: 'var(--color-border)', background: 'var(--color-bg)' }}>
                <th className="px-2 py-2">Name</th>
                <th className="px-2 py-2 w-24">Amount</th>
                <th className="px-2 py-2 w-16">Unit</th>
                <th className="px-2 py-2 w-28">Type</th>
                <th className="px-2 py-2 w-24">Colour (EBC)</th>
                <th className="px-2 py-2 w-24">Potential PPG</th>
                <th className="px-2 py-2 w-28">Addition</th>
                <th className="px-2 py-2 w-48">Lot (stock)</th>
                {canEdit && <th className="px-2 py-2 w-8" />}
              </tr>
            </thead>
            <tbody>
              {fermentables.map((f, i) => (
                <tr key={i} className="border-t" style={{ borderColor: 'var(--color-border)' }}>
                  <td className="px-1 py-1"><input className={inputCls(!canEdit)} value={String(f.name ?? '')} disabled={!canEdit} onChange={(e) => updateRow(setFermentables, i, 'name', e.target.value)} /></td>
                  <td className="px-1 py-1"><input className={inputCls(!canEdit)} type="number" step="0.001" min="0" value={String(f.amount ?? '')} disabled={!canEdit} onChange={(e) => updateRow(setFermentables, i, 'amount', e.target.value === '' ? '' : Number(e.target.value))} /></td>
                  <td className="px-1 py-1">
                    {canEdit
                      ? <select className={inputCls(false)} value={String(f.unit ?? 'kg')} onChange={(e) => updateRow(setFermentables, i, 'unit', e.target.value)}>
                          <option value="kg">kg</option>
                          <option value="g">g</option>
                        </select>
                      : <span className="px-2 text-[var(--color-fg)] text-sm">{String(f.unit ?? 'kg')}</span>
                    }
                  </td>
                  <td className="px-1 py-1"><input className={inputCls(!canEdit)} value={String(f.type ?? '')} disabled={!canEdit} onChange={(e) => updateRow(setFermentables, i, 'type', e.target.value)} /></td>
                  <td className="px-1 py-1"><input className={inputCls(!canEdit)} type="number" step="0.1" min="0" value={String(f.color_ebc ?? '')} disabled={!canEdit} onChange={(e) => updateRow(setFermentables, i, 'color_ebc', e.target.value === '' ? null : Number(e.target.value))} /></td>
                  <td className="px-1 py-1"><input className={inputCls(!canEdit)} type="number" step="0.1" min="0" value={String(f.potential_ppg ?? '')} disabled={!canEdit} onChange={(e) => updateRow(setFermentables, i, 'potential_ppg', e.target.value === '' ? null : Number(e.target.value))} /></td>
                  <td className="px-1 py-1"><input className={inputCls(!canEdit)} value={String(f.addition ?? '')} disabled={!canEdit} onChange={(e) => updateRow(setFermentables, i, 'addition', e.target.value || null)} /></td>
                  <td className="px-1 py-1 min-w-[160px]">
                    <LotPicker
                      name={String(f.name ?? '')}
                      invType="fermentable"
                      value={String(f.inventory_lot_id ?? '')}
                      disabled={!canEdit}
                      onChange={(lotId, lot) => {
                        updateRow(setFermentables, i, 'inventory_lot_id', lotId || null)
                        if (lotId && lot?.color_ebc != null) updateRow(setFermentables, i, 'color_ebc', lot.color_ebc)
                      }}
                    />
                  </td>
                  {canEdit && <td className="px-1 py-1 text-center"><button onClick={() => removeRow(setFermentables, i)} className="text-[var(--color-danger)] hover:opacity-70 text-xs">✕</button></td>}
                </tr>
              ))}
              {fermentables.length === 0 && (
                <tr><td colSpan={canEdit ? 9 : 8} className="px-2 py-4 text-center text-xs text-[var(--color-muted)]">No fermentables</td></tr>
              )}
            </tbody>
          </table>
        </div>
        {canEdit && (
          <button onClick={() => addRow(setFermentables, { name: '', amount: 0, unit: 'kg', type: 'Grain', color_ebc: null, potential_ppg: null, addition: null })}
            className="mt-2 text-xs text-[var(--color-accent)] hover:opacity-70">+ Add fermentable</button>
        )}
      </details>

      {/* Hops */}
      <details open className="mb-4">
        <summary className="cursor-pointer text-sm font-medium text-[var(--color-muted)] hover:text-[var(--color-fg)] select-none mb-2">
          Hops ({hops.length})
        </summary>
        <div className="overflow-x-auto border rounded-lg mt-2" style={{ borderColor: 'var(--color-border)', background: 'var(--color-surface)' }}>
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-[var(--color-muted)] uppercase tracking-wide border-b" style={{ borderColor: 'var(--color-border)', background: 'var(--color-bg)' }}>
                <th className="px-2 py-2">Name</th>
                <th className="px-2 py-2 w-24">Amount (g)</th>
                <th className="px-2 py-2 w-20">Alpha %</th>
                <th className="px-2 py-2 w-24">Use</th>
                <th className="px-2 py-2 w-20">Time (min)</th>
                <th className="px-2 py-2 w-24">Form</th>
                <th className="px-2 py-2 w-20">IBU</th>
                <th className="px-2 py-2 w-48">Lot (stock)</th>
                {canEdit && <th className="px-2 py-2 w-8" />}
              </tr>
            </thead>
            <tbody>
              {hops.map((h, i) => (
                <tr key={i} className="border-t" style={{ borderColor: 'var(--color-border)' }}>
                  <td className="px-1 py-1"><input className={inputCls(!canEdit)} value={String(h.name ?? '')} disabled={!canEdit} onChange={(e) => updateRow(setHops, i, 'name', e.target.value)} /></td>
                  <td className="px-1 py-1"><input className={inputCls(!canEdit)} type="number" step="0.1" min="0" value={String(h.amount ?? '')} disabled={!canEdit} onChange={(e) => updateRow(setHops, i, 'amount', e.target.value === '' ? '' : Number(e.target.value))} /></td>
                  <td className="px-1 py-1"><input className={inputCls(!canEdit)} type="number" step="0.1" min="0" max="100" value={String(h.alpha_acid_pct ?? '')} disabled={!canEdit} onChange={(e) => updateRow(setHops, i, 'alpha_acid_pct', e.target.value === '' ? 0 : Number(e.target.value))} /></td>
                  <td className="px-1 py-1"><input className={inputCls(!canEdit)} value={String(h.use ?? '')} disabled={!canEdit} onChange={(e) => updateRow(setHops, i, 'use', e.target.value)} /></td>
                  <td className="px-1 py-1"><input className={inputCls(!canEdit)} type="number" step="1" min="0" value={String(h.boil_time_minutes ?? '')} disabled={!canEdit} onChange={(e) => updateRow(setHops, i, 'boil_time_minutes', e.target.value === '' ? 0 : Number(e.target.value))} /></td>
                  <td className="px-1 py-1">
                    {canEdit
                      ? <select className={inputCls(false)} value={String(h.form ?? '')} onChange={(e) => updateRow(setHops, i, 'form', e.target.value || null)}>
                          <option value="">—</option>
                          <option value="Pellet">Pellet</option>
                          <option value="Leaf">Leaf</option>
                          <option value="Plug">Plug</option>
                          <option value="Extract">Extract</option>
                        </select>
                      : <span className="px-2 text-[var(--color-fg)] text-sm">{String(h.form ?? '—')}</span>
                    }
                  </td>
                  <td className="px-2 py-1 text-xs text-right tabular-nums text-[var(--color-muted)]">
                    {(() => {
                      const ibu = calcHopIBU(
                        ibuMethod,
                        Number(h.amount) || 0,
                        Number(h.alpha_acid_pct) || 0,
                        Number(h.boil_time_minutes) || 0,
                        batchVolL,
                        batchOg,
                      )
                      return ibu > 0 ? ibu.toFixed(1) : '—'
                    })()}
                  </td>
                  <td className="px-1 py-1 min-w-[160px]">
                    <LotPicker
                      name={String(h.name ?? '')}
                      invType="hop"
                      value={String(h.inventory_lot_id ?? '')}
                      disabled={!canEdit}
                      onChange={(lotId, lot) => {
                        updateRow(setHops, i, 'inventory_lot_id', lotId || null)
                        if (lotId && lot?.alpha_acid_pct != null) updateRow(setHops, i, 'alpha_acid_pct', lot.alpha_acid_pct)
                      }}
                    />
                  </td>
                  {canEdit && <td className="px-1 py-1 text-center"><button onClick={() => removeRow(setHops, i)} className="text-[var(--color-danger)] hover:opacity-70 text-xs">✕</button></td>}
                </tr>
              ))}
              {hops.length === 0 && (
                <tr><td colSpan={canEdit ? 9 : 8} className="px-2 py-4 text-center text-xs text-[var(--color-muted)]">No hops</td></tr>
              )}
            </tbody>
          </table>
        </div>
        {canEdit && (
          <button onClick={() => addRow(setHops, { name: '', amount: 0, unit: 'g', alpha_acid_pct: 0, boil_time_minutes: 60, use: 'Boil', form: null })}
            className="mt-2 text-xs text-[var(--color-accent)] hover:opacity-70">+ Add hop</button>
        )}
      </details>

      {/* Yeasts */}
      <details open className="mb-4">
        <summary className="cursor-pointer text-sm font-medium text-[var(--color-muted)] hover:text-[var(--color-fg)] select-none mb-2">
          Yeasts ({yeasts.length})
        </summary>
        <div className="overflow-x-auto border rounded-lg mt-2" style={{ borderColor: 'var(--color-border)', background: 'var(--color-surface)' }}>
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-[var(--color-muted)] uppercase tracking-wide border-b" style={{ borderColor: 'var(--color-border)', background: 'var(--color-bg)' }}>
                <th className="px-2 py-2">Name</th>
                <th className="px-2 py-2 w-24">Amount</th>
                <th className="px-2 py-2 w-20">Unit</th>
                <th className="px-2 py-2 w-24">Attenuation %</th>
                <th className="px-2 py-2 w-48">Lot (stock)</th>
                {canEdit && <th className="px-2 py-2 w-8" />}
              </tr>
            </thead>
            <tbody>
              {yeasts.map((y, i) => (
                <tr key={i} className="border-t" style={{ borderColor: 'var(--color-border)' }}>
                  <td className="px-1 py-1"><input className={inputCls(!canEdit)} value={String(y.name ?? '')} disabled={!canEdit} onChange={(e) => updateRow(setYeasts, i, 'name', e.target.value)} /></td>
                  <td className="px-1 py-1"><input className={inputCls(!canEdit)} type="number" step="0.01" min="0" value={String(y.amount ?? '')} disabled={!canEdit} onChange={(e) => updateRow(setYeasts, i, 'amount', e.target.value === '' ? '' : Number(e.target.value))} /></td>
                  <td className="px-1 py-1">
                    {canEdit
                      ? <select className={inputCls(false)} value={String(y.unit ?? 'count')} onChange={(e) => updateRow(setYeasts, i, 'unit', e.target.value)}>
                          <option value="count">count</option>
                          <option value="g">g</option>
                          <option value="mL">mL</option>
                        </select>
                      : <span className="px-2 text-[var(--color-fg)] text-sm">{String(y.unit ?? '')}</span>
                    }
                  </td>
                  <td className="px-1 py-1"><input className={inputCls(!canEdit)} type="number" step="0.1" min="0" max="100" value={String(y.attenuation_pct ?? '')} disabled={!canEdit} onChange={(e) => updateRow(setYeasts, i, 'attenuation_pct', e.target.value === '' ? null : Number(e.target.value))} /></td>
                  <td className="px-1 py-1 min-w-[160px]">
                    <LotPicker
                      name={String(y.name ?? '')}
                      invType="yeast"
                      value={String(y.inventory_lot_id ?? '')}
                      disabled={!canEdit}
                      onChange={(lotId, lot) => {
                        updateRow(setYeasts, i, 'inventory_lot_id', lotId || null)
                        if (lotId && lot?.attenuation_pct != null) updateRow(setYeasts, i, 'attenuation_pct', lot.attenuation_pct)
                      }}
                    />
                  </td>
                  {canEdit && <td className="px-1 py-1 text-center"><button onClick={() => removeRow(setYeasts, i)} className="text-[var(--color-danger)] hover:opacity-70 text-xs">✕</button></td>}
                </tr>
              ))}
              {yeasts.length === 0 && (
                <tr><td colSpan={canEdit ? 6 : 5} className="px-2 py-4 text-center text-xs text-[var(--color-muted)]">No yeasts</td></tr>
              )}
            </tbody>
          </table>
        </div>
        {canEdit && (
          <button onClick={() => addRow(setYeasts, { name: '', amount: 1, unit: 'pkg', attenuation_pct: null, yeast_id: null })}
            className="mt-2 text-xs text-[var(--color-accent)] hover:opacity-70">+ Add yeast</button>
        )}
      </details>
    </div>
  )
}

function BatchCostSection({ batchId }: { batchId: string }) {
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
        <button onClick={() => setShowForm(!showForm)} className="px-3 py-1 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90">
          {cost ? 'Recompute' : 'Compute cost'}
        </button>
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
          <button onClick={handleCompute} disabled={computeMutation.isPending} className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50">
            {computeMutation.isPending ? 'Computing...' : 'Compute'}
          </button>
        </div>
      )}
    </div>
  )
}
