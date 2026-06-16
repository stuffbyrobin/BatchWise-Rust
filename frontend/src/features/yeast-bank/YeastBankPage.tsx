import React from 'react'
import { APIError } from '../../api/error'
import {
  useYeastBank, useCreateYeastBankEntry, usePatchYeastBankEntry, useDeleteYeastBankEntry, useHarvestYeast,
  usePropagations, useCreatePropagation, usePatchPropagation, useDeletePropagation,
} from './hooks/useYeastBank'
import type { components } from '../../api/generated'

type YeastBankEntry = components['schemas']['YeastBankEntry']
type Propagation = components['schemas']['Propagation']

const STATUS_COLORS: Record<string, string> = {
  active: 'text-green-600',
  depleted: 'text-yellow-600',
  discarded: 'text-[var(--color-muted)]',
}

function fmtDate(s: string | null | undefined): string {
  if (!s) return '—'
  return String(s).slice(0, 10)
}

function PropagationsPanel({ entry }: { entry: YeastBankEntry }) {
  const { data, isLoading } = usePropagations(entry.id ?? '')
  const create = useCreatePropagation(entry.id ?? '')
  const [showForm, setShowForm] = React.useState(false)
  const [form, setForm] = React.useState({ started_at: '', completed_at: '', volume_ml: '', batch_id: '', notes: '' })
  const [err, setErr] = React.useState<string | null>(null)
  const [completingId, setCompletingId] = React.useState<string | null>(null)

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault()
    setErr(null)
    try {
      await create.mutateAsync({
        ...(form.started_at ? { started_at: new Date(form.started_at).toISOString() } : {}),
        ...(form.completed_at ? { completed_at: new Date(form.completed_at).toISOString() } : {}),
        ...(form.volume_ml ? { volume_ml: Number(form.volume_ml) } : {}),
        ...(form.batch_id ? { batch_id: form.batch_id } : {}),
        ...(form.notes ? { notes: form.notes } : {}),
      })
      setForm({ started_at: '', completed_at: '', volume_ml: '', batch_id: '', notes: '' })
      setShowForm(false)
    } catch (e) {
      setErr(e instanceof APIError ? e.message : 'Failed to create propagation.')
    }
  }

  return (
    <div className="mt-2 pl-4 border-l-2 border-[var(--color-border)]">
      {isLoading ? (
        <p className="text-xs text-[var(--color-muted)]">Loading propagations…</p>
      ) : data && data.items && data.items.length > 0 ? (
        <table className="w-full text-xs mb-2">
          <thead>
            <tr className="text-left text-[var(--color-muted)]">
              <th className="pr-3">Started</th>
              <th className="pr-3">Completed</th>
              <th className="pr-3">Volume (mL)</th>
              <th className="pr-3">Batch ID</th>
              <th className="pr-3">Notes</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {data.items.map((p) => (
              <PropRow
                key={p.id}
                prop={p}
                bankID={entry.id ?? ''}
                completing={completingId === p.id}
                onComplete={() => setCompletingId(p.id ?? null)}
              />
            ))}
          </tbody>
        </table>
      ) : (
        <p className="text-xs text-[var(--color-muted)] mb-2">No propagations yet.</p>
      )}

      {!showForm ? (
        <button
          className="text-xs px-2 py-1 rounded border border-[var(--color-border)] hover:bg-[var(--color-surface-alt)]"
          onClick={() => setShowForm(true)}
        >
          + Log Propagation
        </button>
      ) : (
        <form onSubmit={handleCreate} className="flex flex-wrap gap-2 items-end text-xs mt-1">
          <div>
            <label className="block text-[var(--color-muted)] mb-0.5">Started</label>
            <input className="border rounded px-2 py-1 text-xs" type="date"
              value={form.started_at} onChange={(e) => setForm((f) => ({ ...f, started_at: e.target.value }))} />
          </div>
          <div>
            <label className="block text-[var(--color-muted)] mb-0.5">Completed</label>
            <input className="border rounded px-2 py-1 text-xs" type="date"
              value={form.completed_at} onChange={(e) => setForm((f) => ({ ...f, completed_at: e.target.value }))} />
          </div>
          <div>
            <label className="block text-[var(--color-muted)] mb-0.5">Volume (mL)</label>
            <input className="border rounded px-2 py-1 w-20 text-xs" type="number" min={1} placeholder="1000"
              value={form.volume_ml} onChange={(e) => setForm((f) => ({ ...f, volume_ml: e.target.value }))} />
          </div>
          <div>
            <label className="block text-[var(--color-muted)] mb-0.5">Notes</label>
            <input className="border rounded px-2 py-1 text-xs" placeholder="Optional"
              value={form.notes} onChange={(e) => setForm((f) => ({ ...f, notes: e.target.value }))} />
          </div>
          <button type="submit"
            className="px-2 py-1 rounded bg-[var(--color-accent)] text-white text-xs disabled:opacity-50"
            disabled={create.isPending}>
            {create.isPending ? 'Saving…' : 'Save'}
          </button>
          <button type="button" className="px-2 py-1 rounded border text-xs" onClick={() => setShowForm(false)}>
            Cancel
          </button>
          {err && <span className="text-[var(--color-danger)] w-full">{err}</span>}
        </form>
      )}
    </div>
  )
}

