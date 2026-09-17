import { useEffect, useRef, useState } from 'react'
import { usePatchBatchIngredients, type Batch } from './hooks/useBatches'
import { useInventoryList } from '../inventory/hooks/useInventory'
import { useTenant } from '../account/hooks/useTenant'
import type { components } from '../../api/generated'
import { calcHopIBU, type IBUMethod } from '../../utils/ibu'
import { formatEbc } from '../../utils/ebc'
import { nextStepOrder, useIngredientRows, type EmptyAs } from '../recipes/editor/useIngredientRows'
import { HOP_FORMS, HOP_UNITS, HOP_USES } from '../recipes/editor/model'
import { NumberCell } from '../recipes/editor/NumberCell'

type RecipeFermentable = components['schemas']['RecipeFermentable']
type RecipeHop = components['schemas']['RecipeHop']
type RecipeYeast = components['schemas']['RecipeYeast']
type InventoryLot = components['schemas']['Ingredient']

// Required amounts store 0 when emptied; optional measurements store null.
const FERMENTABLE_NUMERIC: Partial<Record<keyof RecipeFermentable, EmptyAs>> = {
  amount: 'zero',
  color_ebc: 'null',
  potential_ppg: 'null',
}
const HOP_NUMERIC: Partial<Record<keyof RecipeHop, EmptyAs>> = {
  amount: 'zero',
  alpha_acid_pct: 'zero',
  boil_time_minutes: 'zero',
}
const YEAST_NUMERIC: Partial<Record<keyof RecipeYeast, EmptyAs>> = { amount: 'zero', attenuation_pct: 'null' }

/**
 * Editable ingredient lists of a batch's recipe snapshot, with a stock lot
 * picker per row. Resolves the tenant's IBU method and the batch volume and OG
 * used for the per-hop IBU estimate.
 */
export function BatchIngredientsEditor({ batch, canEdit }: { batch: Batch; canEdit: boolean }) {
  const { data: tenant } = useTenant()
  const ibuMethod = (tenant?.ibu_method ?? 'tinseth') as IBUMethod
  const batchOg = typeof batch.target_og === 'number' ? batch.target_og : 1.05
  const snapSize = batch.batch_recipe_snapshot?.batch_size_liters
  const batchVolL =
    typeof snapSize === 'number' && snapSize > 0
      ? snapSize
      : typeof batch.actual_volume_liters === 'number' && batch.actual_volume_liters > 0
        ? batch.actual_volume_liters
        : 20
  return <IngredientsEditor batch={batch} canEdit={canEdit} ibuMethod={ibuMethod} batchOg={batchOg} batchVolL={batchVolL} />
}

