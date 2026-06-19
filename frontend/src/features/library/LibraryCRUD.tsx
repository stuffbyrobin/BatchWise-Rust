import React from 'react'
import type { UseMutationResult, UseQueryResult } from '@tanstack/react-query'
import { APIError } from '../../api/error'
import { SortableHeader } from '../../components/ui/SortableHeader'

export interface FieldDef {
  key: string
  label: string
  type: 'text' | 'number' | 'textarea' | 'select'
  options?: string[]
  required?: boolean
  /** When true, the column header is clickable to sort server-side by `key`.
   *  Only set this for keys the endpoint's sort allow-list accepts. */
  sortable?: boolean
}

interface Props<T extends Record<string, unknown>> {
  title: string
  useList: (params: { sort?: string }) => UseQueryResult<{ items: T[]; total: number }>
  useCreate: () => UseMutationResult<T, Error, Partial<T>>
  useUpdate: (id: string) => UseMutationResult<T, Error, Partial<T>>
  useDelete: () => UseMutationResult<void, Error, string>
  fields: FieldDef[]
  idField?: string
  extraCols?: { key: string; label: string; render: (row: T) => React.ReactNode }[]
}

function blank(fields: FieldDef[]): Record<string, string> {
  return Object.fromEntries(fields.map((f) => [f.key, '']))
}

