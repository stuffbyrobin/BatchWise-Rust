import { useState, useEffect, useMemo, useRef, useCallback } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { useBrewingPhysics } from '../../lib/physics/useBrewingPhysics'
import { useRecipe, useCreateRecipe, useUpdateRecipe } from './hooks/useRecipes'
import { APIError } from '../../api/error'
import type { components } from '../../api/generated'
import { EBCSwatch } from '../../components/ui/EBCSwatch'
import { useTenant } from '../account/hooks/useTenant'
import { useAuth } from '../../auth/useAuth'
import { useCanWrite } from '../../auth/useCan'
import type { IBUMethod } from '../../utils/ibu'
import { formatEbc } from '../../utils/ebc'
import { useRecipeAllergens } from './hooks/useRecipeAllergens'
import { AllergenBadges } from '../../components/AllergenBadges'
import { RecipeWaterChemistry } from './RecipeWaterChemistry'
import { useMaltOptions } from './useMaltOptions'
import { useHopOptions } from './useHopOptions'
import { useYeastOptions } from './useYeastOptions'
import { nextStepOrder, useIngredientRows } from './editor/useIngredientRows'
import {
  FERMENTABLE_NUMERIC,
  HOP_NUMERIC,
  MASH_NUMERIC,
  TYPE_OPTIONS,
  YEAST_NUMERIC,
  newFermentable,
  newHop,
  newMashStep,
  newYeast,
  type Fermentable,
  type Hop,
  type MashStep,
  type RecipeType,
  type Yeast,
} from './editor/model'
import { FermentableRow } from './editor/FermentableRow'
import { HopRow } from './editor/HopRow'
import { YeastRow } from './editor/YeastRow'
import { MashStepRow } from './editor/MashStepRow'

