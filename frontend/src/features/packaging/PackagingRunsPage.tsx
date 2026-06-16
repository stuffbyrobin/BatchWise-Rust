import React from 'react'
import { APIError } from '../../api/error'
import {
  usePackagingRuns, useCreatePackagingRun, usePatchPackagingRun, useDeletePackagingRun,
  useDistributionMovements, useCreateDistributionMovement, useDeleteDistributionMovement,
} from './hooks/usePackaging'
import type { components } from '../../api/generated'

type PackagingRun = components['schemas']['PackagingRun']
type DistributionMovement = components['schemas']['DistributionMovement']

const FORMATS = ['can', 'bottle', 'keg', 'cask', 'polypin', 'bag_in_box', 'other']
const MOVEMENT_TYPES = ['sale', 'taproom_transfer', 'internal_transfer', 'sample', 'return', 'disposal']

function fmtDate(s: string | null | undefined): string {
  if (!s) return '—'
  return String(s).slice(0, 10)
}

function MovementsPanel({ run }: { run: PackagingRun }) {
  const { data, isLoading } = useDistributionMovements({ packaging_run_id: run.id })
  const createMov = useCreateDistributionMovement()
  const deleteMov = useDeleteDistributionMovement()
  const [showForm, setShowForm] = React.useState(false)
  const [form, setForm] = React.useState({ movement_type: 'sale', quantity: '', to_location: '', order_id: '' })
  const [err, setErr] = React.useState<string | null>(null)

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault()
    setErr(null)
    try {
      await createMov.mutateAsync({
        packaging_run_id: run.id ?? '',
        movement_type: form.movement_type as NonNullable<DistributionMovement['movement_type']>,
        quantity: Number(form.quantity),
        to_location: form.to_location,
        ...(form.order_id ? { order_id: form.order_id } : {}),
      })
      setForm({ movement_type: 'sale', quantity: '', to_location: '', order_id: '' })
      setShowForm(false)
    } catch (e) {
      setErr(e instanceof APIError ? e.message : 'Failed to create movement.')
    }
  }

  return (
    <div className="mt-2 pl-4 border-l-2 border-[var(--color-border)]">
      {isLoading ? (
        <p className="text-xs text-[var(--color-muted)]">Loading movements…</p>
      ) : data && data.items && data.items.length > 0 ? (
        <table className="w-full text-xs mb-2">
          <thead>
            <tr className="text-left text-[var(--color-muted)]">
              <th className="pr-3">Type</th><th className="pr-3">Qty</th>
              <th className="pr-3">To</th><th className="pr-3">Date</th><th></th>
            </tr>
          </thead>
          <tbody>
            {data.items.map((m) => (
              <tr key={m.id}>
                <td className="pr-3">{m.movement_type}</td>
                <td className="pr-3">{m.quantity}</td>
                <td className="pr-3">{m.to_location}</td>
                <td className="pr-3">{fmtDate(m.moved_at)}</td>
                <td>
                  <button
                    className="text-[var(--color-danger)] hover:underline text-xs"
                    onClick={() => deleteMov.mutate(m.id ?? '')}
                  >
                    Delete
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <p className="text-xs text-[var(--color-muted)] mb-2">No movements yet.</p>
      )}

      {!showForm ? (
        <button
          className="text-xs px-2 py-1 rounded border border-[var(--color-border)] hover:bg-[var(--color-surface-alt)]"
          onClick={() => setShowForm(true)}
        >
          + Add Movement
        </button>
      ) : (
        <form onSubmit={handleCreate} className="flex flex-wrap gap-2 items-end text-xs mt-1">
          <select
            className="border rounded px-2 py-1 text-xs"
            value={form.movement_type}
            onChange={(e) => setForm((f) => ({ ...f, movement_type: e.target.value }))}
          >
            {MOVEMENT_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
          </select>
          <input
            className="border rounded px-2 py-1 w-16 text-xs"
            type="number" min={1} placeholder="Qty"
            value={form.quantity}
            onChange={(e) => setForm((f) => ({ ...f, quantity: e.target.value }))}
            required
          />
          <input
            className="border rounded px-2 py-1 text-xs"
            placeholder="To location"
            value={form.to_location}
            onChange={(e) => setForm((f) => ({ ...f, to_location: e.target.value }))}
            required
          />
          <input
            className="border rounded px-2 py-1 text-xs"
            placeholder="Order ID (sale)"
            value={form.order_id}
            onChange={(e) => setForm((f) => ({ ...f, order_id: e.target.value }))}
          />
          <button
            type="submit"
            className="px-2 py-1 rounded bg-[var(--color-accent)] text-white text-xs disabled:opacity-50"
            disabled={createMov.isPending}
          >
            {createMov.isPending ? 'Saving…' : 'Save'}
          </button>
          <button
            type="button"
            className="px-2 py-1 rounded border text-xs"
            onClick={() => setShowForm(false)}
          >
            Cancel
          </button>
          {err && <span className="text-[var(--color-danger)] w-full">{err}</span>}
        </form>
      )}
    </div>
  )
}

function RunRow({ run }: { run: PackagingRun }) {
  const [expanded, setExpanded] = React.useState(false)
  const [editing, setEditing] = React.useState(false)
  const [notes, setNotes] = React.useState(run.notes ?? '')
  const patch = usePatchPackagingRun(run.id ?? '')
  const del = useDeletePackagingRun()
  const [delErr, setDelErr] = React.useState<string | null>(null)

  return (
    <>
      <tr>
        <td className="py-2 pr-3">
          <button
            className="text-xs text-[var(--color-accent)] hover:underline"
            onClick={() => setExpanded((x) => !x)}
          >
            {expanded ? '▲' : '▼'}
          </button>
          {' '}{run.lot_number}
        </td>
        <td className="pr-3">{run.format}</td>
        <td className="pr-3">{run.unit_volume_ml} mL</td>
        <td className="pr-3">{run.quantity}</td>
        <td className="pr-3 font-semibold">{run.stock_remaining}</td>
        <td className="pr-3">{fmtDate(run.packaged_at)}</td>
        <td className="pr-3">{fmtDate(run.best_before_date)}</td>
        <td className="pr-3">
          {editing ? (
            <form
              className="flex gap-1"
              onSubmit={async (e) => {
                e.preventDefault()
                await patch.mutateAsync({ notes })
                setEditing(false)
              }}
            >
              <input
                className="border rounded px-1 py-0.5 text-xs w-32"
                value={notes}
                onChange={(e) => setNotes(e.target.value)}
              />
              <button type="submit" className="text-xs text-[var(--color-accent)]">Save</button>
              <button type="button" className="text-xs" onClick={() => setEditing(false)}>✕</button>
            </form>
          ) : (
            <span
              className="text-xs text-[var(--color-muted)] cursor-pointer hover:underline"
              onClick={() => setEditing(true)}
            >
              {run.notes || '—'}
            </span>
          )}
        </td>
        <td>
          <button
            className="text-xs text-[var(--color-danger)] hover:underline disabled:opacity-50"
            disabled={del.isPending}
            onClick={async () => {
              setDelErr(null)
              try {
                await del.mutateAsync(run.id ?? '')
              } catch (e) {
                setDelErr(e instanceof APIError ? e.message : 'Delete failed.')
              }
            }}
          >
            Delete
          </button>
          {delErr && <span className="text-xs text-[var(--color-danger)] ml-1">{delErr}</span>}
        </td>
      </tr>
      {expanded && (
        <tr>
          <td colSpan={9}>
            <MovementsPanel run={run} />
          </td>
        </tr>
      )}
    </>
  )
}

export default function PackagingRunsPage() {
  const { data, isLoading, error } = usePackagingRuns()
  const createRun = useCreatePackagingRun()
  const [showForm, setShowForm] = React.useState(false)
  const [form, setForm] = React.useState({
    batch_id: '', format: 'can', unit_volume_ml: '', quantity: '',
    lot_number: '', packaged_at: '', best_before_date: '', notes: '',
  })
  const [formErr, setFormErr] = React.useState<string | null>(null)

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault()
    setFormErr(null)
    try {
      await createRun.mutateAsync({
        batch_id: form.batch_id,
        format: form.format as NonNullable<PackagingRun['format']>,
        unit_volume_ml: Number(form.unit_volume_ml),
        quantity: Number(form.quantity),
        lot_number: form.lot_number,
        packaged_at: new Date(form.packaged_at).toISOString(),
        ...(form.best_before_date ? { best_before_date: new Date(form.best_before_date).toISOString() } : {}),
        ...(form.notes ? { notes: form.notes } : {}),
      })
      setForm({ batch_id: '', format: 'can', unit_volume_ml: '', quantity: '', lot_number: '', packaged_at: '', best_before_date: '', notes: '' })
      setShowForm(false)
    } catch (e) {
      setFormErr(e instanceof APIError ? e.message : 'Failed to create run.')
    }
  }

  return (
    <div className="p-6 max-w-6xl mx-auto">
      <div className="flex justify-between items-center mb-4">
        <h1 className="text-xl font-semibold">Packaging Runs</h1>
        <button
          className="px-3 py-1.5 rounded bg-[var(--color-accent)] text-white text-sm hover:opacity-90"
          onClick={() => setShowForm((x) => !x)}
        >
          {showForm ? 'Cancel' : '+ New Run'}
        </button>
      </div>

      {showForm && (
        <form
          onSubmit={handleCreate}
          className="mb-6 p-4 border rounded grid grid-cols-2 md:grid-cols-3 gap-3 text-sm bg-[var(--color-surface)]"
        >
          <div className="col-span-2 md:col-span-3 font-medium">New Packaging Run</div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Batch ID</label>
            <input className="w-full border rounded px-2 py-1 text-sm" placeholder="UUID"
              value={form.batch_id} onChange={(e) => setForm((f) => ({ ...f, batch_id: e.target.value }))} required />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Format</label>
            <select className="w-full border rounded px-2 py-1 text-sm"
              value={form.format} onChange={(e) => setForm((f) => ({ ...f, format: e.target.value }))}>
              {FORMATS.map((f) => <option key={f} value={f}>{f}</option>)}
            </select>
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Unit Volume (mL)</label>
            <input className="w-full border rounded px-2 py-1 text-sm" type="number" min={1} placeholder="330"
              value={form.unit_volume_ml} onChange={(e) => setForm((f) => ({ ...f, unit_volume_ml: e.target.value }))} required />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Quantity</label>
            <input className="w-full border rounded px-2 py-1 text-sm" type="number" min={1}
              value={form.quantity} onChange={(e) => setForm((f) => ({ ...f, quantity: e.target.value }))} required />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Lot Number</label>
            <input className="w-full border rounded px-2 py-1 text-sm" placeholder="LOT-001"
              value={form.lot_number} onChange={(e) => setForm((f) => ({ ...f, lot_number: e.target.value }))} required />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Packaged At</label>
            <input className="w-full border rounded px-2 py-1 text-sm" type="date"
              value={form.packaged_at} onChange={(e) => setForm((f) => ({ ...f, packaged_at: e.target.value }))} required />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Best Before (optional)</label>
            <input className="w-full border rounded px-2 py-1 text-sm" type="date"
              value={form.best_before_date} onChange={(e) => setForm((f) => ({ ...f, best_before_date: e.target.value }))} />
          </div>
          <div className="col-span-2">
            <label className="block text-xs text-[var(--color-muted)] mb-1">Notes</label>
            <input className="w-full border rounded px-2 py-1 text-sm"
              value={form.notes} onChange={(e) => setForm((f) => ({ ...f, notes: e.target.value }))} />
          </div>
          {formErr && <div className="col-span-2 md:col-span-3 text-xs text-[var(--color-danger)]">{formErr}</div>}
          <div className="col-span-2 md:col-span-3 flex gap-2">
            <button type="submit" disabled={createRun.isPending}
              className="px-3 py-1.5 rounded bg-[var(--color-accent)] text-white text-sm disabled:opacity-50">
              {createRun.isPending ? 'Creating…' : 'Create Run'}
            </button>
          </div>
        </form>
      )}

      {isLoading && <p className="text-[var(--color-muted)]">Loading…</p>}
      {error && <p className="text-[var(--color-danger)]">Failed to load packaging runs.</p>}

      {data && data.items && data.items.length === 0 && (
        <p className="text-[var(--color-muted)] text-sm">No packaging runs yet.</p>
      )}

      {data && data.items && data.items.length > 0 && (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-[var(--color-muted)] border-b">
                <th className="py-2 pr-3">Lot Number</th>
                <th className="pr-3">Format</th>
                <th className="pr-3">Unit Vol</th>
                <th className="pr-3">Qty</th>
                <th className="pr-3">Stock</th>
                <th className="pr-3">Packaged</th>
                <th className="pr-3">Best Before</th>
                <th className="pr-3">Notes</th>
                <th></th>
              </tr>
            </thead>
            <tbody className="divide-y divide-[var(--color-border)]">
              {data.items.map((run) => <RunRow key={run.id} run={run} />)}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