export function LibraryCRUD<T extends Record<string, unknown>>({
  title,
  useList,
  useCreate,
  useUpdate,
  useDelete,
  fields,
  idField = 'id',
  extraCols = [],
}: Props<T>) {
  const [sort, setSort] = React.useState('')
  const { data, isLoading, isError, error, refetch } = useList({ sort: sort || undefined })
  const createMut = useCreate()
  const [editingId, setEditingId] = React.useState<string | null>(null)
  const deleteMut = useDelete()
  const updateMut = useUpdate(editingId ?? '')

  const [form, setForm] = React.useState<Record<string, string>>(blank(fields))
  const [formError, setFormError] = React.useState<string | null>(null)
  const [showForm, setShowForm] = React.useState(false)

  const openCreate = () => {
    setEditingId(null)
    setForm(blank(fields))
    setFormError(null)
    setShowForm(true)
  }

  const openEdit = (row: T) => {
    setEditingId(String(row[idField]))
    const values = Object.fromEntries(
      fields.map((f) => [f.key, row[f.key] !== null && row[f.key] !== undefined ? String(row[f.key]) : '']),
    )
    setForm(values)
    setFormError(null)
    setShowForm(true)
  }

  const handleDelete = async (id: string) => {
    if (!window.confirm('Delete this item?')) return
    try {
      await deleteMut.mutateAsync(id)
    } catch (e) {
      alert(e instanceof APIError ? e.message : 'Delete failed')
    }
  }

  const handleSave = async () => {
    setFormError(null)
    const body = Object.fromEntries(
      fields.map((f) => {
        const v = form[f.key]
        if (f.type === 'number') return [f.key, v === '' ? undefined : Number(v)]
        return [f.key, v === '' ? undefined : v]
      }),
    ) as Partial<T>
    try {
      if (editingId) {
        await updateMut.mutateAsync(body)
      } else {
        await createMut.mutateAsync(body)
      }
      setShowForm(false)
      setForm(blank(fields))
      setEditingId(null)
    } catch (e) {
      setFormError(e instanceof APIError ? e.message : 'Save failed')
    }
  }

  const isSaving = createMut.isPending || updateMut.isPending

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <h1 className="text-xl font-bold text-[var(--color-fg)]">{title}</h1>
        <button
          onClick={openCreate}
          className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90"
        >
          + New
        </button>
      </div>

      {showForm && (
        <div
          className="mb-6 p-4 rounded border"
          style={{ background: 'var(--color-surface)', borderColor: 'var(--color-border)' }}
        >
          <h2 className="font-semibold mb-3 text-sm text-[var(--color-muted)]">
            {editingId ? 'Edit' : 'New'} {title.replace(/s$/, '')}
          </h2>
          {formError && (
            <div className="mb-3 p-2 rounded text-sm text-[var(--color-danger)] border border-[var(--color-danger)]">
              {formError}
            </div>
          )}
          <div className="grid grid-cols-2 gap-3">
            {fields.map((f) => (
              <div key={f.key} className={f.type === 'textarea' ? 'col-span-2' : ''}>
                <label className="block text-xs text-[var(--color-muted)] mb-1">
                  {f.label}
                  {f.required && <span className="text-[var(--color-danger)]"> *</span>}
                </label>
                {f.type === 'textarea' ? (
                  <textarea
                    value={form[f.key] ?? ''}
                    onChange={(e) => setForm((p) => ({ ...p, [f.key]: e.target.value }))}
                    rows={2}
                    className="w-full p-2 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)]"
                  />
                ) : f.type === 'select' ? (
                  <select
                    value={form[f.key] ?? ''}
                    onChange={(e) => setForm((p) => ({ ...p, [f.key]: e.target.value }))}
                    className="w-full p-2 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)]"
                  >
                    <option value="">— select —</option>
                    {f.options?.map((o) => (
                      <option key={o} value={o}>
                        {o}
                      </option>
                    ))}
                  </select>
                ) : (
                  <input
                    type={f.type === 'number' ? 'number' : 'text'}
                    value={form[f.key] ?? ''}
                    onChange={(e) => setForm((p) => ({ ...p, [f.key]: e.target.value }))}
                    className="w-full p-2 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)]"
                  />
                )}
              </div>
            ))}
          </div>
          <div className="flex gap-2 mt-4">
            <button
              onClick={handleSave}
              disabled={isSaving}
              className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white disabled:opacity-50"
            >
              {isSaving ? 'Saving…' : 'Save'}
            </button>
            <button
              onClick={() => setShowForm(false)}
              className="px-4 py-2 rounded text-sm border border-[var(--color-border)] text-[var(--color-fg)]"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {isLoading && (
        <div className="space-y-2">
          {Array.from({ length: 4 }).map((_, i) => (
            <div key={i} className="h-10 rounded animate-pulse" style={{ background: 'var(--color-border)' }} />
          ))}
        </div>
      )}

      {isError && (
        <div className="p-4 rounded border border-[var(--color-danger)] text-[var(--color-danger)]">
          {error instanceof Error ? error.message : 'Failed to load'}
          <button onClick={() => refetch()} className="ml-3 underline text-sm">Retry</button>
        </div>
      )}

      {!isLoading && !isError && (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b" style={{ borderColor: 'var(--color-border)' }}>
                {fields.slice(0, 4).map((f) =>
                  f.sortable ? (
                    <SortableHeader
                      key={f.key}
                      column={f.key}
                      label={f.label}
                      sort={sort}
                      onSort={setSort}
                      className="py-2 px-3"
                    />
                  ) : (
                    <th key={f.key} className="text-left py-2 px-3 text-xs font-medium text-[var(--color-muted)] uppercase">
                      {f.label}
                    </th>
                  ),
                )}
                {extraCols.map((c) => (
                  <th key={c.key} className="text-left py-2 px-3 text-xs font-medium text-[var(--color-muted)] uppercase">
                    {c.label}
                  </th>
                ))}
                <th className="py-2 px-3" />
              </tr>
            </thead>
            <tbody>
              {(data?.items ?? []).length === 0 ? (
                <tr>
                  <td
                    colSpan={fields.slice(0, 4).length + extraCols.length + 1}
                    className="py-8 text-center text-[var(--color-muted)]"
                  >
                    No {title.toLowerCase()} yet. Click + New to add one.
                  </td>
                </tr>
              ) : (
                (data?.items ?? []).map((row) => (
                  <tr
                    key={String(row[idField])}
                    className="border-b hover:bg-[var(--color-border)]"
                    style={{ borderColor: 'var(--color-border)' }}
                  >
                    {fields.slice(0, 4).map((f) => (
                      <td key={f.key} className="py-2 px-3 text-[var(--color-fg)]">
                        {row[f.key] !== null && row[f.key] !== undefined ? String(row[f.key]) : '—'}
                      </td>
                    ))}
                    {extraCols.map((c) => (
                      <td key={c.key} className="py-2 px-3">
                        {c.render(row)}
                      </td>
                    ))}
                    <td className="py-2 px-3">
                      <div className="flex gap-2 justify-end">
                        <button
                          onClick={() => openEdit(row)}
                          className="text-xs px-2 py-1 rounded border border-[var(--color-border)] text-[var(--color-fg)] hover:bg-[var(--color-accent)] hover:text-white"
                        >
                          Edit
                        </button>
                        <button
                          onClick={() => handleDelete(String(row[idField]))}
                          className="text-xs px-2 py-1 rounded border border-[var(--color-danger)] text-[var(--color-danger)] hover:bg-[var(--color-danger)] hover:text-white"
                        >
                          Delete
                        </button>
                      </div>
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
