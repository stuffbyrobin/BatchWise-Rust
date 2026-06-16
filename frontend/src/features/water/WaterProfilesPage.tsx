import React from 'react'
import { APIError } from '../../api/error'
import {
  useWaterProfiles,
  useCreateWaterProfile,
  useUpdateWaterProfile,
  useDeleteWaterProfile,
} from './hooks/useWater'
import type { components } from '../../api/generated'

type WaterProfile = components['schemas']['WaterProfile']

const ION_FIELDS: { key: keyof WaterProfile; label: string }[] = [
  { key: 'calcium_ppm', label: 'Ca²⁺' },
  { key: 'magnesium_ppm', label: 'Mg²⁺' },
  { key: 'sodium_ppm', label: 'Na⁺' },
  { key: 'sulfate_ppm', label: 'SO₄²⁻' },
  { key: 'chloride_ppm', label: 'Cl⁻' },
  { key: 'bicarbonate_ppm', label: 'HCO₃⁻' },
]

const blank = () => ({
  name: '',
  description: '',
  calcium_ppm: '',
  magnesium_ppm: '',
  sodium_ppm: '',
  sulfate_ppm: '',
  chloride_ppm: '',
  bicarbonate_ppm: '',
  notes: '',
})

type FormState = ReturnType<typeof blank>

function profileToForm(p: WaterProfile): FormState {
  return {
    name: p.name ?? '',
    description: p.description ?? '',
    calcium_ppm: String(p.calcium_ppm ?? 0),
    magnesium_ppm: String(p.magnesium_ppm ?? 0),
    sodium_ppm: String(p.sodium_ppm ?? 0),
    sulfate_ppm: String(p.sulfate_ppm ?? 0),
    chloride_ppm: String(p.chloride_ppm ?? 0),
    bicarbonate_ppm: String(p.bicarbonate_ppm ?? 0),
    notes: p.notes ?? '',
  }
}

function IonGrid({
  form,
  onChange,
  readOnly,
}: {
  form: FormState
  onChange: (k: string, v: string) => void
  readOnly?: boolean
}) {
  return (
    <div className="grid grid-cols-3 gap-3">
      {ION_FIELDS.map((f) => (
        <div key={f.key}>
          <label className="block text-xs text-[var(--color-muted)] mb-1">{f.label} (ppm)</label>
          <input
            type="number"
            min="0"
            step="0.1"
            value={form[f.key as keyof FormState]}
            onChange={(e) => onChange(f.key, e.target.value)}
            disabled={readOnly}
            className="w-full p-2 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)] disabled:opacity-50"
          />
        </div>
      ))}
    </div>
  )
}

