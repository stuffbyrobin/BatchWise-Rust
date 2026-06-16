import React from 'react'
import { APIError } from '../../api/error'
import { useLabelRecords, useCreateLabelRecord, usePatchLabelRecord, useDeleteLabelRecord } from './hooks/useLabels'
import { AllergenBadges } from '../../components/AllergenBadges'
import type { components } from '../../api/generated'

type LabelRecord = components['schemas']['LabelRecord']

function fmtDate(s: string | null | undefined): string {
  if (!s) return '—'
  return String(s).slice(0, 10)
}

function StatusBadge({ status }: { status: string }) {
  const cls =
    status === 'approved'
      ? 'bg-green-100 text-green-700'
      : 'bg-yellow-100 text-yellow-700'
  return (
    <span className={`px-2 py-0.5 rounded text-xs font-medium ${cls}`}>{status}</span>
  )
}

function ApproveButton({ rec }: { rec: LabelRecord }) {
  const patch = usePatchLabelRecord(rec.id ?? '')
  const [err, setErr] = React.useState<string | null>(null)

  if (rec.status === 'approved') {
    return <span className="text-xs text-[var(--color-muted)]">Approved</span>
  }

  return (
    <div className="flex items-center gap-2">
      <button
        className="px-2 py-1 text-xs rounded bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
        disabled={patch.isPending}
        onClick={async () => {
          setErr(null)
          try {
            await patch.mutateAsync({ status: 'approved' })
          } catch (e) {
            setErr(e instanceof APIError ? e.message : 'Approve failed.')
          }
        }}
      >
        {patch.isPending ? 'Approving…' : 'Approve'}
      </button>
      {err && <span className="text-xs text-[var(--color-danger)]">{err}</span>}
    </div>
  )
}

function DeleteButton({ id, status }: { id: string; status: string }) {
  const del = useDeleteLabelRecord(id)
  const [err, setErr] = React.useState<string | null>(null)

  if (status === 'approved') return null

  return (
    <div className="flex items-center gap-2">
      <button
        className="px-2 py-1 text-xs rounded text-[var(--color-danger)] border border-[var(--color-danger)] hover:opacity-80 disabled:opacity-50"
        disabled={del.isPending}
        onClick={async () => {
          setErr(null)
          try {
            await del.mutateAsync()
          } catch (e) {
            setErr(e instanceof APIError ? e.message : 'Delete failed.')
          }
        }}
      >
        {del.isPending ? 'Deleting…' : 'Delete'}
      </button>
      {err && <span className="text-xs text-[var(--color-danger)]">{err}</span>}
    </div>
  )
}

function CreateForm({ onCreated }: { onCreated: () => void }) {
  const create = useCreateLabelRecord()
  const [batchId, setBatchId] = React.useState('')
  const [netVolume, setNetVolume] = React.useState('')
  const [servingVolume, setServingVolume] = React.useState('')
  const [err, setErr] = React.useState<string | null>(null)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setErr(null)
    const vol = parseInt(netVolume, 10)
    if (!batchId.trim()) { setErr('Batch ID is required.'); return }
    if (isNaN(vol) || vol <= 0) { setErr('Net volume must be a positive integer (mL).'); return }
    try {
      await create.mutateAsync({
        batch_id: batchId.trim(),
        net_volume_ml: vol,
        serving_volume_ml: servingVolume ? parseInt(servingVolume, 10) : undefined,
      })
      setBatchId('')
      setNetVolume('')
      setServingVolume('')
      onCreated()
    } catch (e) {
      setErr(e instanceof APIError ? e.message : 'Create failed.')
    }
  }

  const inputCls = 'p-2 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)] focus:outline-none w-full'

  return (
    <div
      className="rounded-xl border p-5 mb-6"
      style={{ borderColor: 'var(--color-border)', background: 'var(--color-surface)' }}
    >
      <h2 className="text-base font-semibold mb-1" style={{ color: 'var(--color-fg)' }}>
        New Label Record
      </h2>
      <p className="text-xs mb-4" style={{ color: 'var(--color-muted)' }}>
        Creates a draft label record from a batch. Product name, ABV, allergens, and responsible
        party are auto-populated from the batch and brewery settings.
      </p>
      <form onSubmit={handleSubmit} className="flex flex-wrap items-end gap-3">
        <div className="flex-1 min-w-[200px]">
          <label className="block text-xs text-[var(--color-muted)] mb-1">Batch ID (UUID)</label>
          <input
            type="text"
            required
            className={inputCls}
            value={batchId}
            onChange={(e) => setBatchId(e.target.value)}
            placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
          />
        </div>
        <div>
          <label className="block text-xs text-[var(--color-muted)] mb-1">Net Volume (mL)</label>
          <input
            type="number"
            required
            min={1}
            className={inputCls + ' w-32'}
            value={netVolume}
            onChange={(e) => setNetVolume(e.target.value)}
            placeholder="e.g. 330"
          />
        </div>
        <div>
          <label className="block text-xs text-[var(--color-muted)] mb-1">Serving Volume (mL, optional)</label>
          <input
            type="number"
            min={1}
            className={inputCls + ' w-40'}
            value={servingVolume}
            onChange={(e) => setServingVolume(e.target.value)}
            placeholder="e.g. 330"
          />
        </div>
        <button
          type="submit"
          disabled={create.isPending}
          className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
        >
          {create.isPending ? 'Creating…' : 'Create'}
        </button>
        {err && <p className="w-full text-xs text-[var(--color-danger)]">{err}</p>}
      </form>
    </div>
  )
}