function PropRow({ prop, bankID, completing, onComplete }: {
  prop: Propagation
  bankID: string
  completing: boolean
  onComplete: () => void
}) {
  const patch = usePatchPropagation(bankID, prop.id ?? '')
  const del = useDeletePropagation(bankID)

  return (
    <tr>
      <td className="pr-3">{fmtDate(prop.started_at)}</td>
      <td className="pr-3">
        {prop.completed_at ? fmtDate(prop.completed_at) : (
          completing ? (
            <button
              className="text-xs text-[var(--color-accent)] hover:underline"
              onClick={() => patch.mutate({ completed_at: new Date().toISOString() })}
              disabled={patch.isPending}
            >
              {patch.isPending ? '…' : 'Mark done'}
            </button>
          ) : (
            <button className="text-xs text-[var(--color-muted)] hover:underline" onClick={onComplete}>
              Pending
            </button>
          )
        )}
      </td>
      <td className="pr-3">{prop.volume_ml ?? '—'}</td>
      <td className="pr-3 font-mono text-xs">{prop.batch_id ? prop.batch_id.slice(0, 8) + '…' : '—'}</td>
      <td className="pr-3">{prop.notes || '—'}</td>
      <td>
        <button
          className="text-[var(--color-danger)] hover:underline text-xs"
          onClick={() => del.mutate(prop.id ?? '')}
          disabled={del.isPending}
        >
          Delete
        </button>
      </td>
    </tr>
  )
}

