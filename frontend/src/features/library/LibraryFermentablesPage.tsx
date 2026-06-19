import React from 'react'
import type { components } from '../../api/generated'
import { APIError } from '../../api/error'
import { SortableHeader } from '../../components/ui/SortableHeader'
import {
  useFermentables,
  useCreateFermentable,
  useUpdateFermentable,
  useDeleteFermentable,
} from './hooks/useLibrary'

type Fermentable = components['schemas']['LibraryFermentable']
type FermentableRequest = components['schemas']['LibraryFermentableRequest']

const SYSTEM_TENANT = '00000000-0000-0000-0000-000000000000'

function fmt(min: number | null | undefined, max: number | null | undefined, unit = ''): string {
  if (min == null && max == null) return '—'
  if (min != null && max != null && min === max) return `${min}${unit}`
  if (min != null && max != null) return `${min}–${max}${unit}`
  if (min != null) return `≥${min}${unit}`
  return `≤${max}${unit}`
}

function blank(): FermentableRequest {
  return { name: '' }
}

function toRequest(f: Fermentable): FermentableRequest {
  return {
    name: f.name,
    supplier: f.supplier ?? undefined,
    type: f.type ?? undefined,
    colour_ebc_min: f.colour_ebc_min ?? undefined,
    colour_ebc_max: f.colour_ebc_max ?? undefined,
    extract_litres_per_kg: f.extract_litres_per_kg ?? undefined,
    moisture_pct_max: f.moisture_pct_max ?? undefined,
    tn_min: f.tn_min ?? undefined,
    tn_max: f.tn_max ?? undefined,
    snr_min: f.snr_min ?? undefined,
    snr_max: f.snr_max ?? undefined,
    attributes: f.attributes ?? undefined,
    notes: f.notes ?? undefined,
  }
}

function num(v: string): number | undefined {
  return v === '' ? undefined : Number(v)
}

