import React from 'react'
import { APIError } from '../../api/error'
import { useSuppliers, useCreateSupplier, usePatchSupplier, useDeleteSupplier } from './hooks/useProcurement'
import type { components } from '../../api/generated'

type Supplier = components['schemas']['Supplier']

function SupplierRow({ supplier }: { supplier: Supplier }) {
  const [editing, setEditing] = React.useState(false)
  const [form, setForm] = React.useState({
    name: supplier.name ?? '',
    contact_name: supplier.contact_name ?? '',
    email: supplier.email ?? '',
    phone: supplier.phone ?? '',
    website: supplier.website ?? '',
    notes: supplier.notes ?? '',
  })
  const [err, setErr] = React.useState<string | null>(null)
  const patch = usePatchSupplier(supplier.id ?? '')
  const del = useDeleteSupplier()

  async function handlePatch(e: React.FormEvent) {
    e.preventDefault()
    setErr(null)
    try {
      await patch.mutateAsync({
        name: form.name || undefined,
        contact_name: form.contact_name || undefined,
        email: form.email || undefined,
        phone: form.phone || undefined,
        website: form.website || undefined,
        notes: form.notes || undefined,
      })
      setEditing(false)
    } catch (e) {
      setErr(e instanceof APIError ? e.message : 'Update failed.')
    }
  }

  async function handleDelete() {
    setErr(null)
    try {
      await del.mutateAsync(supplier.id ?? '')
    } catch (e) {
      setErr(e instanceof APIError ? e.message : 'Delete failed.')
    }
  }

  if (editing) {
    return (
      <tr>
        <td colSpan={6} className="py-2">
          <form onSubmit={handlePatch} className="grid grid-cols-2 md:grid-cols-3 gap-2 text-sm p-3 border rounded bg-[var(--color-surface)]">
            <div>
              <label className="block text-xs text-[var(--color-muted)] mb-1">Name *</label>
              <input className="w-full border rounded px-2 py-1 text-sm" value={form.name}
                onChange={(e) => setForm((f) => ({ ...f, name: e.target.value }))} required />
            </div>
            <div>
              <label className="block text-xs text-[var(--color-muted)] mb-1">Contact</label>
              <input className="w-full border rounded px-2 py-1 text-sm" value={form.contact_name}
                onChange={(e) => setForm((f) => ({ ...f, contact_name: e.target.value }))} />
            </div>
            <div>
              <label className="block text-xs text-[var(--color-muted)] mb-1">Email</label>
              <input className="w-full border rounded px-2 py-1 text-sm" type="email" value={form.email}
                onChange={(e) => setForm((f) => ({ ...f, email: e.target.value }))} />
            </div>
            <div>
              <label className="block text-xs text-[var(--color-muted)] mb-1">Phone</label>
              <input className="w-full border rounded px-2 py-1 text-sm" value={form.phone}
                onChange={(e) => setForm((f) => ({ ...f, phone: e.target.value }))} />
            </div>
            <div>
              <label className="block text-xs text-[var(--color-muted)] mb-1">Website</label>
              <input className="w-full border rounded px-2 py-1 text-sm" value={form.website}
                onChange={(e) => setForm((f) => ({ ...f, website: e.target.value }))} />
            </div>
            <div>
              <label className="block text-xs text-[var(--color-muted)] mb-1">Notes</label>
              <input className="w-full border rounded px-2 py-1 text-sm" value={form.notes}
                onChange={(e) => setForm((f) => ({ ...f, notes: e.target.value }))} />
            </div>
            {err && <div className="col-span-2 md:col-span-3 text-xs text-[var(--color-danger)]">{err}</div>}
            <div className="col-span-2 md:col-span-3 flex gap-2">
              <button type="submit" disabled={patch.isPending}
                className="px-3 py-1 rounded bg-[var(--color-accent)] text-white text-sm disabled:opacity-50">
                {patch.isPending ? 'Saving…' : 'Save'}
              </button>
              <button type="button" className="px-3 py-1 rounded border text-sm"
                onClick={() => setEditing(false)}>Cancel</button>
            </div>
          </form>
        </td>
      </tr>
    )
  }

  return (
    <tr className="border-b border-[var(--color-border)]">
      <td className="py-2 pr-3 font-medium">{supplier.name}</td>
      <td className="pr-3 text-sm">{supplier.contact_name || '—'}</td>
      <td className="pr-3 text-sm">{supplier.email || '—'}</td>
      <td className="pr-3 text-sm">{supplier.phone || '—'}</td>
      <td className="pr-3 text-sm text-[var(--color-muted)]">{supplier.notes || '—'}</td>
      <td className="text-sm flex gap-2">
        <button className="text-[var(--color-accent)] hover:underline text-xs"
          onClick={() => setEditing(true)}>Edit</button>
        <button className="text-[var(--color-danger)] hover:underline text-xs disabled:opacity-50"
          disabled={del.isPending}
          onClick={handleDelete}>Delete</button>
        {err && <span className="text-xs text-[var(--color-danger)]">{err}</span>}
      </td>
    </tr>
  )
}