function LotPicker({
  label,
  name,
  invType,
  value,
  onChange,
  disabled,
}: {
  label: string
  name: string
  invType: string
  value: string
  onChange: (lotId: string, lot: InventoryLot | null) => void
  disabled: boolean
}) {
  const { data } = useInventoryList({ name, type: invType, page_size: 50 })
  const lots = data?.items ?? []

  // When lots data loads, sync analytical fields for the already-selected lot.
  // Uses a ref so it only fires once per selected lot, not on every re-render.
  const syncedLotRef = useRef<string | null>(null)
  useEffect(() => {
    if (!value || lots.length === 0) return
    if (syncedLotRef.current === value) return
    const lot = lots.find((l) => l.id === value) ?? null
    if (lot) {
      syncedLotRef.current = value
      onChange(value, lot)
    }
    // onChange is a new closure each render — intentionally excluded to avoid loops
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [value, lots])

  if (lots.length === 0) {
    return <span className="text-xs text-[var(--color-muted)] px-1">No stock</span>
  }

  return (
    <select
      aria-label={label}
      value={value}
      disabled={disabled}
      onChange={(e) => {
        syncedLotRef.current = e.target.value // mark as synced so useEffect won't re-fire
        const lot = lots.find((l) => l.id === e.target.value) ?? null
        onChange(e.target.value, lot)
      }}
      className="w-full bg-transparent rounded px-1 py-0.5 text-xs text-[var(--color-fg)] border border-transparent focus:outline-none focus:border-[var(--color-accent)] hover:border-[var(--color-border)] disabled:opacity-50"
    >
      <option value="">— FIFO —</option>
      {lots.map((lot) => (
        <option key={lot.id} value={lot.id}>
          {lot.lot_number} ({lot.amount} {lot.unit})
          {invType === 'hop' && lot.alpha_acid_pct != null ? ` · AA: ${lot.alpha_acid_pct}%` : ''}
          {invType === 'fermentable' && lot.color_ebc != null ? ` · ${formatEbc(lot.color_ebc)} EBC` : ''}
          {invType === 'yeast' && lot.attenuation_pct != null ? ` · ${lot.attenuation_pct}% att` : ''}
        </option>
      ))}
    </select>
  )
}

function cellCls(disabled: boolean) {
  return `w-full bg-transparent rounded px-1 py-0.5 text-sm text-[var(--color-fg)] focus:outline-none border border-transparent focus:border-[var(--color-accent)] hover:border-[var(--color-border)] ${disabled ? 'opacity-50 cursor-not-allowed' : ''}`
}

function IngredientsEditor({ batch, canEdit, ibuMethod, batchOg, batchVolL }: {
  batch: Batch; canEdit: boolean; ibuMethod: IBUMethod; batchOg: number; batchVolL: number
}) {
  const snap = batch.batch_recipe_snapshot
  const patchMut = usePatchBatchIngredients(batch.id ?? '')

  const fermentables = useIngredientRows<RecipeFermentable>(FERMENTABLE_NUMERIC, snap?.fermentables ?? [])
  const hops = useIngredientRows<RecipeHop>(HOP_NUMERIC, snap?.hops ?? [])
  const yeasts = useIngredientRows<RecipeYeast>(YEAST_NUMERIC, snap?.yeasts ?? [])
  const [saved, setSaved] = useState(false)
  const [saveError, setSaveError] = useState<string | null>(null)

  const handleSave = () => {
    setSaveError(null)
    patchMut.mutate(
      { fermentables: fermentables.values, hops: hops.values, yeasts: yeasts.values },
      {
        onSuccess: () => { setSaved(true); setTimeout(() => setSaved(false), 2000) },
        onError: (err) => setSaveError(err instanceof Error ? err.message : 'Save failed'),
      },
    )
  }

  const inputCls = cellCls(!canEdit)

  return (
    <div
      // Select a number field's contents on focus so typing replaces the value
      // instead of appending to it.
      onFocus={(e) => {
        const t = e.target as HTMLElement
        if (t instanceof HTMLInputElement && t.type === 'number') t.select()
      }}
    >
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-base font-semibold text-[var(--color-fg)]">Ingredients</h2>
        {canEdit && (
          <div className="flex items-center gap-3">
            {saveError && <span className="text-xs text-[var(--color-danger)]">{saveError}</span>}
            {saved && <span className="text-xs text-green-600">Saved</span>}
            <button
              onClick={handleSave}
              disabled={patchMut.isPending}
              className="px-4 py-1.5 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
            >
              {patchMut.isPending ? 'Saving…' : 'Save ingredients'}
            </button>
          </div>
        )}
        {!canEdit && (
          <span className="text-xs text-[var(--color-muted)]">Locked — terminal status</span>
        )}
      </div>

      {/* Fermentables */}
      <details open className="mb-4">
        <summary className="cursor-pointer text-sm font-medium text-[var(--color-muted)] hover:text-[var(--color-fg)] select-none mb-2">
          Fermentables ({fermentables.rows.length})
        </summary>
        <div className="overflow-x-auto border rounded-lg mt-2" style={{ borderColor: 'var(--color-border)', background: 'var(--color-surface)' }}>
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-[var(--color-muted)] uppercase tracking-wide border-b" style={{ borderColor: 'var(--color-border)', background: 'var(--color-bg)' }}>
                <th className="px-2 py-2">Name</th>
                <th className="px-2 py-2 w-24">Amount</th>
                <th className="px-2 py-2 w-16">Unit</th>
                <th className="px-2 py-2 w-28">Type</th>
                <th className="px-2 py-2 w-24">Colour (EBC)</th>
                <th className="px-2 py-2 w-24">Potential PPG</th>
                <th className="px-2 py-2 w-28">Addition</th>
                <th className="px-2 py-2 w-48">Lot (stock)</th>
                {canEdit && <th className="px-2 py-2 w-8" />}
              </tr>
            </thead>
            <tbody>
              {fermentables.rows.map((f) => (
                <tr key={f.uid} className="border-t" style={{ borderColor: 'var(--color-border)' }}>
                  <td className="px-1 py-1"><input aria-label="Fermentable name" className={inputCls} value={f.name ?? ''} disabled={!canEdit} onChange={(e) => fermentables.update(f.uid, 'name', e.target.value)} /></td>
                  <td className="px-1 py-1"><NumberCell label="Fermentable amount" className={inputCls} step="0.001" min="0" value={f.amount} disabled={!canEdit} onValueChange={(v) => fermentables.update(f.uid, 'amount', v)} /></td>
                  <td className="px-1 py-1">
                    {canEdit
                      ? <select aria-label="Fermentable unit" className={cellCls(false)} value={f.unit ?? 'kg'} onChange={(e) => fermentables.update(f.uid, 'unit', e.target.value)}>
                          <option value="kg">kg</option>
                          <option value="g">g</option>
                        </select>
                      : <span className="px-2 text-[var(--color-fg)] text-sm">{f.unit ?? 'kg'}</span>
                    }
                  </td>
                  <td className="px-1 py-1"><input aria-label="Fermentable type" className={inputCls} value={f.type ?? ''} disabled={!canEdit} onChange={(e) => fermentables.update(f.uid, 'type', e.target.value)} /></td>
                  <td className="px-1 py-1"><NumberCell label="Colour (EBC)" className={inputCls} step="0.1" min="0" value={f.color_ebc} disabled={!canEdit} onValueChange={(v) => fermentables.update(f.uid, 'color_ebc', v)} /></td>
                  <td className="px-1 py-1"><NumberCell label="Potential (PPG)" className={inputCls} step="0.1" min="0" value={f.potential_ppg} disabled={!canEdit} onValueChange={(v) => fermentables.update(f.uid, 'potential_ppg', v)} /></td>
                  <td className="px-1 py-1"><input aria-label="Addition" className={inputCls} value={f.addition ?? ''} disabled={!canEdit} onChange={(e) => fermentables.update(f.uid, 'addition', e.target.value || null)} /></td>
                  <td className="px-1 py-1 min-w-[160px]">
                    <LotPicker
                      label="Fermentable lot"
                      name={f.name ?? ''}
                      invType="fermentable"
                      value={f.inventory_lot_id ?? ''}
                      disabled={!canEdit}
                      onChange={(lotId, lot) =>
                        fermentables.patch(f.uid, {
                          inventory_lot_id: lotId || null,
                          ...(lotId && lot?.color_ebc != null ? { color_ebc: lot.color_ebc } : {}),
                        })
                      }
                    />
                  </td>
                  {canEdit && <td className="px-1 py-1 text-center"><button aria-label="Remove fermentable" onClick={() => fermentables.remove(f.uid)} className="text-[var(--color-danger)] hover:opacity-70 text-xs">✕</button></td>}
                </tr>
              ))}
              {fermentables.rows.length === 0 && (
                <tr><td colSpan={canEdit ? 9 : 8} className="px-2 py-4 text-center text-xs text-[var(--color-muted)]">No fermentables</td></tr>
              )}
            </tbody>
          </table>
        </div>
        {canEdit && (
          <button
            onClick={() => fermentables.add({ step_order: nextStepOrder(fermentables.rows), name: '', amount: 0, unit: 'kg', type: 'Grain', color_ebc: null, potential_ppg: null, addition: null })}
            className="mt-2 text-xs text-[var(--color-accent)] hover:opacity-70">+ Add fermentable</button>
        )}
      </details>

      {/* Hops */}
      <details open className="mb-4">
        <summary className="cursor-pointer text-sm font-medium text-[var(--color-muted)] hover:text-[var(--color-fg)] select-none mb-2">
          Hops ({hops.rows.length})
        </summary>
        <div className="overflow-x-auto border rounded-lg mt-2" style={{ borderColor: 'var(--color-border)', background: 'var(--color-surface)' }}>
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-[var(--color-muted)] uppercase tracking-wide border-b" style={{ borderColor: 'var(--color-border)', background: 'var(--color-bg)' }}>
                <th className="px-2 py-2">Name</th>
                <th className="px-2 py-2 w-24">Amount</th>
                <th className="px-2 py-2 w-16">Unit</th>
                <th className="px-2 py-2 w-20">Alpha %</th>
                <th className="px-2 py-2 w-24">Use</th>
                <th className="px-2 py-2 w-20">Time (min)</th>
                <th className="px-2 py-2 w-24">Form</th>
                <th className="px-2 py-2 w-20">IBU</th>
                <th className="px-2 py-2 w-48">Lot (stock)</th>
                {canEdit && <th className="px-2 py-2 w-8" />}
              </tr>
            </thead>
            <tbody>
              {hops.rows.map((h) => {
                const grams = h.unit === 'kg' ? (h.amount ?? 0) * 1000 : (h.amount ?? 0)
                const ibu = calcHopIBU(ibuMethod, grams, h.alpha_acid_pct ?? 0, h.boil_time_minutes ?? 0, batchVolL, batchOg)
                return (
                  <tr key={h.uid} className="border-t" style={{ borderColor: 'var(--color-border)' }}>
                    <td className="px-1 py-1"><input aria-label="Hop name" className={inputCls} value={h.name ?? ''} disabled={!canEdit} onChange={(e) => hops.update(h.uid, 'name', e.target.value)} /></td>
                    <td className="px-1 py-1"><NumberCell label="Hop amount" className={inputCls} step="0.1" min="0" value={h.amount} disabled={!canEdit} onValueChange={(v) => hops.update(h.uid, 'amount', v)} /></td>
                    <td className="px-1 py-1">
                      {canEdit
                        ? <select aria-label="Hop unit" className={cellCls(false)} value={h.unit ?? 'g'} onChange={(e) => hops.update(h.uid, 'unit', e.target.value)}>
                            {HOP_UNITS.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
                          </select>
                        : <span className="px-2 text-[var(--color-fg)] text-sm">{h.unit ?? 'g'}</span>
                      }
                    </td>
                    <td className="px-1 py-1"><NumberCell label="Alpha acid %" className={inputCls} step="0.1" min="0" max="100" value={h.alpha_acid_pct} disabled={!canEdit} onValueChange={(v) => hops.update(h.uid, 'alpha_acid_pct', v)} /></td>
                    <td className="px-1 py-1">
                      {canEdit
                        ? <select aria-label="Hop use" className={cellCls(false)} value={(h.use ?? '').toLowerCase()} onChange={(e) => hops.update(h.uid, 'use', e.target.value || null)}>
                            <option value="">—</option>
                            {HOP_USES.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
                          </select>
                        : <span className="px-2 text-[var(--color-fg)] text-sm">{h.use ?? '—'}</span>
                      }
                    </td>
                    <td className="px-1 py-1"><NumberCell label="Boil time (min)" className={inputCls} step="1" min="0" value={h.boil_time_minutes} disabled={!canEdit} onValueChange={(v) => hops.update(h.uid, 'boil_time_minutes', v)} /></td>
                    <td className="px-1 py-1">
                      {canEdit
                        ? <select aria-label="Hop form" className={cellCls(false)} value={(h.form ?? '').toLowerCase()} onChange={(e) => hops.update(h.uid, 'form', e.target.value || null)}>
                            <option value="">—</option>
                            {HOP_FORMS.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
                          </select>
                        : <span className="px-2 text-[var(--color-fg)] text-sm">{h.form ?? '—'}</span>
                      }
                    </td>
                    <td className="px-2 py-1 text-xs text-right tabular-nums text-[var(--color-muted)]">{ibu > 0 ? ibu.toFixed(1) : '—'}</td>
                    <td className="px-1 py-1 min-w-[160px]">
                      <LotPicker
                        label="Hop lot"
                        name={h.name ?? ''}
                        invType="hop"
                        value={h.inventory_lot_id ?? ''}
                        disabled={!canEdit}
                        onChange={(lotId, lot) =>
                          hops.patch(h.uid, {
                            inventory_lot_id: lotId || null,
                            ...(lotId && lot?.alpha_acid_pct != null ? { alpha_acid_pct: lot.alpha_acid_pct } : {}),
                          })
                        }
                      />
                    </td>
                    {canEdit && <td className="px-1 py-1 text-center"><button aria-label="Remove hop" onClick={() => hops.remove(h.uid)} className="text-[var(--color-danger)] hover:opacity-70 text-xs">✕</button></td>}
                  </tr>
                )
              })}
              {hops.rows.length === 0 && (
                <tr><td colSpan={canEdit ? 10 : 9} className="px-2 py-4 text-center text-xs text-[var(--color-muted)]">No hops</td></tr>
              )}
            </tbody>
          </table>
        </div>
        {canEdit && (
          <button
            onClick={() => hops.add({ step_order: nextStepOrder(hops.rows), name: '', amount: 0, unit: 'g', alpha_acid_pct: 0, boil_time_minutes: 60, use: 'boil', form: null })}
            className="mt-2 text-xs text-[var(--color-accent)] hover:opacity-70">+ Add hop</button>
        )}
      </details>

      {/* Yeasts */}
      <details open className="mb-4">
        <summary className="cursor-pointer text-sm font-medium text-[var(--color-muted)] hover:text-[var(--color-fg)] select-none mb-2">
          Yeasts ({yeasts.rows.length})
        </summary>
        <div className="overflow-x-auto border rounded-lg mt-2" style={{ borderColor: 'var(--color-border)', background: 'var(--color-surface)' }}>
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-[var(--color-muted)] uppercase tracking-wide border-b" style={{ borderColor: 'var(--color-border)', background: 'var(--color-bg)' }}>
                <th className="px-2 py-2">Name</th>
                <th className="px-2 py-2 w-24">Amount</th>
                <th className="px-2 py-2 w-20">Unit</th>
                <th className="px-2 py-2 w-24">Attenuation %</th>
                <th className="px-2 py-2 w-48">Lot (stock)</th>
                {canEdit && <th className="px-2 py-2 w-8" />}
              </tr>
            </thead>
            <tbody>
              {yeasts.rows.map((y) => (
                <tr key={y.uid} className="border-t" style={{ borderColor: 'var(--color-border)' }}>
                  <td className="px-1 py-1"><input aria-label="Yeast name" className={inputCls} value={y.name ?? ''} disabled={!canEdit} onChange={(e) => yeasts.update(y.uid, 'name', e.target.value)} /></td>
                  <td className="px-1 py-1"><NumberCell label="Yeast amount" className={inputCls} step="0.01" min="0" value={y.amount} disabled={!canEdit} onValueChange={(v) => yeasts.update(y.uid, 'amount', v)} /></td>
                  <td className="px-1 py-1">
                    {canEdit
                      ? <select aria-label="Yeast unit" className={cellCls(false)} value={y.unit ?? 'count'} onChange={(e) => yeasts.update(y.uid, 'unit', e.target.value)}>
                          <option value="count">count</option>
                          <option value="g">g</option>
                          <option value="mL">mL</option>
                        </select>
                      : <span className="px-2 text-[var(--color-fg)] text-sm">{y.unit ?? ''}</span>
                    }
                  </td>
                  <td className="px-1 py-1"><NumberCell label="Attenuation %" className={inputCls} step="0.1" min="0" max="100" value={y.attenuation_pct} disabled={!canEdit} onValueChange={(v) => yeasts.update(y.uid, 'attenuation_pct', v)} /></td>
                  <td className="px-1 py-1 min-w-[160px]">
                    <LotPicker
                      label="Yeast lot"
                      name={y.name ?? ''}
                      invType="yeast"
                      value={y.inventory_lot_id ?? ''}
                      disabled={!canEdit}
                      onChange={(lotId, lot) =>
                        yeasts.patch(y.uid, {
                          inventory_lot_id: lotId || null,
                          ...(lotId && lot?.attenuation_pct != null ? { attenuation_pct: lot.attenuation_pct } : {}),
                        })
                      }
                    />
                  </td>
                  {canEdit && <td className="px-1 py-1 text-center"><button aria-label="Remove yeast" onClick={() => yeasts.remove(y.uid)} className="text-[var(--color-danger)] hover:opacity-70 text-xs">✕</button></td>}
                </tr>
              ))}
              {yeasts.rows.length === 0 && (
                <tr><td colSpan={canEdit ? 6 : 5} className="px-2 py-4 text-center text-xs text-[var(--color-muted)]">No yeasts</td></tr>
              )}
            </tbody>
          </table>
        </div>
        {canEdit && (
          <button
            onClick={() => yeasts.add({ name: '', amount: 1, unit: 'count', attenuation_pct: null, yeast_id: null })}
            className="mt-2 text-xs text-[var(--color-accent)] hover:opacity-70">+ Add yeast</button>
        )}
      </details>
    </div>
  )
}
