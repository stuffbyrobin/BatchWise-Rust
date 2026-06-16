import React from 'react'
import { APIError } from '../../api/error'
import { useWaterProfiles, useWaterAdjustments, useCreateWaterAdjustment, useUpdateWaterAdjustment } from '../water/hooks/useWater'
import { useBrewingPhysics } from '../../lib/physics/useBrewingPhysics'
import type { WaterTreatmentResult } from '../../lib/physics'

type WaterResult = WaterTreatmentResult

const MINERALS = [
  { type: 'CaSO4', label: 'Gypsum (CaSO₄)', hint: 'Ca²⁺, SO₄²⁻' },
  { type: 'CaCl2', label: 'Calcium Chloride (CaCl₂)', hint: 'Ca²⁺, Cl⁻' },
  { type: 'MgSO4', label: 'Epsom Salt (MgSO₄)', hint: 'Mg²⁺, SO₄²⁻' },
  { type: 'NaCl', label: 'Table Salt (NaCl)', hint: 'Na⁺, Cl⁻' },
  { type: 'NaHCO3', label: 'Baking Soda (NaHCO₃)', hint: 'Na⁺, HCO₃⁻' },
  { type: 'CaCO3', label: 'Chalk (CaCO₃)', hint: 'Ca²⁺, HCO₃⁻' },
  { type: 'Ca(OH)2', label: 'Slaked Lime (Ca(OH)₂)', hint: 'Ca²⁺, alkalinity' },
  { type: 'MgCl2', label: 'Magnesium Chloride (MgCl₂)', hint: 'Mg²⁺, Cl⁻' },
]

const RESULT_IONS: { key: keyof WaterResult; label: string }[] = [
  { key: 'calcium_ppm', label: 'Ca²⁺' },
  { key: 'magnesium_ppm', label: 'Mg²⁺' },
  { key: 'sodium_ppm', label: 'Na⁺' },
  { key: 'sulfate_ppm', label: 'SO₄²⁻' },
  { key: 'chloride_ppm', label: 'Cl⁻' },
  { key: 'bicarbonate_ppm', label: 'HCO₃⁻' },
]

interface MineralRow {
  id: number
  type: string
  amount: string
}

type SourceMode = 'profile' | 'inline'

const blankInline = () => ({
  calcium_ppm: '',
  magnesium_ppm: '',
  sodium_ppm: '',
  sulfate_ppm: '',
  chloride_ppm: '',
  bicarbonate_ppm: '',
})

interface RecipeFermentable {
  amount: number
  unit: string
  color_ebc?: number
}