export default function SuppliersPage() {
  const { data, isLoading, error } = useSuppliers()
  const createSupplier = useCreateSupplier()
  const [showForm, setShowForm] = React.useState(false)
  const [form, setForm] = React.useState({ name: '', contact_name: '', email: '', phone: '', website: '', notes: '' })
  const [formErr, setFormErr] = React.useState<string | null>(null)

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault()
    setFormErr(null)
    try {
      await createSupplier.mutateAsync({
        name: form.name,
        ...(form.contact_name ? { contact_name: form.contact_name } : {}),
        ...(form.email ? { email: form.email } : {}),
        ...(form.phone ? { phone: form.phone } : {}),
        ...(form.website ? { website: form.website } : {}),
        ...(form.notes ? { notes: form.notes } : {}),
      })
      setForm({ name: '', contact_name: '', email: '', phone: '', website: '', notes: '' })
      setShowForm(false)
    } catch (e) {
      setFormErr(e instanceof APIError ? e.message : 'Failed to create supplier.')
    }
  }

  return (
    <div className="p-6 max-w-5xl mx-auto">
      <div className="flex justify-between items-center mb-4">
        <h1 className="text-xl font-semibold">Suppliers</h1>
        <button
          className="px-3 py-1.5 rounded bg-[var(--color-accent)] text-white text-sm hover:opacity-90"
          onClick={() => setShowForm((x) => !x)}
        >
          {showForm ? 'Cancel' : '+ New Supplier'}
        </button>
      </div>

      {showForm && (
        <form onSubmit={handleCreate}
          className="mb-6 p-4 border rounded grid grid-cols-2 md:grid-cols-3 gap-3 text-sm bg-[var(--color-surface)]">
          <div className="col-span-2 md:col-span-3 font-medium">New Supplier</div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Name *</label>
            <input className="w-full border rounded px-2 py-1 text-sm" placeholder="Hop Valley Ltd"
              value={form.name} onChange={(e) => setForm((f) => ({ ...f, name: e.target.value }))} required />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Contact</label>
            <input className="w-full border rounded px-2 py-1 text-sm" placeholder="Jane Smith"
              value={form.contact_name} onChange={(e) => setForm((f) => ({ ...f, contact_name: e.target.value }))} />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Email</label>
            <input className="w-full border rounded px-2 py-1 text-sm" type="email" placeholder="jane@supplier.com"
              value={form.email} onChange={(e) => setForm((f) => ({ ...f, email: e.target.value }))} />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Phone</label>
            <input className="w-full border rounded px-2 py-1 text-sm" placeholder="+44 …"
              value={form.phone} onChange={(e) => setForm((f) => ({ ...f, phone: e.target.value }))} />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Website</label>
            <input className="w-full border rounded px-2 py-1 text-sm" placeholder="https://…"
              value={form.website} onChange={(e) => setForm((f) => ({ ...f, website: e.target.value }))} />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Notes</label>
            <input className="w-full border rounded px-2 py-1 text-sm"
              value={form.notes} onChange={(e) => setForm((f) => ({ ...f, notes: e.target.value }))} />
          </div>
          {formErr && <div className="col-span-2 md:col-span-3 text-xs text-[var(--color-danger)]">{formErr}</div>}
          <div className="col-span-2 md:col-span-3 flex gap-2">
            <button type="submit" disabled={createSupplier.isPending}
              className="px-3 py-1.5 rounded bg-[var(--color-accent)] text-white text-sm disabled:opacity-50">
              {createSupplier.isPending ? 'Creating…' : 'Create'}
            </button>
          </div>
        </form>
      )}

      {isLoading && <p className="text-[var(--color-muted)]">Loading…</p>}
      {error && <p className="text-[var(--color-danger)]">Failed to load suppliers.</p>}

      {data && (!data.items || data.items.length === 0) && (
        <p className="text-[var(--color-muted)] text-sm">No suppliers yet.</p>
      )}

      {data && data.items && data.items.length > 0 && (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-[var(--color-muted)] border-b border-[var(--color-border)]">
                <th className="py-2 pr-3">Name</th>
                <th className="pr-3">Contact</th>
                <th className="pr-3">Email</th>
                <th className="pr-3">Phone</th>
                <th className="pr-3">Notes</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {data.items.map((s) => <SupplierRow key={s.id} supplier={s} />)}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