function HarvestForm({ entry, onDone }: { entry: YeastBankEntry; onDone: () => void }) {
  const harvest = useHarvestYeast(entry.id ?? '')
  const [form, setForm] = React.useState({ viability_percent: '', quantity_ml: '', notes: '' })
  const [err, setErr] = React.useState<string | null>(null)

  async function handleHarvest(e: React.FormEvent) {
    e.preventDefault()
    setErr(null)
    try {
      await harvest.mutateAsync({
        ...(form.viability_percent ? { viability_percent: Number(form.viability_percent) } : {}),
        ...(form.quantity_ml ? { quantity_ml: Number(form.quantity_ml) } : {}),
        ...(form.notes ? { notes: form.notes } : {}),
      })
      onDone()
    } catch (e) {
      setErr(e instanceof APIError ? e.message : 'Harvest failed.')
    }
  }

  return (
    <form onSubmit={handleHarvest} className="flex flex-wrap gap-2 items-end text-xs mt-1 p-2 bg-[var(--color-surface-alt)] rounded">
      <span className="w-full font-medium text-sm">Harvest (Gen {(entry.generation ?? 1) + 1})</span>
      <div>
        <label className="block text-[var(--color-muted)] mb-0.5">Viability %</label>
        <input className="border rounded px-2 py-1 w-16 text-xs" type="number" min={0} max={100} placeholder="95"
          value={form.viability_percent} onChange={(e) => setForm((f) => ({ ...f, viability_percent: e.target.value }))} />
      </div>
      <div>
        <label className="block text-[var(--color-muted)] mb-0.5">Quantity (mL)</label>
        <input className="border rounded px-2 py-1 w-20 text-xs" type="number" min={0} placeholder="500"
          value={form.quantity_ml} onChange={(e) => setForm((f) => ({ ...f, quantity_ml: e.target.value }))} />
      </div>
      <div>
        <label className="block text-[var(--color-muted)] mb-0.5">Notes</label>
        <input className="border rounded px-2 py-1 text-xs" placeholder="Optional"
          value={form.notes} onChange={(e) => setForm((f) => ({ ...f, notes: e.target.value }))} />
      </div>
      <button type="submit"
        className="px-2 py-1 rounded bg-[var(--color-accent)] text-white text-xs disabled:opacity-50"
        disabled={harvest.isPending}>
        {harvest.isPending ? 'Harvesting…' : 'Confirm Harvest'}
      </button>
      <button type="button" className="px-2 py-1 rounded border text-xs" onClick={onDone}>Cancel</button>
      {err && <span className="text-[var(--color-danger)] w-full">{err}</span>}
    </form>
  )
}

function EntryRow({ entry }: { entry: YeastBankEntry }) {
  const [expanded, setExpanded] = React.useState(false)
  const [harvesting, setHarvesting] = React.useState(false)
  const [editNotes, setEditNotes] = React.useState(false)
  const [notes, setNotes] = React.useState(entry.notes ?? '')
  const patch = usePatchYeastBankEntry(entry.id ?? '')
  const del = useDeleteYeastBankEntry()
  const [delErr, setDelErr] = React.useState<string | null>(null)

  const isDiscarded = entry.status === 'discarded'

  function setStatus(status: 'active' | 'depleted' | 'discarded') {
    patch.mutate({ status })
  }

  return (
    <>
      <tr>
        <td className="py-2 pr-3">
          <button
            className="text-xs text-[var(--color-accent)] hover:underline mr-1"
            onClick={() => setExpanded((x) => !x)}
          >
            {expanded ? '▲' : '▼'}
          </button>
          {entry.name}
        </td>
        <td className="pr-3">{entry.generation ?? 1}</td>
        <td className="pr-3">
          <span className={STATUS_COLORS[entry.status ?? 'active'] ?? ''}>
            {entry.status}
          </span>
        </td>
        <td className="pr-3">{entry.viability_percent != null ? `${entry.viability_percent}%` : '—'}</td>
        <td className="pr-3">{entry.quantity_ml != null ? `${entry.quantity_ml} mL` : '—'}</td>
        <td className="pr-3">{entry.location || '—'}</td>
        <td className="pr-3">{entry.days_in_storage != null ? `${entry.days_in_storage}d` : '—'}</td>
        <td className="pr-3">
          {editNotes ? (
            <form className="flex gap-1" onSubmit={async (e) => {
              e.preventDefault()
              await patch.mutateAsync({ notes })
              setEditNotes(false)
            }}>
              <input className="border rounded px-1 py-0.5 text-xs w-32" value={notes}
                onChange={(e) => setNotes(e.target.value)} />
              <button type="submit" className="text-xs text-[var(--color-accent)]">Save</button>
              <button type="button" className="text-xs" onClick={() => setEditNotes(false)}>✕</button>
            </form>
          ) : (
            <span className="text-xs text-[var(--color-muted)] cursor-pointer hover:underline"
              onClick={() => setEditNotes(true)}>
              {entry.notes || '—'}
            </span>
          )}
        </td>
        <td className="pr-3 text-xs flex gap-1 items-center flex-wrap">
          {!isDiscarded && entry.status !== 'active' && (
            <button className="hover:underline text-green-600" onClick={() => setStatus('active')}>Active</button>
          )}
          {!isDiscarded && entry.status !== 'depleted' && (
            <button className="hover:underline text-yellow-600" onClick={() => setStatus('depleted')}>Depleted</button>
          )}
          {!isDiscarded && (
            <button className="hover:underline text-[var(--color-muted)]" onClick={() => setStatus('discarded')}>Discard</button>
          )}
          {!isDiscarded && (
            <button className="hover:underline text-[var(--color-accent)]" onClick={() => setHarvesting((x) => !x)}>Harvest</button>
          )}
          <button
            className="text-[var(--color-danger)] hover:underline disabled:opacity-50"
            disabled={del.isPending}
            onClick={async () => {
              setDelErr(null)
              try { await del.mutateAsync(entry.id ?? '') }
              catch (e) { setDelErr(e instanceof APIError ? e.message : 'Delete failed.') }
            }}
          >
            Delete
          </button>
          {delErr && <span className="text-[var(--color-danger)]">{delErr}</span>}
        </td>
      </tr>
      {harvesting && (
        <tr><td colSpan={9}><HarvestForm entry={entry} onDone={() => setHarvesting(false)} /></td></tr>
      )}
      {expanded && (
        <tr><td colSpan={9}><PropagationsPanel entry={entry} /></td></tr>
      )}
    </>
  )
}

