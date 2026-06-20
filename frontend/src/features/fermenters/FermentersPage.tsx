import React from 'react'
import { Link } from 'react-router-dom'
import { APIError } from '../../api/error'
import { useFermenters, useCreateFermenter, useDeleteFermenter } from './hooks/useFermenters'

export default function FermentersPage() {
  const { data, isLoading, error } = useFermenters({ sort: 'name', page_size: 100 })
  const createMut = useCreateFermenter()
  const deleteMut = useDeleteFermenter()

  const [name, setName] = React.useState('')
  const [capacity, setCapacity] = React.useState('')
  const [notes, setNotes] = React.useState('')
  const [formErr, setFormErr] = React.useState<string | null>(null)

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault()
    setFormErr(null)
    try {
      await createMut.mutateAsync({
        name: name.trim(),
        capacity_liters: capacity === '' ? null : Number(capacity),
        notes: notes.trim() || null,
      })
      setName(''); setCapacity(''); setNotes('')
    } catch (err) {
      setFormErr(err instanceof APIError ? (typeof err.details?.reason === 'string' ? err.details.reason : err.message) : 'Failed to create fermenter')
    }
  }

  async function handleDelete(id: string, fName: string) {
    if (!window.confirm(`Delete fermenter "${fName}"? Any assigned batches will be unassigned.`)) return
    try {
      await deleteMut.mutateAsync(id)
    } catch (err) {
      alert(err instanceof APIError ? err.message : 'Delete failed')
    }
  }

  const fermenters = data?.items ?? []

  return (
    <div className="max-w-4xl">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-bold text-[var(--color-fg)]">Fermenters</h1>
        <Link
          to="/fermenters/schedule"
          className="px-4 py-2 rounded text-sm border border-[var(--color-border)] text-[var(--color-fg)] hover:bg-[var(--color-border)]"
        >
          Schedule (Gantt)
        </Link>
      </div>

      {/* Create form */}
      <form
        onSubmit={handleCreate}
        className="mb-6 p-4 rounded border border-[var(--color-border)] bg-[var(--color-surface)] flex flex-wrap items-end gap-3"
      >
        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Name *</label>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
            placeholder="FV1"
            className="px-3 py-1.5 rounded border border-[var(--color-border)] bg-[var(--color-bg)] text-[var(--color-fg)] text-sm w-40"
          />
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Capacity (L)</label>
          <input
            type="number"
            min="0"
            step="any"
            value={capacity}
            onChange={(e) => setCapacity(e.target.value)}
            placeholder="1000"
            className="px-3 py-1.5 rounded border border-[var(--color-border)] bg-[var(--color-bg)] text-[var(--color-fg)] text-sm w-32"
          />
        </div>
        <div className="flex flex-col gap-1 flex-1 min-w-48">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Notes</label>
          <input
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            className="px-3 py-1.5 rounded border border-[var(--color-border)] bg-[var(--color-bg)] text-[var(--color-fg)] text-sm w-full"
          />
        </div>
        <button
          type="submit"
          disabled={createMut.isPending || !name.trim()}
          className="px-4 py-1.5 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
        >
          {createMut.isPending ? 'Adding…' : 'Add fermenter'}
        </button>
      </form>
      {formErr && <div className="mb-4 text-sm text-[var(--color-danger)]">{formErr}</div>}

      {/* List */}
      {isLoading && <p className="text-[var(--color-muted)]">Loading…</p>}
      {error && <p className="text-[var(--color-danger)]">Failed to load fermenters.</p>}
      {!isLoading && !error && (
        <div className="overflow-x-auto border rounded-lg" style={{ borderColor: 'var(--color-border)' }}>
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs uppercase text-[var(--color-muted)] border-b" style={{ borderColor: 'var(--color-border)' }}>
                <th className="p-3 font-medium">Name</th>
                <th className="p-3 font-medium">Capacity (L)</th>
                <th className="p-3 font-medium">Notes</th>
                <th className="p-3"></th>
              </tr>
            </thead>
            <tbody>
              {fermenters.length === 0 && (
                <tr><td colSpan={4} className="p-8 text-center text-[var(--color-muted)]">No fermenters yet. Add one above.</td></tr>
              )}
              {fermenters.map((f) => (
                <tr key={f.id} className="border-t" style={{ borderColor: 'var(--color-border)' }}>
                  <td className="p-3 font-medium text-[var(--color-fg)]">{f.name}</td>
                  <td className="p-3 text-[var(--color-fg)]">{f.capacity_liters ?? '—'}</td>
                  <td className="p-3 text-[var(--color-muted)]">{f.notes ?? ''}</td>
                  <td className="p-3 text-right">
                    <button
                      onClick={() => handleDelete(f.id, f.name)}
                      className="text-xs px-2 py-1 rounded border border-[var(--color-danger)] text-[var(--color-danger)] hover:bg-[var(--color-danger)] hover:text-white"
                    >
                      Delete
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
