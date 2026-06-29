import React from 'react'
import { APIError } from '../../api/error'
import { useWaterProfiles, useCalculateWater } from './hooks/useWater'
import type { components } from '../../api/generated'
import { mineralPayload } from './mineralForms'
import { MineralFormControls } from './MineralFormControls'

type WaterResult = components['schemas']['WaterResult']

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
  /** CaCl₂ only: 'anhydrous' | 'dihydrate' | 'liquid'. */
  form?: string
  /** CaCl₂ liquid only: solution strength %w/w. */
  strength?: string
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

export function WaterCalculatorPage() {
  const { data: profilesData } = useWaterProfiles({ page_size: 100, sort: 'name' })
  const calcMut = useCalculateWater()

  const [sourceMode, setSourceMode] = React.useState<SourceMode>('profile')
  const [profileId, setProfileId] = React.useState('')
  const [inline, setInline] = React.useState(blankInline())
  const [volumeLiters, setVolumeLiters] = React.useState('20')
  const [minerals, setMinerals] = React.useState<MineralRow[]>([])
  const [nextId, setNextId] = React.useState(1)
  const [calcError, setCalcError] = React.useState<string | null>(null)
  const [result, setResult] = React.useState<WaterResult | null>(null)

  const addMineral = () => {
    setMinerals((prev) => [...prev, { id: nextId, type: MINERALS[0].type, amount: '' }])
    setNextId((n) => n + 1)
  }

  const updateMineral = (id: number, key: 'type' | 'amount' | 'form' | 'strength', value: string) => {
    setMinerals((prev) => prev.map((m) => (m.id === id ? { ...m, [key]: value } : m)))
  }

  const removeMineral = (id: number) => {
    setMinerals((prev) => prev.filter((m) => m.id !== id))
  }

  const handleCalculate = async () => {
    setCalcError(null)
    setResult(null)

    const body: Record<string, unknown> = {
      volume_liters: Number(volumeLiters) || 0,
      mineral_additions: minerals
        .filter((m) => m.amount !== '' && Number(m.amount) > 0)
        .map(mineralPayload),
    }

    if (sourceMode === 'profile') {
      if (!profileId) {
        setCalcError('Select a source water profile.')
        return
      }
      body.source_profile_id = profileId
    } else {
      body.source_profile = {
        calcium_ppm: Number(inline.calcium_ppm) || 0,
        magnesium_ppm: Number(inline.magnesium_ppm) || 0,
        sodium_ppm: Number(inline.sodium_ppm) || 0,
        sulfate_ppm: Number(inline.sulfate_ppm) || 0,
        chloride_ppm: Number(inline.chloride_ppm) || 0,
        bicarbonate_ppm: Number(inline.bicarbonate_ppm) || 0,
      }
    }

    try {
      const res = await calcMut.mutateAsync(body)
      setResult(res)
    } catch (e) {
      setCalcError(e instanceof APIError ? e.message : 'Calculation failed')
    }
  }

  const sc = result
    ? result.sulfate_to_chloride != null
      ? Number(result.sulfate_to_chloride).toFixed(2)
      : '—'
    : null

  const profiles = profilesData?.items ?? []

  const inputCls =
    'w-full p-2 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)]'
  const sectionCls = 'mb-5'
  const labelCls = 'block text-xs text-[var(--color-muted)] mb-1'

  return (
    <div className="max-w-3xl">
      <h1 className="text-xl font-bold text-[var(--color-fg)] mb-4">Water Calculator</h1>
      <p className="text-sm text-[var(--color-muted)] mb-5">
        Calculate the effect of mineral additions on your water profile. Results are not saved —
        use the Adjustments page to persist a session.
      </p>

      {/* Source water */}
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

      {/* Volume */}
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

      {/* Mineral additions */}
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
              <MineralFormControls
                type={m.type}
                form={m.form}
                strength={m.strength}
                onForm={(v) => updateMineral(m.id, 'form', v)}
                onStrength={(v) => updateMineral(m.id, 'strength', v)}
              />
              <button
                onClick={() => removeMineral(m.id)}
                className="text-[var(--color-danger)] text-xs px-2 py-1 hover:opacity-70"
              >
                ✕
              </button>
            </div>
          ))}
        </div>
      </div>

      {/* Calculate button */}
      {calcError && (
        <div className="mb-3 p-2 rounded text-sm text-[var(--color-danger)] border border-[var(--color-danger)]">
          {calcError}
        </div>
      )}
      <button
        onClick={handleCalculate}
        disabled={calcMut.isPending}
        className="px-5 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50 mb-6"
      >
        {calcMut.isPending ? 'Calculating…' : 'Calculate'}
      </button>

      {/* Result */}
      {result && (
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
                  {result[f.key] != null ? Number(result[f.key]).toFixed(1) : '—'}
                </p>
                <p className="text-xs text-[var(--color-muted)]">ppm</p>
              </div>
            ))}
          </div>

          <div className="border-t pt-3 grid grid-cols-3 gap-3" style={{ borderColor: 'var(--color-border)' }}>
            <div className="text-center">
              <p className="text-xs text-[var(--color-muted)] mb-0.5">Alkalinity</p>
              <p className="text-base font-semibold text-[var(--color-fg)] tabular-nums">
                {result.alkalinity != null ? Number(result.alkalinity).toFixed(1) : '—'}
              </p>
              <p className="text-xs text-[var(--color-muted)]">ppm as CaCO₃</p>
            </div>
            <div className="text-center">
              <p className="text-xs text-[var(--color-muted)] mb-0.5">Residual Alk.</p>
              <p className="text-base font-semibold text-[var(--color-fg)] tabular-nums">
                {result.residual_alk != null ? Number(result.residual_alk).toFixed(1) : '—'}
              </p>
              <p className="text-xs text-[var(--color-muted)]">ppm as CaCO₃</p>
            </div>
            <div className="text-center">
              <p className="text-xs text-[var(--color-muted)] mb-0.5">SO₄ : Cl</p>
              <p
                className="text-base font-semibold tabular-nums"
                style={{ color: scColor(result.sulfate_to_chloride) }}
              >
                {sc}
              </p>
              <p className="text-xs text-[var(--color-muted)]">ratio</p>
            </div>
          </div>

          {result.mash_ph != null && result.mash_ph > 0 && (
            <div
              className="border-t mt-3 pt-3 flex items-center gap-3"
              style={{ borderColor: 'var(--color-border)' }}
            >
              <p className="text-xs text-[var(--color-muted)]">Predicted mash pH</p>
              <p
                className="text-xl font-bold tabular-nums"
                style={{ color: phColor(result.mash_ph) }}
              >
                {Number(result.mash_ph).toFixed(2)}
              </p>
              <p className="text-xs text-[var(--color-muted)]">(target: 5.2 – 5.4)</p>
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
