import React from 'react'
import { APIError } from '../../api/error'
import {
  useWaterAdjustments,
  useWaterProfiles,
  useCreateWaterAdjustment,
  useUpdateWaterAdjustment,
  useDeleteWaterAdjustment,
} from './hooks/useWater'
import type { components } from '../../api/generated'

type WaterAdjustment = components['schemas']['WaterAdjustment']
type WaterResult = components['schemas']['WaterResult']
type MineralAddition = { type: string; amount: number; form?: string; strength_pct?: number }

const MINERALS = [
  { type: 'CaSO4', label: 'Gypsum (CaSO₄)' },
  { type: 'CaCl2', label: 'Calcium Chloride (CaCl₂)' },
  { type: 'MgSO4', label: 'Epsom Salt (MgSO₄)' },
  { type: 'NaCl', label: 'Table Salt (NaCl)' },
  { type: 'NaHCO3', label: 'Baking Soda (NaHCO₃)' },
  { type: 'CaCO3', label: 'Chalk (CaCO₃)' },
  { type: 'Ca(OH)2', label: 'Slaked Lime' },
  { type: 'MgCl2', label: 'Magnesium Chloride (MgCl₂)' },
]

interface MineralRow {
  id: number
  type: string
  amount: string
  /** CaCl₂ only: 'anhydrous' | 'dihydrate' | 'liquid'. */
  form?: string
  /** CaCl₂ liquid only: solution strength %w/w. */
  strength?: string
}

// CaCl₂ carries form/strength; other salts send just type + amount.
function mineralPayload(m: MineralRow): MineralAddition {
  const out: MineralAddition = { type: m.type, amount: Number(m.amount) }
  if (m.type === 'CaCl2') {
    const form = m.form || 'dihydrate'
    out.form = form
    if (form === 'liquid') out.strength_pct = Number(m.strength) || 0
  }
  return out
}

const blankForm = () => ({
  name: '',
  source_profile_id: '',
  volume_liters: '20',
  notes: '',
})

type FormState = ReturnType<typeof blankForm>

function adjToForm(adj: WaterAdjustment): FormState {
  return {
    name: adj.name ?? '',
    source_profile_id: adj.source_profile_id ?? '',
    volume_liters: String(adj.volume_liters ?? 20),
    notes: adj.notes ?? '',
  }
}

function adjToMinerals(adj: WaterAdjustment, startId: number): { rows: MineralRow[]; nextId: number } {
  const additions = (adj.mineral_additions ?? []) as MineralAddition[]
  const rows = additions.map((m, i) => ({
    id: startId + i,
    type: m.type,
    amount: String(m.amount),
    form: m.form ?? undefined,
    strength: m.strength_pct != null ? String(m.strength_pct) : undefined,
  }))
  return { rows, nextId: startId + additions.length }
}

function ResultBadge({ result }: { result: WaterResult | undefined }) {
  if (!result) return <span className="text-[var(--color-muted)]">—</span>
  const sc = result.sulfate_to_chloride != null ? Number(result.sulfate_to_chloride).toFixed(1) : '—'
  const ph = result.mash_ph && result.mash_ph > 0 ? Number(result.mash_ph).toFixed(2) : null
  return (
    <span className="text-xs tabular-nums text-[var(--color-fg)]">
      SO₄/Cl {sc}
      {ph && <span className="ml-2">· pH {ph}</span>}
    </span>
  )
}

