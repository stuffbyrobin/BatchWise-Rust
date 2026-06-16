import React from 'react'
import { useParams, Link } from 'react-router-dom'
import { useReadings, useCreateReading, useDeleteReading } from './hooks/useFermentation'
import type { components } from '../../api/generated'

type Reading = components['schemas']['FermentationReading']
type CreateRequest = components['schemas']['CreateFermentationReadingRequest']

const STAGES = ['primary', 'secondary', 'conditioning', 'lagering', 'other'] as const

function fmt(n: number | null | undefined): string {
  return n == null ? '—' : String(n)
}

function fmtDate(s: string | undefined): string {
  if (!s) return '—'
  return new Date(s).toLocaleString(undefined, { dateStyle: 'short', timeStyle: 'short' })
}

function LogForm({ batchId, onDone }: { batchId: string; onDone: () => void }) {
  const { mutate, isPending } = useCreateReading(batchId)
  const [form, setForm] = React.useState<{
    stage: string
    gravity: string
    temp_c: string
    ph: string
    notes: string
  }>({ stage: 'primary', gravity: '', temp_c: '', ph: '', notes: '' })
  const [err, setErr] = React.useState<string | null>(null)

  function set(field: string, val: string) {
    setForm((f) => ({ ...f, [field]: val }))
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    setErr(null)
    const body: CreateRequest = { stage: (form.stage || undefined) as CreateRequest['stage'] }
    if (form.gravity) body.gravity = parseFloat(form.gravity)
    if (form.temp_c) body.temp_c = parseFloat(form.temp_c)
    if (form.ph) body.ph = parseFloat(form.ph)
    if (form.notes) body.notes = form.notes
    mutate(body, {
      onSuccess: () => {
        setForm({ stage: 'primary', gravity: '', temp_c: '', ph: '', notes: '' })
        onDone()
      },
      onError: (e) => setErr(e.message),
    })
  }

  return (
    <form onSubmit={handleSubmit} className="bg-[var(--color-surface)] border border-[var(--color-border)] rounded-lg p-4 space-y-3">
      <h3 className="text-sm font-semibold text-[var(--color-text-secondary)]">Log Reading</h3>
      {err && <p className="text-xs text-[var(--color-danger)]">{err}</p>}
      <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
        <label className="flex flex-col gap-1">
          <span className="text-xs text-[var(--color-text-secondary)]">Stage</span>
          <select
            value={form.stage}
            onChange={(e) => set('stage', e.target.value)}
            className="input-field text-sm"
          >
            {STAGES.map((s) => (
              <option key={s} value={s}>{s}</option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs text-[var(--color-text-secondary)]">Gravity (SG)</span>
          <input
            type="number"
            step="0.001"
            min="0.9"
            max="2"
            placeholder="1.050"
            value={form.gravity}
            onChange={(e) => set('gravity', e.target.value)}
            className="input-field text-sm"
          />
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs text-[var(--color-text-secondary)]">Temp (°C)</span>
          <input
            type="number"
            step="0.1"
            placeholder="18.5"
            value={form.temp_c}
            onChange={(e) => set('temp_c', e.target.value)}
            className="input-field text-sm"
          />
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs text-[var(--color-text-secondary)]">pH</span>
          <input
            type="number"
            step="0.1"
            min="0"
            max="14"
            placeholder="4.2"
            value={form.ph}
            onChange={(e) => set('ph', e.target.value)}
            className="input-field text-sm"
          />
        </label>
      </div>
      <label className="flex flex-col gap-1">
        <span className="text-xs text-[var(--color-text-secondary)]">Notes</span>
        <input
          type="text"
          placeholder="Observations…"
          value={form.notes}
          onChange={(e) => set('notes', e.target.value)}
          className="input-field text-sm"
        />
      </label>
      <button
        type="submit"
        disabled={isPending}
        className="btn-primary text-sm px-4 py-1.5"
      >
        {isPending ? 'Saving…' : 'Log Reading'}
      </button>
    </form>
  )
}

function ReadingRow({ reading, batchId }: { reading: Reading; batchId: string }) {
  const { mutate: del, isPending } = useDeleteReading(batchId)
  return (
    <tr className="border-b border-[var(--color-border)] text-sm hover:bg-[var(--color-surface)]">
      <td className="py-2 px-3 text-[var(--color-text-secondary)]">{fmtDate(reading.recorded_at)}</td>
      <td className="py-2 px-3 capitalize">{reading.stage}</td>
      <td className="py-2 px-3 font-mono">{fmt(reading.gravity)}</td>
      <td className="py-2 px-3 font-mono">{fmt(reading.temp_c)}</td>
      <td className="py-2 px-3 font-mono">{fmt(reading.ph)}</td>
      <td className="py-2 px-3 text-[var(--color-text-secondary)] max-w-xs truncate">{reading.notes ?? '—'}</td>
      <td className="py-2 px-3 text-right">
        <button
          onClick={() => del(reading.id ?? '')}
          disabled={isPending}
          className="text-xs text-[var(--color-danger)] hover:opacity-70"
        >
          Delete
        </button>
      </td>
    </tr>
  )
}

export function FermentationPage() {
  const { batchId } = useParams<{ batchId: string }>()
  const [stageFilter, setStageFilter] = React.useState('')
  const { data, isLoading, isError, refetch } = useReadings(batchId ?? '', {
    stage: stageFilter || undefined,
  })

  if (!batchId) return null

  return (
    <div className="page-container space-y-6">
      <div className="flex items-center gap-3">
        <Link
          to={`/batches/${batchId}`}
          className="text-sm text-[var(--color-text-secondary)] hover:text-[var(--color-text)] flex items-center gap-1"
        >
          ← Batch
        </Link>
        <h1 className="text-2xl font-bold text-[var(--color-text)]">Fermentation Log</h1>
      </div>

      <LogForm batchId={batchId} onDone={() => refetch()} />

      <div>
        <div className="flex items-center gap-3 mb-3">
          <h2 className="text-sm font-semibold text-[var(--color-text-secondary)]">Readings</h2>
          <select
            value={stageFilter}
            onChange={(e) => setStageFilter(e.target.value)}
            className="input-field text-xs py-1 px-2"
          >
            <option value="">All stages</option>
            {STAGES.map((s) => (
              <option key={s} value={s}>{s}</option>
            ))}
          </select>
        </div>

        {isLoading && <p className="text-sm text-[var(--color-text-secondary)]">Loading…</p>}
        {isError && <p className="text-sm text-[var(--color-danger)]">Failed to load readings.</p>}

        {data && (
          data.items && data.items.length > 0 ? (
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="text-xs text-[var(--color-text-secondary)] border-b border-[var(--color-border)]">
                    <th className="py-2 px-3 text-left font-medium">Recorded</th>
                    <th className="py-2 px-3 text-left font-medium">Stage</th>
                    <th className="py-2 px-3 text-left font-medium">Gravity</th>
                    <th className="py-2 px-3 text-left font-medium">Temp °C</th>
                    <th className="py-2 px-3 text-left font-medium">pH</th>
                    <th className="py-2 px-3 text-left font-medium">Notes</th>
                    <th className="py-2 px-3" />
                  </tr>
                </thead>
                <tbody>
                  {data.items.map((rd) => (
                    <ReadingRow key={rd.id} reading={rd} batchId={batchId} />
                  ))}
                </tbody>
              </table>
              <p className="text-xs text-[var(--color-text-secondary)] mt-2">
                {data.total} reading{data.total !== 1 ? 's' : ''}
              </p>
            </div>
          ) : (
            <p className="text-sm text-[var(--color-text-secondary)]">No readings yet. Log one above.</p>
          )
        )}
      </div>
    </div>
  )
}