export function LabelRecordsPage() {
  const [statusFilter, setStatusFilter] = React.useState<string>('')
  const [page, setPage] = React.useState(1)
  const [showCreate, setShowCreate] = React.useState(false)

  const { data, isLoading, isError, error } = useLabelRecords({
    status: statusFilter || undefined,
    page,
    page_size: 20,
  })

  return (
    <div className="p-6 max-w-5xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-bold" style={{ color: 'var(--color-fg)' }}>
          Label Records
        </h1>
        <button
          className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90"
          onClick={() => setShowCreate((v) => !v)}
        >
          {showCreate ? 'Cancel' : 'New Label Record'}
        </button>
      </div>

      {showCreate && (
        <CreateForm onCreated={() => setShowCreate(false)} />
      )}

      {/* Filters */}
      <div className="flex items-center gap-3 mb-4">
        <label className="text-xs text-[var(--color-muted)]">Status:</label>
        <select
          className="p-1.5 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)]"
          value={statusFilter}
          onChange={(e) => { setStatusFilter(e.target.value); setPage(1) }}
        >
          <option value="">All</option>
          <option value="draft">Draft</option>
          <option value="approved">Approved</option>
        </select>
      </div>

      {isLoading && <p className="text-sm text-[var(--color-muted)]">Loading…</p>}
      {isError && (
        <p className="text-sm text-[var(--color-danger)]">
          {error instanceof APIError ? error.message : 'Failed to load label records.'}
        </p>
      )}

      {!isLoading && !isError && (
        <>
          {(data?.items?.length ?? 0) === 0 ? (
            <p className="text-sm text-[var(--color-muted)]">
              No label records yet. Create one above.
            </p>
          ) : (
            <div className="space-y-4">
              {data?.items?.map((rec) => (
                <LabelRecordCard key={rec.id} rec={rec} />
              ))}
            </div>
          )}

          {/* Pagination */}
          {(data?.total ?? 0) > 20 && (
            <div className="flex items-center gap-3 mt-4">
              <button
                disabled={page <= 1}
                onClick={() => setPage((p) => p - 1)}
                className="px-3 py-1 rounded border text-sm disabled:opacity-50"
                style={{ borderColor: 'var(--color-border)' }}
              >
                Previous
              </button>
              <span className="text-xs text-[var(--color-muted)]">
                Page {page} of {Math.ceil((data?.total ?? 0) / 20)}
              </span>
              <button
                disabled={page >= Math.ceil((data?.total ?? 0) / 20)}
                onClick={() => setPage((p) => p + 1)}
                className="px-3 py-1 rounded border text-sm disabled:opacity-50"
                style={{ borderColor: 'var(--color-border)' }}
              >
                Next
              </button>
            </div>
          )}

          <p className="text-xs mt-3 text-[var(--color-muted)]">
            {data?.total ?? 0} record{(data?.total ?? 0) !== 1 ? 's' : ''} total
          </p>
        </>
      )}
    </div>
  )
}

function LabelRecordCard({ rec }: { rec: LabelRecord }) {
  const [expanded, setExpanded] = React.useState(false)

  return (
    <div
      className="rounded-xl border p-4"
      style={{ borderColor: 'var(--color-border)', background: 'var(--color-surface)' }}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <span className="font-semibold text-sm" style={{ color: 'var(--color-fg)' }}>
              {rec.product_name}
            </span>
            <StatusBadge status={rec.status ?? 'draft'} />
          </div>
          <div className="text-xs text-[var(--color-muted)] space-x-3">
            <span>ABV: {rec.abv_percent}%</span>
            <span>Vol: {rec.net_volume_ml} mL</span>
            <span>Lot: {rec.lot_identifier}</span>
            {rec.best_before_date && <span>BB: {fmtDate(rec.best_before_date)}</span>}
          </div>
          {(rec.allergens?.length ?? 0) > 0 && (
            <div className="mt-2">
              <AllergenBadges allergens={rec.allergens ?? []} />
            </div>
          )}
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <button
            onClick={() => setExpanded((v) => !v)}
            className="text-xs px-2 py-1 rounded border"
            style={{ borderColor: 'var(--color-border)', color: 'var(--color-muted)' }}
          >
            {expanded ? 'Less' : 'Details'}
          </button>
          <ApproveButton rec={rec} />
          <DeleteButton id={rec.id ?? ''} status={rec.status ?? 'draft'} />
        </div>
      </div>

      {expanded && (
        <div className="mt-3 pt-3 border-t text-xs space-y-1.5" style={{ borderColor: 'var(--color-border)', color: 'var(--color-fg)' }}>
          <div><span className="text-[var(--color-muted)]">Responsible Party:</span> {rec.responsible_party}</div>
          <div><span className="text-[var(--color-muted)]">Country of Origin:</span> {rec.country_of_origin}</div>
          {rec.ingredient_list && (
            <div><span className="text-[var(--color-muted)]">Ingredients:</span> {rec.ingredient_list}</div>
          )}
          {rec.energy_kj_per_100ml != null && (
            <div>
              <span className="text-[var(--color-muted)]">Energy:</span>{' '}
              {rec.energy_kj_per_100ml} kJ / {rec.energy_kcal_per_100ml} kcal per 100 mL
            </div>
          )}
          {rec.alcohol_units_per_serving != null && rec.serving_volume_ml != null && (
            <div>
              <span className="text-[var(--color-muted)]">Alcohol units per {rec.serving_volume_ml} mL serving:</span>{' '}
              {rec.alcohol_units_per_serving}
            </div>
          )}
          <div className="text-[var(--color-muted)]">
            Created: {fmtDate(rec.created_at)} · Updated: {fmtDate(rec.updated_at)}
          </div>
        </div>
      )}
    </div>
  )
}