export default function YeastBankPage() {
  const [statusFilter, setStatusFilter] = React.useState('')
  const { data, isLoading, error } = useYeastBank({ status: statusFilter || undefined })
  const create = useCreateYeastBankEntry()
  const [showForm, setShowForm] = React.useState(false)
  const [form, setForm] = React.useState({
    name: '', generation: '', harvested_at: '',
    viability_percent: '', quantity_ml: '', storage_temp_c: '', location: '', notes: '',
  })
  const [formErr, setFormErr] = React.useState<string | null>(null)

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault()
    setFormErr(null)
    try {
      await create.mutateAsync({
        name: form.name,
        ...(form.generation ? { generation: Number(form.generation) } : {}),
        ...(form.harvested_at ? { harvested_at: new Date(form.harvested_at).toISOString() } : {}),
        ...(form.viability_percent ? { viability_percent: Number(form.viability_percent) } : {}),
        ...(form.quantity_ml ? { quantity_ml: Number(form.quantity_ml) } : {}),
        ...(form.storage_temp_c ? { storage_temp_c: Number(form.storage_temp_c) } : {}),
        ...(form.location ? { location: form.location } : {}),
        ...(form.notes ? { notes: form.notes } : {}),
      })
      setForm({ name: '', generation: '', harvested_at: '', viability_percent: '', quantity_ml: '', storage_temp_c: '', location: '', notes: '' })
      setShowForm(false)
    } catch (e) {
      setFormErr(e instanceof APIError ? e.message : 'Failed to create entry.')
    }
  }

  return (
    <div className="p-6 max-w-6xl mx-auto">
      <div className="flex justify-between items-center mb-4">
        <h1 className="text-xl font-semibold">Yeast Bank</h1>
        <div className="flex gap-2 items-center">
          <select
            className="border rounded px-2 py-1 text-sm"
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value)}
          >
            <option value="">All statuses</option>
            <option value="active">Active</option>
            <option value="depleted">Depleted</option>
            <option value="discarded">Discarded</option>
          </select>
          <button
            className="px-3 py-1.5 rounded bg-[var(--color-accent)] text-white text-sm hover:opacity-90"
            onClick={() => setShowForm((x) => !x)}
          >
            {showForm ? 'Cancel' : '+ New Entry'}
          </button>
        </div>
      </div>

      {showForm && (
        <form onSubmit={handleCreate}
          className="mb-6 p-4 border rounded grid grid-cols-2 md:grid-cols-3 gap-3 text-sm bg-[var(--color-surface)]">
          <div className="col-span-2 md:col-span-3 font-medium">New Yeast Bank Entry</div>
          <div className="col-span-2 md:col-span-1">
            <label className="block text-xs text-[var(--color-muted)] mb-1">Name *</label>
            <input className="w-full border rounded px-2 py-1 text-sm" placeholder="WY1056 Batch A"
              value={form.name} onChange={(e) => setForm((f) => ({ ...f, name: e.target.value }))} required />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Generation</label>
            <input className="w-full border rounded px-2 py-1 text-sm" type="number" min={1} placeholder="1"
              value={form.generation} onChange={(e) => setForm((f) => ({ ...f, generation: e.target.value }))} />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Last Harvested</label>
            <input className="w-full border rounded px-2 py-1 text-sm" type="date"
              value={form.harvested_at} onChange={(e) => setForm((f) => ({ ...f, harvested_at: e.target.value }))} />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Viability %</label>
            <input className="w-full border rounded px-2 py-1 text-sm" type="number" min={0} max={100} placeholder="95"
              value={form.viability_percent} onChange={(e) => setForm((f) => ({ ...f, viability_percent: e.target.value }))} />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Quantity (mL)</label>
            <input className="w-full border rounded px-2 py-1 text-sm" type="number" min={0} placeholder="500"
              value={form.quantity_ml} onChange={(e) => setForm((f) => ({ ...f, quantity_ml: e.target.value }))} />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Storage Temp (&deg;C)</label>
            <input className="w-full border rounded px-2 py-1 text-sm" type="number" placeholder="2"
              value={form.storage_temp_c} onChange={(e) => setForm((f) => ({ ...f, storage_temp_c: e.target.value }))} />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Location</label>
            <input className="w-full border rounded px-2 py-1 text-sm" placeholder="Fridge 2, shelf A"
              value={form.location} onChange={(e) => setForm((f) => ({ ...f, location: e.target.value }))} />
          </div>
          <div className="col-span-2">
            <label className="block text-xs text-[var(--color-muted)] mb-1">Notes</label>
            <input className="w-full border rounded px-2 py-1 text-sm"
              value={form.notes} onChange={(e) => setForm((f) => ({ ...f, notes: e.target.value }))} />
          </div>
          {formErr && <div className="col-span-2 md:col-span-3 text-xs text-[var(--color-danger)]">{formErr}</div>}
          <div className="col-span-2 md:col-span-3">
            <button type="submit" disabled={create.isPending}
              className="px-3 py-1.5 rounded bg-[var(--color-accent)] text-white text-sm disabled:opacity-50">
              {create.isPending ? 'Creating…' : 'Create Entry'}
            </button>
          </div>
        </form>
      )}

      {isLoading && <p className="text-[var(--color-muted)]">Loading…</p>}
      {error && <p className="text-[var(--color-danger)]">Failed to load yeast bank.</p>}

      {data && data.items && data.items.length === 0 && (
        <p className="text-[var(--color-muted)] text-sm">No yeast bank entries yet.</p>
      )}

      {data && data.items && data.items.length > 0 && (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-[var(--color-muted)] border-b">
                <th className="py-2 pr-3">Name</th>
                <th className="pr-3">Gen</th>
                <th className="pr-3">Status</th>
                <th className="pr-3">Viability</th>
                <th className="pr-3">Quantity</th>
                <th className="pr-3">Location</th>
                <th className="pr-3">Age</th>
                <th className="pr-3">Notes</th>
                <th className="pr-3">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-[var(--color-border)]">
              {data.items.map((e) => <EntryRow key={e.id} entry={e} />)}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