export function LibraryFermentablesPage() {
  const [search, setSearch] = React.useState('')
  const [typeFilter, setTypeFilter] = React.useState('')
  const [sort, setSort] = React.useState('')
  const params = {
    page_size: 100,
    ...(search ? { name: search } : {}),
    ...(typeFilter ? { type: typeFilter } : {}),
    ...(sort ? { sort } : {}),
  }
  const { data, isLoading, isError, error, refetch } = useFermentables(params)
  const createMut = useCreateFermentable()
  const deleteMut = useDeleteFermentable()
  const [editingId, setEditingId] = React.useState<string | null>(null)
  const updateMut = useUpdateFermentable(editingId ?? '')

  const [showForm, setShowForm] = React.useState(false)
  const [form, setForm] = React.useState<FermentableRequest>(blank())
  const [formError, setFormError] = React.useState<string | null>(null)

  const openCreate = () => {
    setEditingId(null)
    setForm(blank())
    setFormError(null)
    setShowForm(true)
  }

  const openEdit = (row: Fermentable) => {
    setEditingId(row.id)
    setForm(toRequest(row))
    setFormError(null)
    setShowForm(true)
  }

  const closeForm = () => {
    setShowForm(false)
    setEditingId(null)
  }

  const setField = (key: keyof FermentableRequest, value: string | number | undefined) => {
    setForm((p) => ({ ...p, [key]: value }))
  }

  const handleSave = async () => {
    setFormError(null)
    try {
      if (editingId) {
        await updateMut.mutateAsync(form as Partial<Fermentable>)
      } else {
        await createMut.mutateAsync(form as Partial<Fermentable>)
      }
      closeForm()
    } catch (e) {
      setFormError(e instanceof APIError ? e.message : 'Save failed')
    }
  }

  const handleDelete = async (id: string) => {
    if (!window.confirm('Delete this fermentable?')) return
    try {
      await deleteMut.mutateAsync(id)
    } catch (e) {
      alert(e instanceof APIError ? e.message : 'Delete failed')
    }
  }

  const isSaving = createMut.isPending || updateMut.isPending
  const items = data?.items ?? []

  const inputClass =
    'w-full p-2 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)] text-[var(--color-fg)]'
  const labelClass = 'block text-xs text-[var(--color-muted)] mb-1'

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <h1 className="text-xl font-bold text-[var(--color-fg)]">Fermentables</h1>
        <button
          onClick={openCreate}
          className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90"
        >
          + New
        </button>
      </div>

      <div className="flex gap-2 mb-4">
        <input
          type="search"
          placeholder="Search by name…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="px-3 py-1.5 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)] text-[var(--color-fg)] w-56"
        />
        <select
          value={typeFilter}
          onChange={(e) => setTypeFilter(e.target.value)}
          className="px-3 py-1.5 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)] text-[var(--color-fg)]"
        >
          <option value="">All types</option>
          {['Base Malt', 'Heritage Malt', 'Kilned', 'Roasted', 'Crystal', 'Specialty', 'Adjunct', 'Distilling'].map(
            (t) => (
              <option key={t} value={t}>
                {t}
              </option>
            ),
          )}
        </select>
      </div>

      {showForm && (
        <div
          className="mb-6 p-4 rounded border"
          style={{ background: 'var(--color-surface)', borderColor: 'var(--color-border)' }}
        >
          <h2 className="font-semibold mb-3 text-sm text-[var(--color-muted)]">
            {editingId ? 'Edit' : 'New'} Fermentable
          </h2>
          {formError && (
            <div className="mb-3 p-2 rounded text-sm text-[var(--color-danger)] border border-[var(--color-danger)]">
              {formError}
            </div>
          )}
          <div className="grid grid-cols-2 gap-3">
            <div className="col-span-2 grid grid-cols-3 gap-3">
              <div>
                <label className={labelClass}>Name *</label>
                <input
                  type="text"
                  value={form.name ?? ''}
                  onChange={(e) => setField('name', e.target.value)}
                  className={inputClass}
                />
              </div>
              <div>
                <label className={labelClass}>Supplier</label>
                <input
                  type="text"
                  value={form.supplier ?? ''}
                  onChange={(e) => setField('supplier', e.target.value)}
                  className={inputClass}
                />
              </div>
              <div>
                <label className={labelClass}>Type</label>
                <select
                  value={form.type ?? ''}
                  onChange={(e) => setField('type', e.target.value)}
                  className={inputClass}
                >
                  <option value="">— select —</option>
                  {[
                    'Base Malt',
                    'Heritage Malt',
                    'Kilned',
                    'Roasted',
                    'Crystal',
                    'Specialty',
                    'Adjunct',
                    'Distilling',
                  ].map((t) => (
                    <option key={t} value={t}>
                      {t}
                    </option>
                  ))}
                </select>
              </div>
            </div>
            <div>
              <label className={labelClass}>EBC min</label>
              <input
                type="number"
                step="0.1"
                value={form.colour_ebc_min ?? ''}
                onChange={(e) => setField('colour_ebc_min', num(e.target.value))}
                className={inputClass}
              />
            </div>
            <div>
              <label className={labelClass}>EBC max</label>
              <input
                type="number"
                step="0.1"
                value={form.colour_ebc_max ?? ''}
                onChange={(e) => setField('colour_ebc_max', num(e.target.value))}
                className={inputClass}
              />
            </div>
            <div>
              <label className={labelClass}>Extract (L°/kg)</label>
              <input
                type="number"
                step="0.1"
                value={form.extract_litres_per_kg ?? ''}
                onChange={(e) => setField('extract_litres_per_kg', num(e.target.value))}
                className={inputClass}
              />
            </div>
            <div>
              <label className={labelClass}>Moisture % max</label>
              <input
                type="number"
                step="0.1"
                value={form.moisture_pct_max ?? ''}
                onChange={(e) => setField('moisture_pct_max', num(e.target.value))}
                className={inputClass}
              />
            </div>
            <div>
              <label className={labelClass}>TN% min</label>
              <input
                type="number"
                step="0.001"
                value={form.tn_min ?? ''}
                onChange={(e) => setField('tn_min', num(e.target.value))}
                className={inputClass}
              />
            </div>
            <div>
              <label className={labelClass}>TN% max</label>
              <input
                type="number"
                step="0.001"
                value={form.tn_max ?? ''}
                onChange={(e) => setField('tn_max', num(e.target.value))}
                className={inputClass}
              />
            </div>
            <div>
              <label className={labelClass}>SNR min</label>
              <input
                type="number"
                step="0.1"
                value={form.snr_min ?? ''}
                onChange={(e) => setField('snr_min', num(e.target.value))}
                className={inputClass}
              />
            </div>
            <div>
              <label className={labelClass}>SNR max</label>
              <input
                type="number"
                step="0.1"
                value={form.snr_max ?? ''}
                onChange={(e) => setField('snr_max', num(e.target.value))}
                className={inputClass}
              />
            </div>
            <div className="col-span-2">
              <label className={labelClass}>Flavour attributes</label>
              <input
                type="text"
                value={form.attributes ?? ''}
                onChange={(e) => setField('attributes', e.target.value)}
                className={inputClass}
              />
            </div>
            <div className="col-span-2">
              <label className={labelClass}>Notes</label>
              <textarea
                value={form.notes ?? ''}
                onChange={(e) => setField('notes', e.target.value)}
                rows={2}
                className={inputClass}
              />
            </div>
          </div>
          <div className="flex gap-2 mt-4">
            <button
              onClick={handleSave}
              disabled={isSaving || !form.name}
              className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white disabled:opacity-50"
            >
              {isSaving ? 'Saving…' : 'Save'}
            </button>
            <button
              onClick={closeForm}
              className="px-4 py-2 rounded text-sm border border-[var(--color-border)] text-[var(--color-fg)]"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {isLoading && (
        <div className="space-y-2">
          {Array.from({ length: 6 }).map((_, i) => (
            <div key={i} className="h-10 rounded animate-pulse" style={{ background: 'var(--color-border)' }} />
          ))}
        </div>
      )}

      {isError && (
        <div className="p-4 rounded border border-[var(--color-danger)] text-[var(--color-danger)]">
          {error instanceof Error ? error.message : 'Failed to load'}
          <button onClick={() => refetch()} className="ml-3 underline text-sm">
            Retry
          </button>
        </div>
      )}

      {!isLoading && !isError && (
        <div className="overflow-x-auto">
          <table className="w-full text-sm whitespace-nowrap">
            <thead>
              <tr className="border-b" style={{ borderColor: 'var(--color-border)' }}>
                {[
                  { label: 'Name', sortKey: 'name' },
                  { label: 'Supplier', sortKey: 'supplier' },
                  { label: 'Type', sortKey: 'type' },
                  { label: 'EBC' },
                  { label: 'Extract L°/kg' },
                  { label: 'Moisture %' },
                  { label: 'TN%' },
                  { label: 'SNR' },
                  { label: 'Flavour' },
                ].map((h) =>
                  h.sortKey ? (
                    <SortableHeader
                      key={h.label}
                      column={h.sortKey}
                      label={h.label}
                      sort={sort}
                      onSort={setSort}
                      className="py-2 px-3"
                    />
                  ) : (
                    <th key={h.label} className="text-left py-2 px-3 text-xs font-medium text-[var(--color-muted)] uppercase">
                      {h.label}
                    </th>
                  ),
                )}
                <th className="py-2 px-3" />
              </tr>
            </thead>
            <tbody>
              {items.length === 0 ? (
                <tr>
                  <td colSpan={10} className="py-8 text-center text-[var(--color-muted)]">
                    No fermentables found.
                  </td>
                </tr>
              ) : (
                items.map((row) => {
                  const isSystem = row.tenant_id === SYSTEM_TENANT
                  return (
                    <tr
                      key={row.id}
                      className="border-b hover:bg-[var(--color-border)]"
                      style={{ borderColor: 'var(--color-border)' }}
                    >
                      <td className="py-2 px-3 font-medium text-[var(--color-fg)]">{row.name}</td>
                      <td className="py-2 px-3 text-[var(--color-fg)]">{row.supplier ?? '—'}</td>
                      <td className="py-2 px-3 text-[var(--color-fg)]">{row.type ?? '—'}</td>
                      <td className="py-2 px-3 text-[var(--color-fg)]">
                        {fmt(row.colour_ebc_min, row.colour_ebc_max)}
                      </td>
                      <td className="py-2 px-3 text-[var(--color-fg)]">
                        {row.extract_litres_per_kg != null ? row.extract_litres_per_kg : '—'}
                      </td>
                      <td className="py-2 px-3 text-[var(--color-fg)]">
                        {row.moisture_pct_max != null ? `≤${row.moisture_pct_max}` : '—'}
                      </td>
                      <td className="py-2 px-3 text-[var(--color-fg)]">{fmt(row.tn_min, row.tn_max, '%')}</td>
                      <td className="py-2 px-3 text-[var(--color-fg)]">{fmt(row.snr_min, row.snr_max)}</td>
                      <td className="py-2 px-3 text-[var(--color-fg)] max-w-[180px] truncate">
                        {row.attributes ?? '—'}
                      </td>
                      <td className="py-2 px-3">
                        {!isSystem && (
                          <div className="flex gap-2 justify-end">
                            <button
                              onClick={() => openEdit(row)}
                              className="text-xs px-2 py-1 rounded border border-[var(--color-border)] text-[var(--color-fg)] hover:bg-[var(--color-accent)] hover:text-white"
                            >
                              Edit
                            </button>
                            <button
                              onClick={() => handleDelete(row.id)}
                              className="text-xs px-2 py-1 rounded border border-[var(--color-danger)] text-[var(--color-danger)] hover:bg-[var(--color-danger)] hover:text-white"
                            >
                              Delete
                            </button>
                          </div>
                        )}
                      </td>
                    </tr>
                  )
                })
              )}
            </tbody>
          </table>
          {data && data.total > items.length && (
            <p className="mt-3 text-xs text-[var(--color-muted)]">
              Showing {items.length} of {data.total}
            </p>
          )}
        </div>
      )}
    </div>
  )
}
