import React from 'react'
import { APIError } from '../../api/error'
import {
  usePurchaseOrders, useCreatePO, usePatchPO, useDeletePO,
  useAddLine, useDeleteLine, useReceivePO,
} from './hooks/useProcurement'
import { useSuppliers } from './hooks/useProcurement'
import { SortableHeader } from '../../components/ui/SortableHeader'
import type { components } from '../../api/generated'

type PurchaseOrder = components['schemas']['PurchaseOrder']
type PurchaseOrderLine = components['schemas']['PurchaseOrderLine']

const STATUSES = ['draft', 'sent', 'partially_received', 'received', 'cancelled']
const INGREDIENT_TYPES = ['fermentable', 'hop', 'yeast', 'adjunct', 'other']

function fmtDate(s: string | null | undefined): string {
  if (!s) return '—'
  return String(s).slice(0, 10)
}

function fmtGBP(pence: number | null | undefined): string {
  if (pence == null) return '—'
  return `£${(pence / 100).toFixed(2)}`
}

function StatusBadge({ status }: { status: string | null | undefined }) {
  const colors: Record<string, string> = {
    draft: 'bg-gray-100 text-gray-600',
    sent: 'bg-blue-100 text-blue-700',
    partially_received: 'bg-yellow-100 text-yellow-700',
    received: 'bg-green-100 text-green-700',
    cancelled: 'bg-red-100 text-red-600',
  }
  const cls = colors[status ?? ''] ?? 'bg-gray-100 text-gray-600'
  return <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${cls}`}>{status ?? '—'}</span>
}

function LinesPanel({ po }: { po: PurchaseOrder }) {
  const addLine = useAddLine(po.id ?? '')
  const deleteLine = useDeleteLine(po.id ?? '')
  const [showForm, setShowForm] = React.useState(false)
  const [form, setForm] = React.useState({
    ingredient_type: 'fermentable', ingredient_name: '', quantity: '',
    unit: 'kg', unit_cost_pence: '', unit_cost_currency: 'GBP',
  })
  const [err, setErr] = React.useState<string | null>(null)

  const isDraft = po.status === 'draft'

  async function handleAdd(e: React.FormEvent) {
    e.preventDefault()
    setErr(null)
    try {
      await addLine.mutateAsync({
        ingredient_type: form.ingredient_type as NonNullable<PurchaseOrderLine['ingredient_type']>,
        ingredient_name: form.ingredient_name,
        quantity: Number(form.quantity),
        unit: form.unit,
        unit_cost_pence: Math.round(Number(form.unit_cost_pence) * 100),
        unit_cost_currency: form.unit_cost_currency || 'GBP',
      })
      setForm({ ingredient_type: 'fermentable', ingredient_name: '', quantity: '', unit: 'kg', unit_cost_pence: '', unit_cost_currency: 'GBP' })
      setShowForm(false)
    } catch (e) {
      setErr(e instanceof APIError ? e.message : 'Failed to add line.')
    }
  }

  const lines = po.lines ?? []

  return (
    <div className="mt-2 pl-4 border-l-2 border-[var(--color-border)]">
      {lines.length > 0 ? (
        <table className="w-full text-xs mb-2">
          <thead>
            <tr className="text-left text-[var(--color-muted)]">
              <th className="pr-3">Type</th>
              <th className="pr-3">Ingredient</th>
              <th className="pr-3">Qty</th>
              <th className="pr-3">Unit</th>
              <th className="pr-3">Unit Cost</th>
              <th className="pr-3">Received</th>
              {isDraft && <th></th>}
            </tr>
          </thead>
          <tbody>
            {lines.map((l) => (
              <tr key={l.id}>
                <td className="pr-3">{l.ingredient_type}</td>
                <td className="pr-3 font-medium">{l.ingredient_name}</td>
                <td className="pr-3">{l.quantity}</td>
                <td className="pr-3">{l.unit}</td>
                <td className="pr-3">{fmtGBP(l.unit_cost_pence)}</td>
                <td className="pr-3">{l.received_quantity != null ? l.received_quantity : '—'}</td>
                {isDraft && (
                  <td>
                    <button
                      className="text-[var(--color-danger)] hover:underline text-xs"
                      onClick={() => deleteLine.mutate(l.id ?? '')}
                    >
                      Remove
                    </button>
                  </td>
                )}
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <p className="text-xs text-[var(--color-muted)] mb-2">No lines yet.</p>
      )}

      {isDraft && !showForm && (
        <button
          className="text-xs px-2 py-1 rounded border border-[var(--color-border)] hover:bg-[var(--color-surface-alt)]"
          onClick={() => setShowForm(true)}
        >
          + Add Line
        </button>
      )}

      {isDraft && showForm && (
        <form onSubmit={handleAdd} className="flex flex-wrap gap-2 items-end text-xs mt-1">
          <select
            className="border rounded px-2 py-1 text-xs"
            value={form.ingredient_type}
            onChange={(e) => setForm((f) => ({ ...f, ingredient_type: e.target.value }))}
          >
            {INGREDIENT_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
          </select>
          <input
            className="border rounded px-2 py-1 text-xs w-28"
            placeholder="Ingredient name"
            value={form.ingredient_name}
            onChange={(e) => setForm((f) => ({ ...f, ingredient_name: e.target.value }))}
            required
          />
          <input
            className="border rounded px-2 py-1 w-16 text-xs"
            type="number" min={0.001} step="any" placeholder="Qty"
            value={form.quantity}
            onChange={(e) => setForm((f) => ({ ...f, quantity: e.target.value }))}
            required
          />
          <input
            className="border rounded px-2 py-1 w-12 text-xs"
            placeholder="kg"
            value={form.unit}
            onChange={(e) => setForm((f) => ({ ...f, unit: e.target.value }))}
            required
          />
          <input
            className="border rounded px-2 py-1 w-20 text-xs"
            type="number" min={0} step="0.01" placeholder="Unit cost £"
            value={form.unit_cost_pence}
            onChange={(e) => setForm((f) => ({ ...f, unit_cost_pence: e.target.value }))}
            required
          />
          <button
            type="submit"
            className="px-2 py-1 rounded bg-[var(--color-accent)] text-white text-xs disabled:opacity-50"
            disabled={addLine.isPending}
          >
            {addLine.isPending ? 'Adding…' : 'Add'}
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

function ReceivePanel({ po, onClose }: { po: PurchaseOrder; onClose: () => void }) {
  const receivePO = useReceivePO(po.id ?? '')
  const lines = po.lines ?? []
  const [quantities, setQuantities] = React.useState<Record<string, string>>(
    Object.fromEntries(lines.map((l) => [l.id ?? '', String(l.quantity)]))
  )
  const [err, setErr] = React.useState<string | null>(null)

  async function handleReceive(e: React.FormEvent) {
    e.preventDefault()
    setErr(null)
    try {
      await receivePO.mutateAsync({
        lines: lines
          .filter((l) => l.id && quantities[l.id] !== '')
          .map((l) => ({ line_id: l.id ?? '', received_quantity: Number(quantities[l.id ?? '']) })),
      })
      onClose()
    } catch (e) {
      setErr(e instanceof APIError ? e.message : 'Receive failed.')
    }
  }

  return (
    <div className="mt-2 p-3 border rounded bg-[var(--color-surface)] text-sm">
      <div className="font-medium mb-2">Record Receipt</div>
      <form onSubmit={handleReceive}>
        <table className="w-full text-xs mb-3">
          <thead>
            <tr className="text-left text-[var(--color-muted)]">
              <th className="pr-3">Ingredient</th>
              <th className="pr-3">Ordered</th>
              <th className="pr-3">Received</th>
            </tr>
          </thead>
          <tbody>
            {lines.map((l) => (
              <tr key={l.id}>
                <td className="pr-3">{l.ingredient_name}</td>
                <td className="pr-3">{l.quantity} {l.unit}</td>
                <td>
                  <input
                    className="border rounded px-2 py-0.5 w-20 text-xs"
                    type="number" min={0} step="any"
                    value={quantities[l.id ?? ''] ?? ''}
                    onChange={(e) => setQuantities((q) => ({ ...q, [l.id ?? '']: e.target.value }))}
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        {err && <p className="text-xs text-[var(--color-danger)] mb-2">{err}</p>}
        <div className="flex gap-2">
          <button type="submit" disabled={receivePO.isPending}
            className="px-3 py-1 rounded bg-[var(--color-accent)] text-white text-xs disabled:opacity-50">
            {receivePO.isPending ? 'Saving…' : 'Save Receipt'}
          </button>
          <button type="button" className="px-3 py-1 rounded border text-xs" onClick={onClose}>Cancel</button>
        </div>
      </form>
    </div>
  )
}

const NEXT_STATUSES: Record<string, string[]> = {
  draft: ['sent', 'cancelled'],
  sent: ['cancelled'],
  partially_received: [],
  received: [],
  cancelled: [],
}

function PORow({ po }: { po: PurchaseOrder }) {
  const [expanded, setExpanded] = React.useState(false)
  const [showReceive, setShowReceive] = React.useState(false)
  const [statusErr, setStatusErr] = React.useState<string | null>(null)
  const patchPO = usePatchPO(po.id ?? '')
  const deletePO = useDeletePO()

  const canReceive = po.status === 'sent' || po.status === 'partially_received'
  const nextStatuses = NEXT_STATUSES[po.status ?? ''] ?? []

  async function handleStatus(status: string) {
    setStatusErr(null)
    try {
      await patchPO.mutateAsync({ status: status as 'sent' | 'cancelled' | 'received' | 'partially_received' })
    } catch (e) {
      setStatusErr(e instanceof APIError ? e.message : 'Status update failed.')
    }
  }

  async function handleDelete() {
    setStatusErr(null)
    try {
      await deletePO.mutateAsync(po.id ?? '')
    } catch (e) {
      setStatusErr(e instanceof APIError ? e.message : 'Delete failed.')
    }
  }

  return (
    <>
      <tr className="border-b border-[var(--color-border)]">
        <td className="py-2 pr-3">
          <button
            className="text-xs text-[var(--color-accent)] hover:underline mr-1"
            onClick={() => { setExpanded((x) => !x); setShowReceive(false) }}
          >
            {expanded ? '▲' : '▼'}
          </button>
          <span className="font-mono text-xs">{po.po_number}</span>
        </td>
        <td className="pr-3 text-sm">{po.supplier_name}</td>
        <td className="pr-3"><StatusBadge status={po.status} /></td>
        <td className="pr-3 text-sm">{fmtDate(po.order_date)}</td>
        <td className="pr-3 text-sm">{fmtDate(po.expected_delivery)}</td>
        <td className="pr-3 text-sm">{po.lines?.length ?? 0} lines</td>
        <td className="text-sm">
          <div className="flex flex-wrap gap-1 items-center">
            {nextStatuses.map((s) => (
              <button key={s}
                className="text-xs px-2 py-0.5 rounded border border-[var(--color-border)] hover:bg-[var(--color-surface-alt)] disabled:opacity-50"
                disabled={patchPO.isPending}
                onClick={() => handleStatus(s)}
              >
                → {s}
              </button>
            ))}
            {canReceive && (
              <button
                className="text-xs px-2 py-0.5 rounded bg-green-600 text-white hover:opacity-90"
                onClick={() => { setShowReceive((x) => !x); setExpanded(true) }}
              >
                Receive
              </button>
            )}
            {po.status === 'draft' && (
              <button
                className="text-xs text-[var(--color-danger)] hover:underline disabled:opacity-50"
                disabled={deletePO.isPending}
                onClick={handleDelete}
              >
                Delete
              </button>
            )}
            {statusErr && <span className="text-xs text-[var(--color-danger)]">{statusErr}</span>}
          </div>
        </td>
      </tr>
      {expanded && (
        <tr>
          <td colSpan={7} className="py-1 px-2">
            {showReceive ? (
              <ReceivePanel po={po} onClose={() => setShowReceive(false)} />
            ) : (
              <LinesPanel po={po} />
            )}
          </td>
        </tr>
      )}
    </>
  )
}

export default function PurchaseOrdersPage() {
  const { data: suppliersData } = useSuppliers({ page_size: 200 })
  const [statusFilter, setStatusFilter] = React.useState('')
  const [sort, setSort] = React.useState('')
  const { data, isLoading, error } = usePurchaseOrders({ status: statusFilter || undefined, sort: sort || undefined })
  const createPO = useCreatePO()
  const [showForm, setShowForm] = React.useState(false)
  const [form, setForm] = React.useState({ supplier_id: '', expected_delivery: '', notes: '' })
  const [formErr, setFormErr] = React.useState<string | null>(null)

  const suppliers = suppliersData?.items ?? []

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault()
    setFormErr(null)
    try {
      await createPO.mutateAsync({
        supplier_id: form.supplier_id,
        ...(form.expected_delivery ? { expected_delivery: form.expected_delivery } : {}),
        ...(form.notes ? { notes: form.notes } : {}),
      })
      setForm({ supplier_id: '', expected_delivery: '', notes: '' })
      setShowForm(false)
    } catch (e) {
      setFormErr(e instanceof APIError ? e.message : 'Failed to create purchase order.')
    }
  }

  return (
    <div className="p-6 max-w-6xl mx-auto">
      <div className="flex justify-between items-center mb-4">
        <h1 className="text-xl font-semibold">Purchase Orders</h1>
        <button
          className="px-3 py-1.5 rounded bg-[var(--color-accent)] text-white text-sm hover:opacity-90"
          onClick={() => setShowForm((x) => !x)}
        >
          {showForm ? 'Cancel' : '+ New PO'}
        </button>
      </div>

      {showForm && (
        <form onSubmit={handleCreate}
          className="mb-6 p-4 border rounded grid grid-cols-2 md:grid-cols-3 gap-3 text-sm bg-[var(--color-surface)]">
          <div className="col-span-2 md:col-span-3 font-medium">New Purchase Order</div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Supplier *</label>
            <select className="w-full border rounded px-2 py-1 text-sm"
              value={form.supplier_id}
              onChange={(e) => setForm((f) => ({ ...f, supplier_id: e.target.value }))}
              required>
              <option value="">Select supplier…</option>
              {suppliers.map((s) => <option key={s.id} value={s.id ?? ''}>{s.name}</option>)}
            </select>
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Expected Delivery</label>
            <input className="w-full border rounded px-2 py-1 text-sm" type="date"
              value={form.expected_delivery}
              onChange={(e) => setForm((f) => ({ ...f, expected_delivery: e.target.value }))} />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Notes</label>
            <input className="w-full border rounded px-2 py-1 text-sm"
              value={form.notes}
              onChange={(e) => setForm((f) => ({ ...f, notes: e.target.value }))} />
          </div>
          {formErr && <div className="col-span-2 md:col-span-3 text-xs text-[var(--color-danger)]">{formErr}</div>}
          <div className="col-span-2 md:col-span-3 flex gap-2">
            <button type="submit" disabled={createPO.isPending}
              className="px-3 py-1.5 rounded bg-[var(--color-accent)] text-white text-sm disabled:opacity-50">
              {createPO.isPending ? 'Creating…' : 'Create PO'}
            </button>
          </div>
        </form>
      )}

      <div className="flex gap-2 mb-4">
        <select
          className="border rounded px-2 py-1 text-sm"
          value={statusFilter}
          onChange={(e) => setStatusFilter(e.target.value)}
        >
          <option value="">All statuses</option>
          {STATUSES.map((s) => <option key={s} value={s}>{s}</option>)}
        </select>
      </div>

      {isLoading && <p className="text-[var(--color-muted)]">Loading…</p>}
      {error && <p className="text-[var(--color-danger)]">Failed to load purchase orders.</p>}

      {data && (!data.items || data.items.length === 0) && (
        <p className="text-[var(--color-muted)] text-sm">No purchase orders found.</p>
      )}

      {data && data.items && data.items.length > 0 && (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs uppercase text-[var(--color-muted)] border-b border-[var(--color-border)]">
                <SortableHeader column="po_number" label="PO Number" sort={sort} onSort={setSort} className="py-2 pr-3" />
                <SortableHeader column="supplier_name" label="Supplier" sort={sort} onSort={setSort} className="pr-3" />
                <SortableHeader column="status" label="Status" sort={sort} onSort={setSort} className="pr-3" />
                <SortableHeader column="order_date" label="Order Date" sort={sort} onSort={setSort} className="pr-3" />
                <SortableHeader column="expected_delivery" label="Expected" sort={sort} onSort={setSort} className="pr-3" />
                <th className="pr-3 font-medium">Lines</th>
                <th className="font-medium">Actions</th>
              </tr>
            </thead>
            <tbody>
              {data.items.map((po) => <PORow key={po.id} po={po} />)}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