export function RecipeWaterChemistry({
  recipeId,
  fermentables = [],
}: {
  recipeId?: string
  fermentables?: RecipeFermentable[]
}) {
  const { data: profilesData } = useWaterProfiles({ page_size: 100, sort: 'name' })
  const { data: adjustmentsData } = useWaterAdjustments({ recipe_id: recipeId, page_size: 1, sort: '-created_at' })
  const createMut = useCreateWaterAdjustment()
  const physics = useBrewingPhysics()

  const [sourceMode, setSourceMode] = React.useState<SourceMode>('profile')
  const [profileId, setProfileId] = React.useState('')
  const [inline, setInline] = React.useState(blankInline())
  const [volumeLiters, setVolumeLiters] = React.useState('20')
  const [minerals, setMinerals] = React.useState<MineralRow[]>([])
  const [nextId, setNextId] = React.useState(1)
  const [name, setName] = React.useState('Recipe water')
  const [saveError, setSaveError] = React.useState<string | null>(null)
  const [saveOk, setSaveOk] = React.useState(false)
  const [existingId, setExistingId] = React.useState<string | null>(null)

  const updateMut = useUpdateWaterAdjustment(existingId || '')

  React.useEffect(() => {
    if (!recipeId) return
    if (adjustmentsData?.items && adjustmentsData.items.length > 0) {
      const adj = adjustmentsData.items[0]
      setName(adj.name || 'Recipe water')
      setSourceMode('profile')
      setProfileId(adj.source_profile_id || '')
      setVolumeLiters(String(adj.volume_liters || 20))
      setMinerals(
        adj.mineral_additions?.map((ma, idx) => ({
          id: idx + 1,
          type: ma.type ?? MINERALS[0].type,
          amount: ma.amount != null ? String(ma.amount) : '',
        })) ?? []
      )
      setNextId((adj.mineral_additions?.length ?? 0) + 2)
      setExistingId(adj.id ?? null)
    }
  }, [adjustmentsData, recipeId])

  const addMineral = () => {
    setMinerals((prev) => [...prev, { id: nextId, type: MINERALS[0].type, amount: '' }])
    setNextId((n) => n + 1)
  }

  const updateMineral = (id: number, key: 'type' | 'amount', value: string) => {
    setMinerals((prev) => prev.map((m) => (m.id === id ? { ...m, [key]: value } : m)))
  }

  const removeMineral = (id: number) => {
    setMinerals((prev) => prev.filter((m) => m.id !== id))
  }

  // Resolve the source ions client-side: from the selected saved profile, or
  // from the inline numeric fields. Returns null when no source is available.
  const source = React.useMemo(() => {
    if (sourceMode === 'profile') {
      if (!profileId) return null
      const p = profilesData?.items?.find((x) => x.id === profileId)
      if (!p) return null
      return {
        calcium_ppm: p.calcium_ppm ?? 0,
        magnesium_ppm: p.magnesium_ppm ?? 0,
        sodium_ppm: p.sodium_ppm ?? 0,
        sulfate_ppm: p.sulfate_ppm ?? 0,
        chloride_ppm: p.chloride_ppm ?? 0,
        bicarbonate_ppm: p.bicarbonate_ppm ?? 0,
      }
    }
    return {
      calcium_ppm: Number(inline.calcium_ppm) || 0,
      magnesium_ppm: Number(inline.magnesium_ppm) || 0,
      sodium_ppm: Number(inline.sodium_ppm) || 0,
      sulfate_ppm: Number(inline.sulfate_ppm) || 0,
      chloride_ppm: Number(inline.chloride_ppm) || 0,
      bicarbonate_ppm: Number(inline.bicarbonate_ppm) || 0,
    }
  }, [sourceMode, profileId, inline, profilesData])

  // Derive water-chemistry grains from the recipe grain bill. The recipe model
  // has no explicit water grain-type, so we classify by EBC colour (estimate).
  const grains = React.useMemo(
    () =>
      fermentables
        .filter((f) => f.color_ebc != null && f.amount > 0)
        .map((f) => {
          const ebc = f.color_ebc as number
          const grain_type = ebc < 20 ? 'base' : ebc < 300 ? 'crystal' : 'roast'
          return {
            grain_type,
            weight_kg: f.unit === 'g' ? f.amount / 1000 : f.amount,
            colour_lovibond: ebc / 2.65,
          }
        }),
    [fermentables]
  )

  // Live treated-water profile + mash pH, recomputed in-browser by the same Rust
  // physics the server runs. Null when physics isn't ready or no source is set.
  const liveResult: WaterResult | null = React.useMemo(() => {
    if (!physics.ready || !source) return null
    return physics.computeWaterTreatment({
      source,
      volume_liters: Number(volumeLiters) || 0,
      minerals: minerals
        .filter((m) => m.amount !== '' && Number(m.amount) > 0)
        .map((m) => ({ type: m.type, amount: Number(m.amount) })),
      grains,
    })
    // physics.computeWaterTreatment is a stable module singleton; gate on `ready`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [physics.ready, source, volumeLiters, minerals, grains])

  const handleSave = async () => {
    setSaveError(null)
    setSaveOk(false)

    const body: Record<string, unknown> = {
      name,
      volume_liters: Number(volumeLiters) || 0,
      source_profile_id: profileId,
      mineral_additions: minerals
        .filter((m) => m.amount !== '' && Number(m.amount) > 0)
        .map((m) => ({ type: m.type, amount: Number(m.amount) })),
      recipe_id: recipeId,
    }

    try {
      if (existingId) {
        await updateMut.mutateAsync(body)
      } else {
        const res = await createMut.mutateAsync(body)
        setExistingId((res as { id: string }).id)
      }
      setSaveOk(true)
    } catch (e) {
      setSaveError(e instanceof APIError ? e.message : 'Save failed')
    }
  }

  const sc = liveResult
    ? liveResult.sulfate_to_chloride != null
      ? Number(liveResult.sulfate_to_chloride).toFixed(2)
      : '—'
    : null

  const profiles = profilesData?.items ?? []

  const inputCls =
    'w-full p-2 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)]'
  const sectionCls = 'mb-5'
  const labelCls = 'block text-xs text-[var(--color-muted)] mb-1'

  return (
    <div className='bg-[var(--color-surface)] p-6 rounded shadow mb-4'>
      <h2 className='text-lg font-semibold text-[var(--color-fg)] mb-4'>Water Chemistry & pH</h2>

      <div className={sectionCls} style={{ maxWidth: 300 }}>
        <label className={labelCls}>Adjustment name</label>
        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          className={inputCls}
        />
      </div>

      <div className={sectionCls}>
        <p className="text-sm font-semibold text-[var(--color-fg)] mb-2">Source water</p>
        <div className="flex gap-3 mb-3">
          <label className="flex items-center gap-1.5 text-sm cursor-pointer">
            <input
              type="radio"
              checked={sourceMode === 'profile'}
              onChange={() => setSourceMode('profile')}
            />
            Saved profile
          </label>
          <label className="flex items-center gap-1.5 text-sm cursor-pointer">
            <input
              type="radio"
              checked={sourceMode === 'inline'}
              onChange={() => setSourceMode('inline')}
            />
            Enter manually
          </label>
        </div>

        {sourceMode === 'profile' ? (
          <select
            value={profileId}
            onChange={(e) => setProfileId(e.target.value)}
            className={inputCls}
          >
            <option value="">— select a water profile —</option>
            {profiles.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
                {p.is_system ? ' (system)' : ''}
              </option>
            ))}
          </select>
        ) : (
          <div className="grid grid-cols-3 gap-3">
            {[
              { key: 'calcium_ppm', label: 'Ca²⁺ (ppm)' },
              { key: 'magnesium_ppm', label: 'Mg²⁺ (ppm)' },
              { key: 'sodium_ppm', label: 'Na⁺ (ppm)' },
              { key: 'sulfate_ppm', label: 'SO₄²⁻ (ppm)' },
              { key: 'chloride_ppm', label: 'Cl⁻ (ppm)' },
              { key: 'bicarbonate_ppm', label: 'HCO₃⁻ (ppm)' },
            ].map((f) => (
              <div key={f.key}>
                <label className={labelCls}>{f.label}</label>
                <input
                  type="number"
                  min="0"
                  step="0.1"
                  value={inline[f.key as keyof typeof inline]}
                  onChange={(e) => setInline((p) => ({ ...p, [f.key]: e.target.value }))}
                  className={inputCls}
                />
              </div>
            ))}
          </div>
        )}
      </div>

      <div className={sectionCls} style={{ maxWidth: 200 }}>
        <label className={labelCls}>
          Strike / total volume (litres) <span className="text-[var(--color-danger)]">*</span>
        </label>
        <input
          type="number"
          min="0"
          step="0.5"
          value={volumeLiters}
          onChange={(e) => setVolumeLiters(e.target.value)}
          className={inputCls}
        />
      </div>

      <div className={sectionCls}>
        <div className="flex items-center justify-between mb-2">
          <p className="text-sm font-semibold text-[var(--color-fg)]">Mineral additions</p>
          <button
            onClick={addMineral}
            className="text-xs px-3 py-1.5 rounded border border-[var(--color-accent)] text-[var(--color-accent)] hover:bg-[var(--color-accent)] hover:text-white"
          >
            + Add mineral
          </button>
        </div>

        {minerals.length === 0 && (
          <p className="text-sm text-[var(--color-muted)]">No additions — source water will be used unchanged.</p>
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
              <div className="flex items-center gap-1">
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
              </div>
              <button
                onClick={() => removeMineral(m.id)}
                className="text-[var(--color-danger)] text-xs px-2 py-1 hover:opacity-70"
              >
                ×
              </button>
            </div>
          ))}
        </div>
      </div>

      {saveError && (
        <div className="mb-3 p-2 rounded text-sm text-[var(--color-danger)] border border-[var(--color-danger)]">
          {saveError}
        </div>
      )}
      {saveOk && (
        <div className="mb-3 p-2 rounded text-sm text-[#22c55e] bg-[#22c55e1a] border border-[#22c55e]">
          Saved
        </div>
      )}
      <div className="flex gap-3 mb-6">
        <button
          onClick={handleSave}
          disabled={
            !recipeId ||
            sourceMode !== 'profile' ||
            !profileId ||
            !name.trim() ||
            createMut.isPending ||
            updateMut.isPending
          }
          className="px-5 py-2 rounded text-sm border border-[var(--color-accent)] text-[var(--color-accent)] hover:bg-[var(--color-accent)] hover:text-white disabled:opacity-50"
        >
          {createMut.isPending || updateMut.isPending ? 'Saving…' : 'Save to recipe'}
        </button>
      </div>
      {!recipeId && (
        <p className='text-xs text-[var(--color-muted)] mt-1'>Save the recipe first to attach water chemistry.</p>
      )}
      {sourceMode === 'inline' && (
        <p className='text-xs text-[var(--color-muted)] mt-1'>Saving requires a saved water profile.</p>
      )}

      {liveResult && (
        <div
          className="p-4 rounded border"
          style={{ background: 'var(--color-surface)', borderColor: 'var(--color-border)' }}
        >
          <p className="text-sm font-semibold text-[var(--color-fg)] mb-3">Treated water profile</p>

          <div className="grid grid-cols-3 gap-3 mb-4">
            {RESULT_IONS.map((f) => (
              <div key={f.key} className="text-center">
                <p className="text-xs text-[var(--color-muted)] mb-0.5">{f.label}</p>
                <p className="text-lg font-semibold text-[var(--color-fg)] tabular-nums">
                  {liveResult[f.key] != null ? Number(liveResult[f.key]).toFixed(1) : '—'}
                </p>
                <p className="text-xs text-[var(--color-muted)]">ppm</p>
              </div>
            ))}
          </div>

          <div className="border-t pt-3 grid grid-cols-3 gap-3" style={{ borderColor: 'var(--color-border)' }}>
            <div className="text-center">
              <p className="text-xs text-[var(--color-muted)] mb-0.5">Alkalinity</p>
              <p className="text-base font-semibold text-[var(--color-fg)] tabular-nums">
                {liveResult.alkalinity != null ? Number(liveResult.alkalinity).toFixed(1) : '—'}
              </p>
              <p className="text-xs text-[var(--color-muted)]">ppm as CaCO₃</p>
            </div>
            <div className="text-center">
              <p className="text-xs text-[var(--color-muted)] mb-0.5">Residual Alk.</p>
              <p className="text-base font-semibold text-[var(--color-fg)] tabular-nums">
                {liveResult.residual_alk != null ? Number(liveResult.residual_alk).toFixed(1) : '—'}
              </p>
              <p className="text-xs text-[var(--color-muted)]">ppm as CaCO₃</p>
            </div>
            <div className="text-center">
              <p className="text-xs text-[var(--color-muted)] mb-0.5">SO₄ : Cl</p>
              <p
                className="text-base font-semibold tabular-nums"
                style={{ color: scColor(liveResult.sulfate_to_chloride) }}
              >
                {sc}
              </p>
              <p className="text-xs text-[var(--color-muted)]">ratio</p>
            </div>
          </div>

          {liveResult.mash_ph != null && liveResult.mash_ph > 0 && (
            <div
              className="border-t mt-3 pt-3"
              style={{ borderColor: 'var(--color-border)' }}
            >
              <div className="flex items-center gap-3">
                <p className="text-xs text-[var(--color-muted)]">Predicted mash pH</p>
                <p
                  className="text-xl font-bold tabular-nums"
                  style={{ color: phColor(liveResult.mash_ph) }}
                >
                  {Number(liveResult.mash_ph).toFixed(2)}
                </p>
                <p className="text-xs text-[var(--color-muted)]">(target: 5.2 – 5.4)</p>
              </div>
              <p className="text-xs text-[var(--color-muted)] mt-1">
                Estimated from the grain bill (colour-classified)
              </p>
            </div>
          )}
        </div>
      )}
    </div>
  )
}

function scColor(ratio: number | undefined | null): string {
  if (ratio == null) return 'var(--color-fg)'
  const r = Number(ratio)
  if (r < 0.5) return 'var(--color-danger)'
  if (r > 9) return 'var(--color-danger)'
  if (r >= 1 && r <= 4) return '#22c55e'
  return 'var(--color-fg)'
}

function phColor(ph: number | undefined | null): string {
  if (ph == null) return 'var(--color-fg)'
  const p = Number(ph)
  if (p >= 5.2 && p <= 5.4) return '#22c55e'
  if (p >= 5.0 && p < 5.2) return '#f59e0b'
  if (p > 5.4 && p <= 5.6) return '#f59e0b'
  return 'var(--color-danger)'
}
