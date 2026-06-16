import React from 'react'
import { APIError } from '../../api/error'
import { useDistributionMovements, useCreateDistributionMovement, useDeleteDistributionMovement } from './hooks/usePackaging'
import type { components } from '../../api/generated'

type DistributionMovement = components['schemas']['DistributionMovement']

const MOVEMENT_TYPES = ['sale', 'taproom_transfer', 'internal_transfer', 'sample', 'return', 'disposal']

function fmtDate(s: string | null | undefined): string {
  if (!s) return '—'
  return new Date(String(s)).toLocaleDateString()
}

export default function DistributionMovementsPage() {
  const [filterType, setFilterType] = React.useState('')
  const { data, isLoading, error } = useDistributionMovements({ movement_type: filterType || undefined })
  const createMov = useCreateDistributionMovement()
  const deleteMov = useDeleteDistributionMovement()
  const [showForm, setShowForm] = React.useState(false)
  const [form, setForm] = React.useState({
    packaging_run_id: '', movement_type: 'sale', quantity: '',
    from_location: '', to_location: '', order_id: '', reference: '', notes: '',
  })
  const [formErr, setFormErr] = React.useState<string | null>(null)

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault()
    setFormErr(null)
    try {
      await createMov.mutateAsync({
        packaging_run_id: form.packaging_run_id,
        movement_type: form.movement_type as NonNullable<DistributionMovement['movement_type']>,
        quantity: Number(form.quantity),
        to_location: form.to_location,
        ...(form.from_location ? { from_location: form.from_location } : {}),
        ...(form.order_id ? { order_id: form.order_id } : {}),
        ...(form.reference ? { reference: form.reference } : {}),
        ...(form.notes ? { notes: form.notes } : {}),
      })
      setForm({ packaging_run_id: '', movement_type: 'sale', quantity: '', from_location: '', to_location: '', order_id: '', reference: '', notes: '' })
      setShowForm(false)
    } catch (e) {
      setFormErr(e instanceof APIError ? e.message : 'Failed to record movement.')
    }
  }

  return (
    <div className="p-6 max-w-6xl mx-auto">
      <div className="flex justify-between items-center mb-4">
        <h1 className="text-xl font-semibold">Distribution Movements</h1>
        <button
          className="px-3 py-1.5 rounded bg-[var(--color-accent)] text-white text-sm hover:opacity-90"
          onClick={() => setShowForm((x) => !x)}
        >
          {showForm ? 'Cancel' : '+ Record Movement'}
        </button>
      </div>

      {showForm && (
        <form
          onSubmit={handleCreate}
          className="mb-6 p-4 border rounded grid grid-cols-2 md:grid-cols-3 gap-3 text-sm bg-[var(--color-surface)]"
        >
          <div className="col-span-2 md:col-span-3 font-medium">New Distribution Movement</div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Packaging Run ID</label>
            <input className="w-full border rounded px-2 py-1 text-sm" placeholder="UUID"
              value={form.packaging_run_id} onChange={(e) => setForm((f) => ({ ...f, packaging_run_id: e.target.value }))} required />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Movement Type</label>
            <select className="w-full border rounded px-2 py-1 text-sm"
              value={form.movement_type} onChange={(e) => setForm((f) => ({ ...f, movement_type: e.target.value }))}>
              {MOVEMENT_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
            </select>
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Quantity</label>
            <input className="w-full border rounded px-2 py-1 text-sm" type="number" min={1}
              value={form.quantity} onChange={(e) => setForm((f) => ({ ...f, quantity: e.target.value }))} required />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">From Location</label>
            <input className="w-full border rounded px-2 py-1 text-sm" placeholder="brewery"
              value={form.from_location} onChange={(e) => setForm((f) => ({ ...f, from_location: e.target.value }))} />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">To Location</label>
            <input className="w-full border rounded px-2 py-1 text-sm" placeholder="Customer Depot"
              value={form.to_location} onChange={(e) => setForm((f) => ({ ...f, to_location: e.target.value }))} required />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Order ID (required for sales)</label>
            <input className="w-full border rounded px-2 py-1 text-sm" placeholder="UUID"
              value={form.order_id} onChange={(e) => setForm((f) => ({ ...f, order_id: e.target.value }))} />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Reference</label>
            <input className="w-full border rounded px-2 py-1 text-sm"
              value={form.reference} onChange={(e) => setForm((f) => ({ ...f, reference: e.target.value }))} />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Notes</label>
            <input className="w-full border rounded px-2 py-1 text-sm"
              value={form.notes} onChange={(e) => setForm((f) => ({ ...f, notes: e.target.value }))} />
          </div>
          {formErr && <div className="col-span-2 md:col-span-3 text-xs text-[var(--color-danger)]">{formErr}</div>}
          <div className="col-span-2 md:col-span-3 flex gap-2">
            <button type="submit" disabled={createMov.isPending}
              className="px-3 py-1.5 rounded bg-[var(--color-accent)] text-white text-sm disabled:opacity-50">
              {createMov.isPending ? 'Saving…' : 'Save Movement'}
            </button>
          </div>
        </form>
      )}

      <div className="mb-4 flex gap-2 items-center text-sm">
        <label className="text-[var(--color-muted)]">Filter by type:</label>
        <select
          className="border rounded px-2 py-1 text-sm"
          value={filterType}
          onChange={(e) => setFilterType(e.target.value)}
        >
          <option value="">All</option>
          {MOVEMENT_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
        </select>
      </div>

      {isLoading && <p className="text-[var(--color-muted)]">Loading…</p>}
      {error && <p className="text-[var(--color-danger)]">Failed to load movements.</p>}

      {data && data.items && data.items.length === 0 && (
        <p className="text-[var(--color-muted)] text-sm">No movements yet.</p>
      )}

      {data && data.items && data.items.length > 0 && (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-[var(--color-muted)] border-b">
                <th className="py-2 pr-3">Type</th>
                <th className="pr-3">Qty</th>
                <th className="pr-3">From</th>
                <th className="pr-3">To</th>
                <th className="pr-3">Date</th>
                <th className="pr-3">Reference</th>
                <th></th>
              </tr>
            </thead>
            <tbody className="divide-y divide-[var(--color-border)]">
              {data.items.map((m) => (
                <tr key={m.id}>
                  <td className="py-2 pr-3">{m.movement_type}</td>
                  <td className="pr-3">{m.quantity}</td>
                  <td className="pr-3">{m.from_location}</td>
                  <td className="pr-3">{m.to_location}</td>
                  <td className="pr-3">{fmtDate(m.moved_at)}</td>
                  <td className="pr-3">{m.reference || '—'}</td>
                  <td>
                    <button
                      className="text-xs text-[var(--color-danger)] hover:underline disabled:opacity-50"
                      disabled={deleteMov.isPending}
                      onClick={() => deleteMov.mutate(m.id ?? '')}
                    >
                      Delete
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="text-xs text-[var(--color-muted)] mt-2">
            {data.total} total · page {data.page} of {data.total_pages}
          </p>
        </div>
      )}
    </div>
  )
}