export function WaterProfilesPage() {
  const { data, isLoading, isError, error, refetch } = useWaterProfiles({ page_size: 100, sort: 'name' })
  const createMut = useCreateWaterProfile()
  const deleteMut = useDeleteWaterProfile()

  const [editingId, setEditingId] = React.useState<string | null>(null)
  const updateMut = useUpdateWaterProfile(editingId ?? '')

  const [showForm, setShowForm] = React.useState(false)
  const [form, setForm] = React.useState<FormState>(blank())
  const [formError, setFormError] = React.useState<string | null>(null)

  const set = (k: string, v: string) => setForm((p) => ({ ...p, [k]: v }))

  const openCreate = () => {
    setEditingId(null)
    setForm(blank())
    setFormError(null)
    setShowForm(true)
  }

  const openEdit = (p: WaterProfile) => {
    setEditingId(p.id ?? null)
    setForm(profileToForm(p))
    setFormError(null)
    setShowForm(true)
  }

  const handleSave = async () => {
    setFormError(null)
    const body = {
      name: form.name,
      description: form.description || undefined,
      calcium_ppm: form.calcium_ppm === '' ? 0 : Number(form.calcium_ppm),
      magnesium_ppm: form.magnesium_ppm === '' ? 0 : Number(form.magnesium_ppm),
      sodium_ppm: form.sodium_ppm === '' ? 0 : Number(form.sodium_ppm),
      sulfate_ppm: form.sulfate_ppm === '' ? 0 : Number(form.sulfate_ppm),
      chloride_ppm: form.chloride_ppm === '' ? 0 : Number(form.chloride_ppm),
      bicarbonate_ppm: form.bicarbonate_ppm === '' ? 0 : Number(form.bicarbonate_ppm),
      notes: form.notes || undefined,
    }
    try {
      if (editingId) {
        await updateMut.mutateAsync(body as Partial<WaterProfile>)
      } else {
        await createMut.mutateAsync(body as Partial<WaterProfile>)
      }
      setShowForm(false)
      setEditingId(null)
    } catch (e) {
      setFormError(e instanceof APIError ? e.message : 'Save failed')
    }
  }

  const handleDelete = async (id: string) => {
    if (!window.confirm('Delete this profile?')) return
    try {
      await deleteMut.mutateAsync(id)
    } catch (e) {
      alert(e instanceof APIError ? e.message : 'Delete failed')
    }
  }

  const isSaving = createMut.isPending || updateMut.isPending
  const profiles = data?.items ?? []
  const systemProfiles = profiles.filter((p) => p.is_system)
  const tenantProfiles = profiles.filter((p) => !p.is_system)

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <h1 className="text-xl font-bold text-[var(--color-fg)]">Water Profiles</h1>
        <button
          onClick={openCreate}
          className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90"
        >
          + New Profile
        </button>
      </div>

      {showForm && (
        <div
          className="mb-6 p-4 rounded border"
          style={{ background: 'var(--color-surface)', borderColor: 'var(--color-border)' }}
        >
          <h2 className="font-semibold mb-3 text-sm text-[var(--color-muted)]">
            {editingId ? 'Edit Profile' : 'New Profile'}
          </h2>
          {formError && (
            <div className="mb-3 p-2 rounded text-sm text-[var(--color-danger)] border border-[var(--color-danger)]">
              {formError}
            </div>
          )}
          <div className="mb-3">
            <label className="block text-xs text-[var(--color-muted)] mb-1">
              Name <span className="text-[var(--color-danger)]">*</span>
            </label>
            <input
              type="text"
              value={form.name}
              onChange={(e) => set('name', e.target.value)}
              className="w-full p-2 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)]"
            />
          </div>
          <div className="mb-3">
            <label className="block text-xs text-[var(--color-muted)] mb-1">Description</label>
            <input
              type="text"
              value={form.description}
              onChange={(e) => set('description', e.target.value)}
              className="w-full p-2 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)]"
            />
          </div>
          <p className="text-xs text-[var(--color-muted)] mb-2 font-medium">Ion profile (ppm)</p>
          <IonGrid form={form} onChange={set} />
          <div className="mt-3">
            <label className="block text-xs text-[var(--color-muted)] mb-1">Notes</label>
            <textarea
              value={form.notes}
              onChange={(e) => set('notes', e.target.value)}
              rows={2}
              className="w-full p-2 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)]"
            />
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
          {Array.from({ length: 5 }).map((_, i) => (
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
        <>
          {tenantProfiles.length > 0 && (
            <ProfileTable
              profiles={tenantProfiles}
              onEdit={openEdit}
              onDelete={handleDelete}
            />
          )}
          {tenantProfiles.length === 0 && !showForm && (
            <p className="text-sm text-[var(--color-muted)] mb-6">
              No custom profiles yet. Click + New Profile to create one.
            </p>
          )}

          {systemProfiles.length > 0 && (
            <>
              <p className="text-xs font-semibold uppercase tracking-wider text-[var(--color-muted)] mt-6 mb-2">
                System Profiles (read-only)
              </p>
              <ProfileTable profiles={systemProfiles} />
            </>
          )}
        </>
      )}
    </div>
  )
}

function ProfileTable({
  profiles,
  onEdit,
  onDelete,
}: {
  profiles: WaterProfile[]
  onEdit?: (p: WaterProfile) => void
  onDelete?: (id: string) => void
}) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b" style={{ borderColor: 'var(--color-border)' }}>
            <th className="text-left py-2 px-3 text-[var(--color-muted)] font-medium">Name</th>
            {ION_FIELDS.map((f) => (
              <th key={f.key} className="text-right py-2 px-3 text-[var(--color-muted)] font-medium">
                {f.label}
              </th>
            ))}
            {onEdit && <th className="py-2 px-3" />}
          </tr>
        </thead>
        <tbody>
          {profiles.map((p) => (
            <tr
              key={p.id}
              className="border-b hover:bg-[var(--color-border)]"
              style={{ borderColor: 'var(--color-border)', opacity: p.is_system ? 0.75 : 1 }}
            >
              <td className="py-2 px-3 text-[var(--color-fg)]">
                {p.name}
                {p.is_system && (
                  <span className="ml-2 text-xs text-[var(--color-muted)] border border-[var(--color-border)] rounded px-1">
                    system
                  </span>
                )}
              </td>
              {ION_FIELDS.map((f) => (
                <td key={f.key} className="py-2 px-3 text-right text-[var(--color-fg)] tabular-nums">
                  {p[f.key] != null ? Number(p[f.key]).toFixed(1) : '—'}
                </td>
              ))}
              {onEdit && (
                <td className="py-2 px-3">
                  {!p.is_system && (
                    <div className="flex gap-2 justify-end">
                      <button
                        onClick={() => onEdit(p)}
                        className="text-xs px-2 py-1 rounded border border-[var(--color-border)] text-[var(--color-fg)] hover:bg-[var(--color-accent)] hover:text-white"
                      >
                        Edit
                      </button>
                      <button
                        onClick={() => onDelete?.(p.id!)}
                        className="text-xs px-2 py-1 rounded border border-[var(--color-danger)] text-[var(--color-danger)] hover:bg-[var(--color-danger)] hover:text-white"
                      >
                        Delete
                      </button>
                    </div>
                  )}
                </td>
              )}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