export default function RecipeEditorPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const isEditMode = !!id
  const canWrite = useCanWrite('production')
  const { data: tenant } = useTenant()
  const ibuMethod = (tenant?.ibu_method ?? 'tinseth') as IBUMethod
  const { user } = useAuth()
  const hasAllergens = user?.feature_flags?.['allergens'] === true
  const { data: allergenData } = useRecipeAllergens(hasAllergens && isEditMode ? id : undefined)

  // Top-level state
  const [name, setName] = useState('')
  const [type, setType] = useState<RecipeType>('all_grain')
  const [batchSizeLiters, setBatchSizeLiters] = useState(0)
  const [boilSizeLiters, setBoilSizeLiters] = useState<number | null>(null)
  const [boilTimeMinutes, setBoilTimeMinutes] = useState<number | null>(null)
  const [efficiencyPct, setEfficiencyPct] = useState<number | null>(null)
  const [notes, setNotes] = useState('')

  // Ingredient tables, keyed by client-only row uids. A new recipe starts with
  // one blank fermentable.
  const fermentables = useIngredientRows<Fermentable>(FERMENTABLE_NUMERIC, isEditMode ? [] : [newFermentable(1)])
  const hops = useIngredientRows<Hop>(HOP_NUMERIC)
  const yeasts = useIngredientRows<Yeast>(YEAST_NUMERIC)
  const mashSteps = useIngredientRows<MashStep>(MASH_NUMERIC)
  // The hook's callbacks are stable, so the memoized rows only re-render when
  // their own row changes.
  const { reset: resetFermentables, update: updateFermentable, remove: removeFermentable, pick: pickFermentable } = fermentables
  const { reset: resetHops, update: updateHop, remove: removeHop, pick: pickHop } = hops
  const { reset: resetYeasts, update: updateYeast, remove: removeYeast, pick: pickYeast } = yeasts
  const { reset: resetMashSteps, update: updateMashStep, remove: removeMashStep } = mashSteps

  // Computed values from API response
  const [calcOg, setCalcOg] = useState<number | null>(null)
  const [calcFg, setCalcFg] = useState<number | null>(null)
  const [calcAbvPct, setCalcAbvPct] = useState<number | null>(null)
  const [calcIbu, setCalcIbu] = useState<number | null>(null)
  const [calcColorEbc, setCalcColorEbc] = useState<number | null>(null)

  // Load recipe data in edit mode
  const { data: recipeData, isLoading, isError: isLoadError, error: loadError } = useRecipe(id || '')
  // Id of the recipe whose data has already been copied into local state.
  const hydratedRecipeId = useRef<string | null>(null)

  // Create/Update mutations
  const createMutation = useCreateRecipe()
  const updateMutation = useUpdateRecipe(id || '')

  // Error state
  const [errorMsg, setErrorMsg] = useState<string | null>(null)

  // Water chemistry section collapse toggle
  const [showWater, setShowWater] = useState(false)

  // Live (unsaved) estimates, computed client-side by the same Rust physics the
  // server runs (compiled to WASM). Recomputed as the grain/hop bill changes,
  // so the brewer sees OG/FG/ABV/IBU/colour update while editing — before saving.
  const physics = useBrewingPhysics()
  const liveCalcs = useMemo(() => {
    if (!physics.ready || !batchSizeLiters) return null
    return physics.computeRecipeCalcs({
      batch_size_liters: batchSizeLiters,
      efficiency_pct: efficiencyPct,
      attenuation_pct: yeasts.rows[0]?.attenuation_pct ?? null,
      fermentables: fermentables.rows.map((f) => ({
        amount: f.amount,
        unit: f.unit,
        potential_ppg: f.potential_ppg ?? null,
        color_ebc: f.color_ebc ?? null,
      })),
      hops: hops.rows.map((h) => ({
        amount: h.amount,
        unit: h.unit,
        alpha_acid_pct: h.alpha_acid_pct,
        boil_time_minutes: h.boil_time_minutes,
        form: h.form ?? null,
        use: h.use ?? null,
      })),
    })
    // physics.computeRecipeCalcs is a stable module singleton; gate on `ready`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [physics.ready, batchSizeLiters, efficiencyPct, fermentables.rows, hops.rows, yeasts.rows])

  // Populate state from loaded recipe
  useEffect(() => {
    if (!recipeData) return
    // Later refetches (e.g. the query invalidation after a save) must not
    // overwrite in-progress edits or regenerate row uids, which would remount
    // every row and drop the custom-row flags.
    if (recipeData.id != null && hydratedRecipeId.current === recipeData.id) return
    hydratedRecipeId.current = recipeData.id ?? null

    setName(recipeData.name ?? '')
    setType((recipeData.type as RecipeType) ?? 'all_grain')
    setBatchSizeLiters(recipeData.batch_size_liters ?? 0)
    setBoilSizeLiters(recipeData.boil_size_liters ?? null)
    setBoilTimeMinutes(recipeData.boil_time_minutes ?? null)
    setEfficiencyPct(recipeData.efficiency_pct ?? null)
    setNotes(recipeData.notes ?? '')

    // Set computed values
    setCalcOg(recipeData.calc_og ?? null)
    setCalcFg(recipeData.calc_fg ?? null)
    setCalcAbvPct(recipeData.calc_abv_pct ?? null)
    setCalcIbu(recipeData.calc_ibu ?? null)
    setCalcColorEbc(recipeData.calc_color_ebc ?? null)

    if (recipeData.fermentables) {
      const mapped: Fermentable[] = recipeData.fermentables.map((f) => ({
        step_order: f.step_order ?? 1,
        name: f.name ?? '',
        amount: f.amount ?? 0,
        unit: (f.unit as Fermentable['unit']) ?? 'kg',
        color_ebc: f.color_ebc ?? undefined,
        potential_ppg: f.potential_ppg ?? undefined,
        type: f.type ?? undefined,
        addition: f.addition ?? undefined,
      }))
      resetFermentables(mapped.length > 0 ? mapped : [newFermentable(1)])
    }

    if (recipeData.hops) {
      resetHops(
        recipeData.hops.map((h) => ({
          step_order: h.step_order ?? 1,
          name: h.name ?? '',
          amount: h.amount ?? 0,
          unit: (h.unit as Hop['unit']) ?? 'g',
          alpha_acid_pct: h.alpha_acid_pct ?? 0,
          boil_time_minutes: h.boil_time_minutes ?? 0,
          form: (h.form ?? undefined) as Hop['form'],
          use: (h.use ?? undefined) as Hop['use'],
        })),
      )
    }

    if (recipeData.yeasts) {
      resetYeasts(
        recipeData.yeasts.map((y) => ({
          yeast_id: y.yeast_id ?? undefined,
          name: y.name ?? '',
          amount: y.amount ?? 0,
          unit: (y.unit as Yeast['unit']) ?? 'g',
          attenuation_pct: y.attenuation_pct ?? undefined,
        })),
      )
    }

    if (recipeData.mash_steps) {
      resetMashSteps(
        recipeData.mash_steps.map((m) => ({
          step_order: m.step_order ?? 1,
          step_type: (m.step_type as MashStep['step_type']) ?? 'infusion',
          target_temp_c: m.target_temp_c ?? 0,
          hold_minutes: m.hold_minutes ?? 0,
          infusion_volume_liters: m.infusion_volume_liters ?? undefined,
        })),
      )
    }
  }, [recipeData, resetFermentables, resetHops, resetYeasts, resetMashSteps])

  // In-stock + library options for the pickers.
  const malts = useMaltOptions()
  const hopOptions = useHopOptions()
  const yeastOptions = useYeastOptions()

  const pickMalt = useCallback(
    (uid: number, value: string) =>
      pickFermentable(uid, value, {
        resolve: (key) => {
          const opt = malts.byKey.get(key)
          if (!opt) return undefined
          return { name: opt.name, type: opt.type, color_ebc: opt.color_ebc, potential_ppg: opt.potential_ppg }
        },
        none: { name: '' },
      }),
    [pickFermentable, malts.byKey],
  )

  const pickHopOption = useCallback(
    (uid: number, value: string) =>
      pickHop(uid, value, {
        resolve: (key) => {
          const opt = hopOptions.byKey.get(key)
          if (!opt) return undefined
          return opt.alpha_acid_pct != null ? { name: opt.name, alpha_acid_pct: opt.alpha_acid_pct } : { name: opt.name }
        },
        none: { name: '' },
      }),
    [pickHop, hopOptions.byKey],
  )

  const pickYeastOption = useCallback(
    (uid: number, value: string) =>
      pickYeast(uid, value, {
        resolve: (key) => {
          const opt = yeastOptions.byKey.get(key)
          if (!opt) return undefined
          // Generic (library) picks keep their strain id so batch scheduling can
          // use its kinetics; stock picks have no library id.
          const fields: Partial<Yeast> = { name: opt.name, yeast_id: opt.source === 'generic' ? opt.id : undefined }
          if (opt.attenuation_pct != null) fields.attenuation_pct = opt.attenuation_pct
          return fields
        },
        none: { name: '', yeast_id: undefined },
        custom: { yeast_id: undefined },
      }),
    [pickYeast, yeastOptions.byKey],
  )

  const addFermentable = () => fermentables.add(newFermentable(nextStepOrder(fermentables.rows)))
  const addHop = () => hops.add(newHop(nextStepOrder(hops.rows)))
  const addYeast = () => yeasts.add(newYeast())
  const addMashStep = () => mashSteps.add(newMashStep(nextStepOrder(mashSteps.rows)))

  // Per-hop IBU estimate inputs.
  const ibuBatchSize = batchSizeLiters || 20
  const ibuOg = calcOg ?? 1.05

  const handleSave = () => {
    const payload: components['schemas']['CreateRecipeRequest'] | components['schemas']['PatchRecipeRequest'] = {
      name,
      type,
      batch_size_liters: batchSizeLiters,
      boil_size_liters: boilSizeLiters ?? undefined,
      boil_time_minutes: boilTimeMinutes ?? undefined,
      efficiency_pct: efficiencyPct ?? undefined,
      notes: notes || undefined,
      fermentables: fermentables.values.length > 0 ? fermentables.values : undefined,
      hops: hops.values.length > 0 ? hops.values : undefined,
      yeasts: yeasts.values.length > 0 ? yeasts.values : undefined,
      mash_steps: mashSteps.values.length > 0 ? mashSteps.values : undefined,
    }

    if (isEditMode) {
      updateMutation.mutate(payload as components['schemas']['PatchRecipeRequest'], {
        onSuccess: (data) => {
          // Update computed values from response
          setCalcOg(data.calc_og ?? null)
          setCalcFg(data.calc_fg ?? null)
          setCalcAbvPct(data.calc_abv_pct ?? null)
          setCalcIbu(data.calc_ibu ?? null)
          setCalcColorEbc(data.calc_color_ebc ?? null)
          setErrorMsg(null)
        },
        onError: (err) => {
          setErrorMsg((err as APIError).message || 'Save failed')
        },
      })
    } else {
      createMutation.mutate(payload as components['schemas']['CreateRecipeRequest'], {
        onSuccess: (data) => {
          navigate(`/recipes/${data.id}`)
        },
        onError: (err) => {
          setErrorMsg((err as APIError).message || 'Create failed')
        },
      })
    }
  }

  const isSaving = createMutation.isPending || updateMutation.isPending

  if (isLoadError) {
    return (
      <div className="p-4">
        <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded">
          Error loading recipe: {(loadError as APIError).message}
        </div>
      </div>
    )
  }

  return (
    <div
      className="p-4"
      // Select a number field's contents on focus so typing replaces the
      // default 0 instead of appending to it (e.g. "04"). onFocus bubbles, so
      // this one handler covers every numeric input in the editor.
      onFocus={(e) => {
        const t = e.target as HTMLElement
        if (t instanceof HTMLInputElement && t.type === 'number') t.select()
      }}
    >
      <div className="flex justify-between items-center mb-4">
        <h1 className="text-2xl font-bold">{isEditMode ? 'Edit Recipe' : 'New Recipe'}</h1>
      </div>

      {isLoading ? (
        <div className="text-center py-8">Loading...</div>
      ) : (
        <>
          {errorMsg && (
            <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4">
              {errorMsg}
            </div>
          )}

          <div className="bg-[var(--color-surface)] p-6 rounded shadow mb-4">
            <h2 className="text-lg font-semibold mb-4 text-[var(--color-fg)]">Basic Information</h2>
            <fieldset disabled={!canWrite} className="contents">
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              <div>
                <label htmlFor="recipe-name" className="block text-sm font-medium text-[var(--color-fg)] mb-1">Name *</label>
                <input
                  id="recipe-name"
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="Recipe name"
                  className="border border-[var(--color-border)] rounded px-3 py-2 w-full bg-[var(--color-surface)] text-[var(--color-fg)]"
                />
              </div>
              <div>
                <label htmlFor="recipe-type" className="block text-sm font-medium text-[var(--color-fg)] mb-1">Type</label>
                <select
                  id="recipe-type"
                  value={type}
                  onChange={(e) => setType(e.target.value as RecipeType)}
                  className="border border-[var(--color-border)] rounded px-3 py-2 w-full bg-[var(--color-surface)] text-[var(--color-fg)]"
                >
                  {TYPE_OPTIONS.map((opt) => (
                    <option key={opt.value} value={opt.value}>
                      {opt.label}
                    </option>
                  ))}
                </select>
              </div>
              <div>
                <label htmlFor="recipe-batch-size" className="block text-sm font-medium text-[var(--color-fg)] mb-1">Batch Size (L) *</label>
                <input
                  id="recipe-batch-size"
                  type="number"
                  value={batchSizeLiters}
                  onChange={(e) => setBatchSizeLiters(Number(e.target.value))}
                  className="border border-[var(--color-border)] rounded px-3 py-2 w-full bg-[var(--color-surface)] text-[var(--color-fg)]"
                />
              </div>
              <div>
                <label htmlFor="recipe-boil-size" className="block text-sm font-medium text-[var(--color-fg)] mb-1">Boil Size (L)</label>
                <input
                  id="recipe-boil-size"
                  type="number"
                  value={boilSizeLiters ?? ''}
                  onChange={(e) => setBoilSizeLiters(e.target.value === '' ? null : Number(e.target.value))}
                  placeholder="Boil size"
                  className="border border-[var(--color-border)] rounded px-3 py-2 w-full bg-[var(--color-surface)] text-[var(--color-fg)]"
                />
              </div>
              <div>
                <label htmlFor="recipe-boil-time" className="block text-sm font-medium text-[var(--color-fg)] mb-1">Boil Time (min)</label>
                <input
                  id="recipe-boil-time"
                  type="number"
                  value={boilTimeMinutes ?? ''}
                  onChange={(e) => setBoilTimeMinutes(e.target.value === '' ? null : Number(e.target.value))}
                  placeholder="Boil time"
                  className="border border-[var(--color-border)] rounded px-3 py-2 w-full bg-[var(--color-surface)] text-[var(--color-fg)]"
                />
              </div>
              <div>
                <label htmlFor="recipe-efficiency" className="block text-sm font-medium text-[var(--color-fg)] mb-1">Efficiency %</label>
                <input
                  id="recipe-efficiency"
                  type="number"
                  value={efficiencyPct ?? ''}
                  onChange={(e) => setEfficiencyPct(e.target.value === '' ? null : Number(e.target.value))}
                  placeholder="Efficiency"
                  className="border border-[var(--color-border)] rounded px-3 py-2 w-full bg-[var(--color-surface)] text-[var(--color-fg)]"
                />
              </div>
              <div className="md:col-span-3">
                <label htmlFor="recipe-notes" className="block text-sm font-medium text-[var(--color-fg)] mb-1">Notes</label>
                <textarea
                  id="recipe-notes"
                  value={notes}
                  onChange={(e) => setNotes(e.target.value)}
                  placeholder="Additional notes..."
                  rows={4}
                  className="border border-[var(--color-border)] rounded px-3 py-2 w-full bg-[var(--color-surface)] text-[var(--color-fg)]"
                />
              </div>
              </div>
            </fieldset>
          </div>

          {/* Fermentables */}
          <div className="bg-[var(--color-surface)] p-6 rounded shadow mb-4">
            <div className="flex justify-between items-center mb-4">
              <h2 className="text-lg font-semibold text-[var(--color-fg)]">Fermentables</h2>
              {canWrite && (
                <button onClick={addFermentable} className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700">
                  Add Fermentable
                </button>
              )}
            </div>
            <div className="overflow-x-auto">
              <fieldset disabled={!canWrite} className="contents">
                <table className="w-full">
                  <thead className="bg-[var(--color-bg)] border-b border-[var(--color-border)]">
                    <tr>
                      <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Order</th>
                      <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Name</th>
                      <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Amount</th>
                      <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Unit</th>
                      <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Color EBC</th>
                      <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Potential (PPG)</th>
                      <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Type</th>
                      <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Addition</th>
                      <th className="px-3 py-2"></th>
                    </tr>
                  </thead>
                  <tbody>
                    {fermentables.rows.map((f) => (
                      <FermentableRow
                        key={f.uid}
                        row={f}
                        custom={fermentables.customRows.has(f.uid)}
                        options={malts}
                        onUpdate={updateFermentable}
                        onPick={pickMalt}
                        onRemove={removeFermentable}
                      />
                    ))}
                  </tbody>
                </table>
              </fieldset>
            </div>
          </div>

          {/* Hops */}
          <div className="bg-[var(--color-surface)] p-6 rounded shadow mb-4">
            <div className="flex justify-between items-center mb-4">
              <h2 className="text-lg font-semibold text-[var(--color-fg)]">Hops</h2>
              {canWrite && (
                <button onClick={addHop} className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700">
                  Add Hop
                </button>
              )}
            </div>
            <div className="overflow-x-auto">
              <fieldset disabled={!canWrite} className="contents">
                <table className="w-full">
                  <thead className="bg-[var(--color-bg)] border-b border-[var(--color-border)]">
                    <tr>
                      <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Order</th>
                      <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Name</th>
                      <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Amount</th>
                      <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Unit</th>
                      <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Alpha Acid %</th>
                      <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Boil Time (min)</th>
                      <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Form</th>
                      <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Use</th>
                      <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">IBU</th>
                      <th className="px-3 py-2"></th>
                    </tr>
                  </thead>
                  <tbody>
                    {hops.rows.map((h) => (
                      <HopRow
                        key={h.uid}
                        row={h}
                        custom={hops.customRows.has(h.uid)}
                        options={hopOptions}
                        ibuMethod={ibuMethod}
                        batchSizeLiters={ibuBatchSize}
                        og={ibuOg}
                        onUpdate={updateHop}
                        onPick={pickHopOption}
                        onRemove={removeHop}
                      />
                    ))}
                  </tbody>
                </table>
              </fieldset>
            </div>
          </div>

          {/* Yeasts */}
          <div className="bg-[var(--color-surface)] p-6 rounded shadow mb-4">
            <div className="flex justify-between items-center mb-4">
              <h2 className="text-lg font-semibold text-[var(--color-fg)]">Yeasts</h2>
              {canWrite && (
                <button onClick={addYeast} className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700">
                  Add Yeast
                </button>
              )}
            </div>
            <div className="overflow-x-auto">
              <fieldset disabled={!canWrite} className="contents">
              <table className="w-full">
                <thead className="bg-[var(--color-bg)] border-b border-[var(--color-border)]">
                  <tr>
                    <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Name</th>
                    <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Amount</th>
                    <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Unit</th>
                    <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Attenuation %</th>
                    <th className="px-3 py-2"></th>
                  </tr>
                </thead>
                <tbody>
                  {yeasts.rows.map((y) => (
                    <YeastRow
                      key={y.uid}
                      row={y}
                      custom={yeasts.customRows.has(y.uid)}
                      options={yeastOptions}
                      onUpdate={updateYeast}
                      onPick={pickYeastOption}
                      onRemove={removeYeast}
                    />
                  ))}
                </tbody>
              </table>
              </fieldset>
            </div>
          </div>

          {/* Mash Steps */}
          <div className="bg-[var(--color-surface)] p-6 rounded shadow mb-4">
            <div className="flex justify-between items-center mb-4">
              <h2 className="text-lg font-semibold text-[var(--color-fg)]">Mash Steps</h2>
              {canWrite && (
                <button onClick={addMashStep} className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700">
                  Add Mash Step
                </button>
              )}
            </div>
            <div className="overflow-x-auto">
              <fieldset disabled={!canWrite} className="contents">
              <table className="w-full">
                <thead className="bg-[var(--color-bg)] border-b border-[var(--color-border)]">
                  <tr>
                    <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Order</th>
                    <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Type</th>
                    <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Target Temp (C)</th>
                    <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Hold (min)</th>
                    <th className="px-3 py-2 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Infusion Volume (L)</th>
                    <th className="px-3 py-2"></th>
                  </tr>
                </thead>
                <tbody>
                  {mashSteps.rows.map((m) => (
                    <MashStepRow key={m.uid} row={m} onUpdate={updateMashStep} onRemove={removeMashStep} />
                  ))}
                </tbody>
              </table>
              </fieldset>
            </div>
          </div>

          {/* Water Chemistry & pH */}
          <div className="mb-4">
            <button
              type="button"
              onClick={() => setShowWater((v) => !v)}
              className="w-full flex justify-between items-center bg-[var(--color-surface)] px-6 py-4 rounded shadow text-left hover:opacity-90"
            >
              <h2 className="text-lg font-semibold text-[var(--color-fg)]">Water Chemistry &amp; pH</h2>
              <span className="text-[var(--color-muted)]">{showWater ? '▲' : '▼'}</span>
            </button>
            {showWater && (
              <div className="mt-2">
                <RecipeWaterChemistry recipeId={id} fermentables={fermentables.values} />
              </div>
            )}
          </div>

          {/* Live estimate (unsaved) — computed in-browser by the same Rust
              physics the server runs, updating as you edit the bill. */}
          {liveCalcs && (
            <div className="bg-[var(--color-surface)] p-6 rounded shadow mb-4">
              <h2 className="text-lg font-semibold mb-1 text-[var(--color-fg)]">Live estimate</h2>
              <p className="text-xs text-[var(--color-muted)] mb-4">
                Computed in your browser as you edit (same engine as the server). Saved values appear below.
              </p>
              <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
                <div>
                  <div className="block text-sm font-medium text-[var(--color-muted)] mb-1">OG</div>
                  <div className="border border-[var(--color-border)] rounded px-3 py-2 bg-[var(--color-bg)] text-[var(--color-fg)] tabular-nums">{liveCalcs.calcOg.toFixed(3)}</div>
                </div>
                <div>
                  <div className="block text-sm font-medium text-[var(--color-muted)] mb-1">FG</div>
                  <div className="border border-[var(--color-border)] rounded px-3 py-2 bg-[var(--color-bg)] text-[var(--color-fg)] tabular-nums">{liveCalcs.calcFg.toFixed(3)}</div>
                </div>
                <div>
                  <div className="block text-sm font-medium text-[var(--color-muted)] mb-1">ABV %</div>
                  <div className="border border-[var(--color-border)] rounded px-3 py-2 bg-[var(--color-bg)] text-[var(--color-fg)] tabular-nums">{liveCalcs.calcAbvPct.toFixed(1)}%</div>
                </div>
                <div>
                  <div className="block text-sm font-medium text-[var(--color-muted)] mb-1">IBU</div>
                  <div className="border border-[var(--color-border)] rounded px-3 py-2 bg-[var(--color-bg)] text-[var(--color-fg)] tabular-nums">{liveCalcs.calcIbu.toFixed(1)}</div>
                </div>
                <div>
                  <div className="block text-sm font-medium text-[var(--color-muted)] mb-1">Color (EBC)</div>
                  <div className="border border-[var(--color-border)] rounded px-3 py-2 bg-[var(--color-bg)] text-[var(--color-fg)] flex items-center gap-2 tabular-nums">
                    <EBCSwatch ebc={liveCalcs.calcColorEbc} />
                    {formatEbc(liveCalcs.calcColorEbc)}
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* Computed Values */}
          {(calcOg !== null || calcFg !== null || calcAbvPct !== null || calcIbu !== null || calcColorEbc !== null) && (
            <div className="bg-[var(--color-surface)] p-6 rounded shadow mb-4">
              <h2 className="text-lg font-semibold mb-4 text-[var(--color-fg)]">Computed Values (read-only)</h2>
              <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
                <div>
                  <div className="block text-sm font-medium text-[var(--color-muted)] mb-1">OG</div>
                  <div className="border border-[var(--color-border)] rounded px-3 py-2 bg-[var(--color-bg)] text-[var(--color-fg)]">{calcOg?.toFixed(3) ?? '-'}</div>
                </div>
                <div>
                  <div className="block text-sm font-medium text-[var(--color-muted)] mb-1">FG</div>
                  <div className="border border-[var(--color-border)] rounded px-3 py-2 bg-[var(--color-bg)] text-[var(--color-fg)]">{calcFg?.toFixed(3) ?? '-'}</div>
                </div>
                <div>
                  <div className="block text-sm font-medium text-[var(--color-muted)] mb-1">ABV %</div>
                  <div className="border border-[var(--color-border)] rounded px-3 py-2 bg-[var(--color-bg)] text-[var(--color-fg)]">{calcAbvPct?.toFixed(1) ?? '-'}%</div>
                </div>
                <div>
                  <div className="block text-sm font-medium text-[var(--color-muted)] mb-1">IBU</div>
                  <div className="border border-[var(--color-border)] rounded px-3 py-2 bg-[var(--color-bg)] text-[var(--color-fg)]">{calcIbu?.toFixed(1) ?? '-'}</div>
                </div>
                <div>
                  <div className="block text-sm font-medium text-[var(--color-muted)] mb-1">Color (EBC)</div>
                  <div className="border border-[var(--color-border)] rounded px-3 py-2 bg-[var(--color-bg)] text-[var(--color-fg)] flex items-center gap-2">{calcColorEbc != null && <EBCSwatch ebc={calcColorEbc} />}{formatEbc(calcColorEbc)}</div>
                </div>
              </div>
            </div>
          )}

          {hasAllergens && isEditMode && (
            <div className="rounded-xl border p-4 mb-4" style={{ borderColor: 'var(--color-border)', background: 'var(--color-surface)' }}>
              <h3 className="text-sm font-semibold mb-2" style={{ color: 'var(--color-fg)' }}>Allergen Declaration</h3>
              {allergenData ? (
                <>
                  {(allergenData.allergens?.length ?? 0) === 0 ? (
                    <p className="text-xs text-[var(--color-muted)]">No allergens detected for this recipe's ingredients.</p>
                  ) : (
                    <AllergenBadges allergens={allergenData.allergens ?? []} />
                  )}
                  {(allergenData.unmatched?.length ?? 0) > 0 && (
                    <p className="text-xs mt-2 text-[var(--color-muted)]">
                      Unmatched ingredients (no allergen data): {allergenData.unmatched?.join(', ')}
                    </p>
                  )}
                </>
              ) : (
                <p className="text-xs text-[var(--color-muted)]">Loading allergen data…</p>
              )}
            </div>
          )}

          <div className="flex gap-4">
            {canWrite && (
              <button
                onClick={handleSave}
                disabled={!name || !batchSizeLiters || isSaving}
                className="bg-green-600 text-white px-6 py-2 rounded hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isSaving ? 'Saving...' : 'Save'}
              </button>
            )}
            <button
              onClick={() => navigate('/recipes')}
              className="bg-[var(--color-border)] text-[var(--color-fg)] px-6 py-2 rounded hover:opacity-80"
            >
              Cancel
            </button>
          </div>
        </>
      )}
    </div>
  )
}