export function WaterAdjustmentsPage() {
  const { data, isLoading, isError, error, refetch } = useWaterAdjustments({
    page_size: 100,
    sort: '-created_at',
  })
  const { data: profilesData } = useWaterProfiles({ page_size: 100, sort: 'name' })
  const createMut = useCreateWaterAdjustment()
  const deleteMut = useDeleteWaterAdjustment()

  const [editingId, setEditingId] = React.useState<string | null>(null)
  const updateMut = useUpdateWaterAdjustment(editingId ?? '')

  const [showForm, setShowForm] = React.useState(false)
  const [form, setForm] = React.useState<FormState>(blankForm())
  const [minerals, setMinerals] = React.useState<MineralRow[]>([])
  const [nextId, setNextId] = React.useState(1)
  const [expanded, setExpanded] = React.useState<string | null>(null)
  const [formError, setFormError] = React.useState<string | null>(null)

  const set = (k: string, v: string) => setForm((p) => ({ ...p, [k]: v }))

  const addMineral = () => {
    setMinerals((prev) => [...prev, { id: nextId, type: MINERALS[0].type, amount: '' }])
    setNextId((n) => n + 1)
  }
  const updateMineral = (id: number, key: 'type' | 'amount' | 'form' | 'strength', value: string) =>
    setMinerals((prev) => prev.map((m) => (m.id === id ? { ...m, [key]: value } : m)))
  const removeMineral = (id: number) =>
    setMinerals((prev) => prev.filter((m) => m.id !== id))

  const openCreate = () => {
    setEditingId(null)
    setForm(blankForm())
    setMinerals([])
    setFormError(null)
    setShowForm(true)
  }

  const openEdit = (adj: WaterAdjustment) => {
    setEditingId(adj.id ?? null)
    setForm(adjToForm(adj))
    const { rows, nextId: nid } = adjToMinerals(adj, nextId)
    setMinerals(rows)
    setNextId(nid)
    setFormError(null)
    setShowForm(true)
  }

  const buildBody = () => ({
    name: form.name,
    source_profile_id: form.source_profile_id || undefined,
    volume_liters: Number(form.volume_liters) || 0,
    notes: form.notes || undefined,
    mineral_additions: minerals
      .filter((m) => m.amount !== '' && Number(m.amount) > 0)
      .map(mineralPayload),
  })

  const handleSave = async () => {
    setFormError(null)
    try {
      if (editingId) {
        await updateMut.mutateAsync(buildBody())
      } else {
        await createMut.mutateAsync(buildBody())
      }
      setShowForm(false)
      setEditingId(null)
    } catch (e) {
      setFormError(e instanceof APIError ? e.message : 'Save failed')
    }
  }

  const handleDelete = async (id: string) => {
    if (!window.confirm('Delete this adjustment?')) return
    try {
      await deleteMut.mutateAsync(id)
      if (expanded === id) setExpanded(null)
    } catch (e) {
      alert(e instanceof APIError ? e.message : 'Delete failed')
    }
  }

  const isSaving = createMut.isPending || updateMut.isPending
  const profiles = profilesData?.items ?? []
  const adjustments = data?.items ?? []

  const inputCls = 'w-full p-2 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)]'

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <h1 className="text-xl font-bold text-[var(--color-fg)]">Water Adjustments</h1>
        <button
          onClick={openCreate}
          className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90"
        >
          + New Adjustment
        </button>
      </div>

      {showForm && (
        <div
          className="mb-6 p-4 rounded border"
          style={{ background: 'var(--color-surface)', borderColor: 'var(--color-border)' }}
        >
          <h2 className="font-semibold mb-3 text-sm text-[var(--color-muted)]">
            {editingId ? 'Edit Adjustment' : 'New Adjustment'}
          </h2>
          {formError && (
            <div className="mb-3 p-2 rounded text-sm text-[var(--color-danger)] border border-[var(--color-danger)]">
              {formError}
            </div>
          )}

          <div className="grid grid-cols-2 gap-3 mb-3">
            <div>
              <label className="block text-xs text-[var(--color-muted)] mb-1">
                Name <span className="text-[var(--color-danger)]">*</span>
              </label>
              <input
                type="text"
                value={form.name}
                onChange={(e) => set('name', e.target.value)}
                className={inputCls}
              />
            </div>
            <div>
              <label className="block text-xs text-[var(--color-muted)] mb-1">
                Volume (litres) <span className="text-[var(--color-danger)]">*</span>
              </label>
              <input
                type="number"
                min="0"
                step="0.5"
                value={form.volume_liters}
                onChange={(e) => set('volume_liters', e.target.value)}
                className={inputCls}
              />
            </div>
          </div>

          <div className="mb-3">
            <label className="block text-xs text-[var(--color-muted)] mb-1">Source water profile</label>
            <select
              value={form.source_profile_id}
              onChange={(e) => set('source_profile_id', e.target.value)}
              className={inputCls}
            >
              <option value="">— distilled / RO water (no source) —</option>
              {profiles.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                  {p.is_system ? ' (system)' : ''}
                </option>
              ))}
            </select>
          </div>

          {/* Mineral additions */}
          <div className="mb-3">
            <div className="flex items-center justify-between mb-2">
              <p className="text-xs font-medium text-[var(--color-muted)]">Mineral additions</p>
              <button
                onClick={addMineral}
                className="text-xs px-2 py-1 rounded border border-[var(--color-accent)] text-[var(--color-accent)] hover:bg-[var(--color-accent)] hover:text-white"
              >
                + Add
              </button>
            </div>
            {minerals.length === 0 && (
              <p className="text-xs text-[var(--color-muted)]">No minerals added.</p>
            )}
            <div className="space-y-2">
              {minerals.map((m) => (
                <div key={m.id} className="flex gap-2 items-center">
                  <select
                    value={m.type}
                    onChange={(e) => updateMineral(m.id, 'type', e.target.value)}
                    className="flex-1 p-2 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)]"
                  >
                    {MINERALS.map((opt) => (
                      <option key={opt.type} value={opt.type}>
                        {opt.label}
                      </option>
                    ))}
                  </select>
                  <input
                    type="number"
                    min="0"
                    step="0.1"
                    placeholder="g"
                    value={m.amount}
                    onChange={(e) => updateMineral(m.id, 'amount', e.target.value)}
                    className="w-20 p-2 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)]"
                  />
                  <span className="text-xs text-[var(--color-muted)]">g</span>
                  {m.type === 'CaCl2' && (
                    <>
                      <select
                        value={m.form || 'dihydrate'}
                        onChange={(e) => updateMineral(m.id, 'form', e.target.value)}
                        title="CaCl₂ form"
                        className="p-2 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)]"
                      >
                        <option value="anhydrous">Anhydrous</option>
                        <option value="dihydrate">Dihydrate</option>
                        <option value="liquid">Liquid</option>
                      </select>
                      {(m.form || 'dihydrate') === 'liquid' && (
                        <>
                          <input
                            type="number"
                            min="0"
                            max="100"
                            step="1"
                            placeholder="%w/w"
                            value={m.strength ?? ''}
                            onChange={(e) => updateMineral(m.id, 'strength', e.target.value)}
                            className="w-16 p-2 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)]"
                          />
                          <span className="text-xs text-[var(--color-muted)]">%w/w</span>
                        </>
                      )}
                    </>
                  )}
                  <button
                    onClick={() => removeMineral(m.id)}
                    className="text-[var(--color-danger)] text-xs px-1"
                  >
                    ✕
                  </button>
                </div>
              ))}
            </div>
          </div>

          <div className="mb-4">
            <label className="block text-xs text-[var(--color-muted)] mb-1">Notes</label>
            <textarea
              value={form.notes}
              onChange={(e) => set('notes', e.target.value)}
              rows={2}
              className={inputCls}
            />
          </div>

          <div className="flex gap-2">
            <button
              onClick={handleSave}
              disabled={isSaving || !form.name || !form.volume_liters}
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
            <div key={i} className="h-12 rounded animate-pulse" style={{ background: 'var(--color-border)' }} />
          ))}
        </div>
      )}

      {isError && (
        <div className="p-4 rounded border border-[var(--color-danger)] text-[var(--color-danger)]">
          {error instanceof Error ? error.message : 'Failed to load'}
          <button onClick={() => refetch()} className="ml-3 underline text-sm">Retry</button>
        </div>
      )}

      {!isLoading && !isError && adjustments.length === 0 && !showForm && (
        <p className="text-sm text-[var(--color-muted)]">
          No adjustments yet. Click + New Adjustment to create one.
        </p>
      )}

      {!isLoading && !isError && adjustments.length > 0 && (
        <div className="space-y-2">
          {adjustments.map((adj) => (
            <AdjustmentRow
              key={adj.id}
              adj={adj}
              expanded={expanded === adj.id}
              onToggle={() => setExpanded((prev) => (prev === adj.id ? null : (adj.id ?? null)))}
              onEdit={() => openEdit(adj)}
              onDelete={() => handleDelete(adj.id!)}
            />
          ))}
        </div>
      )}
    </div>
  )
}

function AdjustmentRow({
  adj,
  expanded,
  onToggle,
  onEdit,
  onDelete,
}: {
  adj: WaterAdjustment
  expanded: boolean
  onToggle: () => void
  onEdit: () => void
  onDelete: () => void
}) {
  const minerals = (adj.mineral_additions ?? []) as MineralAddition[]

  return (
    <div
      className="rounded border"
      style={{ background: 'var(--color-surface)', borderColor: 'var(--color-border)' }}
    >
      <div
        className="flex items-center gap-3 px-4 py-3 cursor-pointer select-none"
        onClick={onToggle}
      >
        <span className="text-[var(--color-muted)] text-xs">{expanded ? '▾' : '▸'}</span>
        <span className="font-medium text-sm text-[var(--color-fg)] flex-1">{adj.name}</span>
        <span className="text-xs text-[var(--color-muted)] mr-2">{adj.volume_liters} L</span>
        <ResultBadge result={adj.result as WaterResult | undefined} />
        <div className="flex gap-2 ml-3" onClick={(e) => e.stopPropagation()}>
          <button
            onClick={onEdit}
            className="text-xs px-2 py-1 rounded border border-[var(--color-border)] text-[var(--color-fg)] hover:bg-[var(--color-accent)] hover:text-white"
          >
            Edit
          </button>
          <button
            onClick={onDelete}
            className="text-xs px-2 py-1 rounded border border-[var(--color-danger)] text-[var(--color-danger)] hover:bg-[var(--color-danger)] hover:text-white"
          >
            Delete
          </button>
        </div>
      </div>

      {expanded && (
        <div
          className="px-4 pb-4 border-t"
          style={{ borderColor: 'var(--color-border)' }}
        >
          <div className="grid grid-cols-2 gap-4 mt-3">
            {/* Minerals */}
            <div>
              <p className="text-xs font-medium text-[var(--color-muted)] mb-2">Mineral additions</p>
              {minerals.length === 0 ? (
                <p className="text-xs text-[var(--color-muted)]">None</p>
              ) : (
                <table className="text-xs w-full">
                  <tbody>
                    {minerals.map((m, i) => (
                      <tr key={i}>
                        <td className="py-0.5 text-[var(--color-fg)]">
                          {m.type}
                          {m.type === 'CaCl2' && m.form && m.form !== 'dihydrate'
                            ? ` (${m.form})`
                            : ''}
                        </td>
                        <td className="py-0.5 text-right tabular-nums text-[var(--color-fg)]">
                          {m.amount} g
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>

            {/* Result ions */}
            {adj.result && (
              <div>
                <p className="text-xs font-medium text-[var(--color-muted)] mb-2">Treated water</p>
                <IonResultGrid result={adj.result as WaterResult} />
              </div>
            )}
          </div>

          {adj.notes && (
            <p className="mt-3 text-xs text-[var(--color-muted)]">{adj.notes}</p>
          )}
        </div>
      )}
    </div>
  )
}

function IonResultGrid({ result }: { result: WaterResult }) {
  const ions: { key: keyof WaterResult; label: string }[] = [
    { key: 'calcium_ppm', label: 'Ca²⁺' },
    { key: 'magnesium_ppm', label: 'Mg²⁺' },
    { key: 'sodium_ppm', label: 'Na⁺' },
    { key: 'sulfate_ppm', label: 'SO₄²⁻' },
    { key: 'chloride_ppm', label: 'Cl⁻' },
    { key: 'bicarbonate_ppm', label: 'HCO₃⁻' },
  ]
  return (
    <table className="text-xs w-full">
      <tbody>
        {ions.map((f) => (
          <tr key={f.key}>
            <td className="py-0.5 text-[var(--color-muted)]">{f.label}</td>
            <td className="py-0.5 text-right tabular-nums text-[var(--color-fg)]">
              {result[f.key] != null ? `${Number(result[f.key]).toFixed(1)} ppm` : '—'}
            </td>
          </tr>
        ))}
        <tr>
          <td className="py-0.5 text-[var(--color-muted)]">SO₄ : Cl</td>
          <td className="py-0.5 text-right tabular-nums text-[var(--color-fg)]">
            {result.sulfate_to_chloride != null
              ? Number(result.sulfate_to_chloride).toFixed(2)
              : '—'}
          </td>
        </tr>
        {result.mash_ph != null && result.mash_ph > 0 && (
          <tr>
            <td className="py-0.5 text-[var(--color-muted)]">Mash pH</td>
            <td className="py-0.5 text-right tabular-nums font-medium text-[var(--color-fg)]">
              {Number(result.mash_ph).toFixed(2)}
            </td>
          </tr>
        )}
      </tbody>
    </table>
  )
}
