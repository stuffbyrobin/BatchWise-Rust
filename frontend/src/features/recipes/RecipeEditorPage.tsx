import { useState, useEffect, useMemo } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { useBrewingPhysics } from '../../lib/physics/useBrewingPhysics'
import { useRecipe, useCreateRecipe, useUpdateRecipe } from './hooks/useRecipes'
import { APIError } from '../../api/error'
import type { components } from '../../api/generated'
import { EBCSwatch } from '../../components/ui/EBCSwatch'
import { useTenant } from '../account/hooks/useTenant'
import { useAuth } from '../../auth/useAuth'
import { calcHopIBU, type IBUMethod } from '../../utils/ibu'
import { useRecipeAllergens } from './hooks/useRecipeAllergens'
import { AllergenBadges } from '../../components/AllergenBadges'
import { RecipeWaterChemistry } from './RecipeWaterChemistry'

type RecipeType = 'all_grain' | 'extract' | 'partial_mash' | 'cider' | 'mead' | 'other'

type Fermentable = {
  step_order: number
  name: string
  amount: number
  unit: 'kg' | 'g'
  color_ebc?: number
  potential_ppg?: number
  type?: string
  addition?: string
}

type Hop = {
  step_order: number
  name: string
  amount: number
  unit: 'g' | 'kg'
  alpha_acid_pct: number
  boil_time_minutes: number
  form?: 'pellet' | 'leaf' | 'extract'
  use?: 'boil' | 'whirlpool' | 'dry-hop' | 'first-wort' | 'mash'
}

type Yeast = {
  name: string
  amount: number
  unit: 'g' | 'mL' | 'count'
  attenuation_pct?: number
}

type MashStep = {
  step_order: number
  step_type: 'infusion' | 'temperature' | 'decoction'
  target_temp_c: number
  hold_minutes: number
  infusion_volume_liters?: number
}

const TYPE_OPTIONS: { value: RecipeType; label: string }[] = [
  { value: 'all_grain', label: 'All Grain' },
  { value: 'extract', label: 'Extract' },
  { value: 'partial_mash', label: 'Partial Mash' },
  { value: 'cider', label: 'Cider' },
  { value: 'mead', label: 'Mead' },
  { value: 'other', label: 'Other' },
]

const FERMENTABLE_UNITS: { value: 'kg' | 'g'; label: string }[] = [
  { value: 'kg', label: 'kg' },
  { value: 'g', label: 'g' },
]

const HOP_UNITS: { value: 'g' | 'kg'; label: string }[] = [
  { value: 'g', label: 'g' },
  { value: 'kg', label: 'kg' },
]

const YEAST_UNITS: { value: 'g' | 'mL' | 'count'; label: string }[] = [
  { value: 'g', label: 'g' },
  { value: 'mL', label: 'mL' },
  { value: 'count', label: 'count' },
]

const MASH_STEP_TYPES: { value: 'infusion' | 'temperature' | 'decoction'; label: string }[] = [
  { value: 'infusion', label: 'Infusion' },
  { value: 'temperature', label: 'Temperature' },
  { value: 'decoction', label: 'Decoction' },
]

const HOP_FORMS: { value: 'pellet' | 'leaf' | 'extract'; label: string }[] = [
  { value: 'pellet', label: 'Pellet' },
  { value: 'leaf', label: 'Leaf' },
  { value: 'extract', label: 'Extract' },
]

const HOP_USES: { value: 'boil' | 'whirlpool' | 'dry-hop' | 'first-wort' | 'mash'; label: string }[] = [
  { value: 'boil', label: 'Boil' },
  { value: 'whirlpool', label: 'Whirlpool' },
  { value: 'dry-hop', label: 'Dry Hop' },
  { value: 'first-wort', label: 'First Wort' },
  { value: 'mash', label: 'Mash' },
]

export default function RecipeEditorPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const isEditMode = !!id
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

  // Array states
  const [fermentables, setFermentables] = useState<Fermentable[]>([])
  const [hops, setHops] = useState<Hop[]>([])
  const [yeasts, setYeasts] = useState<Yeast[]>([])
  const [mashSteps, setMashSteps] = useState<MashStep[]>([])

  // Computed values from API response
  const [calcOg, setCalcOg] = useState<number | null>(null)
  const [calcFg, setCalcFg] = useState<number | null>(null)
  const [calcAbvPct, setCalcAbvPct] = useState<number | null>(null)
  const [calcIbu, setCalcIbu] = useState<number | null>(null)
  const [calcColorEbc, setCalcColorEbc] = useState<number | null>(null)

  // Load recipe data in edit mode
  const { data: recipeData, isLoading, isError: isLoadError, error: loadError } = useRecipe(id || '')

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
      attenuation_pct: yeasts[0]?.attenuation_pct ?? null,
      fermentables: fermentables.map((f) => ({
        amount: f.amount,
        unit: f.unit,
        potential_ppg: f.potential_ppg ?? null,
        color_ebc: f.color_ebc ?? null,
      })),
      hops: hops.map((h) => ({
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
  }, [physics.ready, batchSizeLiters, efficiencyPct, fermentables, hops, yeasts])

  // Populate state from loaded recipe
  useEffect(() => {
    if (!recipeData) return

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

    // Map fermentables
    if (recipeData.fermentables) {
      const mapped = recipeData.fermentables.map((f) => ({
        step_order: f.step_order ?? 1,
        name: f.name ?? '',
        amount: f.amount ?? 0,
        unit: (f.unit as 'kg' | 'g') ?? 'kg',
        color_ebc: f.color_ebc ?? undefined,
        potential_ppg: f.potential_ppg ?? undefined,
        type: f.type ?? undefined,
        addition: f.addition ?? undefined,
      }))
      setFermentables(mapped.length > 0 ? mapped : [{ step_order: 1, name: '', amount: 0, unit: 'kg' }])
    }

    // Map hops
    if (recipeData.hops) {
      const mapped = recipeData.hops.map((h) => ({
        step_order: h.step_order ?? 1,
        name: h.name ?? '',
        amount: h.amount ?? 0,
        unit: (h.unit as 'g' | 'kg') ?? 'g',
        alpha_acid_pct: h.alpha_acid_pct ?? 0,
        boil_time_minutes: h.boil_time_minutes ?? 0,
        form: (h.form ?? undefined) as Hop['form'],
        use: (h.use ?? undefined) as Hop['use'],
      }))
      setHops(mapped.length > 0 ? mapped : [])
    }

    // Map yeasts
    if (recipeData.yeasts) {
      const mapped = recipeData.yeasts.map((y) => ({
        name: y.name ?? '',
        amount: y.amount ?? 0,
        unit: (y.unit as 'g' | 'mL' | 'count') ?? 'g',
        attenuation_pct: y.attenuation_pct ?? undefined,
      }))
      setYeasts(mapped.length > 0 ? mapped : [])
    }

    // Map mash steps
    if (recipeData.mash_steps) {
      const mapped = recipeData.mash_steps.map((m) => ({
        step_order: m.step_order ?? 1,
        step_type: (m.step_type as 'infusion' | 'temperature' | 'decoction') ?? 'infusion',
        target_temp_c: m.target_temp_c ?? 0,
        hold_minutes: m.hold_minutes ?? 0,
        infusion_volume_liters: m.infusion_volume_liters ?? undefined,
      }))
      setMashSteps(mapped.length > 0 ? mapped : [])
    }
  }, [recipeData])

  // Initialize empty arrays for create mode
  useEffect(() => {
    if (!isEditMode && fermentables.length === 0) {
      setFermentables([{ step_order: 1, name: '', amount: 0, unit: 'kg' }])
    }
  }, [isEditMode, fermentables.length])

  // Helper functions for arrays
  const addFermentable = () => {
    const newOrder = fermentables.length > 0 ? Math.max(...fermentables.map((f) => f.step_order)) + 1 : 1
    setFermentables([...fermentables, { step_order: newOrder, name: '', amount: 0, unit: 'kg' }])
  }

  const removeFermentable = (index: number) => {
    const newArray = [...fermentables]
    newArray.splice(index, 1)
    setFermentables(newArray)
  }

  const updateFermentable = (index: number, field: keyof Fermentable, value: string | number) => {
    const newArray = [...fermentables]
    newArray[index] = { ...newArray[index], [field]: typeof value === 'string' ? (value === '' ? 0 : Number(value)) : value }
    setFermentables(newArray)
  }

  const addHop = () => {
    const newOrder = hops.length > 0 ? Math.max(...hops.map((h) => h.step_order)) + 1 : 1
    setHops([...hops, { step_order: newOrder, name: '', amount: 0, unit: 'g', alpha_acid_pct: 0, boil_time_minutes: 0 }])
  }

  const removeHop = (index: number) => {
    const newArray = [...hops]
    newArray.splice(index, 1)
    setHops(newArray)
  }

  const updateHop = (index: number, field: keyof Hop, value: string | number) => {
    const newArray = [...hops]
    newArray[index] = { ...newArray[index], [field]: typeof value === 'string' ? (value === '' ? 0 : Number(value)) : value }
    setHops(newArray)
  }

  const addYeast = () => {
    setYeasts([...yeasts, { name: '', amount: 0, unit: 'g' }])
  }

  const removeYeast = (index: number) => {
    const newArray = [...yeasts]
    newArray.splice(index, 1)
    setYeasts(newArray)
  }

  const updateYeast = (index: number, field: keyof Yeast, value: string | number) => {
    const newArray = [...yeasts]
    newArray[index] = { ...newArray[index], [field]: typeof value === 'string' ? (value === '' ? 0 : Number(value)) : value }
    setYeasts(newArray)
  }

  const addMashStep = () => {
    const newOrder = mashSteps.length > 0 ? Math.max(...mashSteps.map((m) => m.step_order)) + 1 : 1
    setMashSteps([...mashSteps, { step_order: newOrder, step_type: 'infusion', target_temp_c: 0, hold_minutes: 0 }])
  }

  const removeMashStep = (index: number) => {
    const newArray = [...mashSteps]
    newArray.splice(index, 1)
    setMashSteps(newArray)
  }

  const updateMashStep = (index: number, field: keyof MashStep, value: string | number) => {
    const newArray = [...mashSteps]
    newArray[index] = { ...newArray[index], [field]: typeof value === 'string' ? (value === '' ? 0 : Number(value)) : value }
    setMashSteps(newArray)
  }

  const handleSave = () => {
    const payload: components['schemas']['CreateRecipeRequest'] | components['schemas']['PatchRecipeRequest'] = {
      name,
      type,
      batch_size_liters: batchSizeLiters,
      boil_size_liters: boilSizeLiters ?? undefined,
      boil_time_minutes: boilTimeMinutes ?? undefined,
      efficiency_pct: efficiencyPct ?? undefined,
      notes: notes || undefined,
      fermentables: fermentables.length > 0 ? fermentables : undefined,
      hops: hops.length > 0 ? hops : undefined,
      yeasts: yeasts.length > 0 ? yeasts : undefined,
      mash_steps: mashSteps.length > 0 ? mashSteps : undefined,
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

  // Render helpers
  const inputCls = "border border-[var(--color-border)] rounded px-2 py-1 bg-[var(--color-surface)] text-[var(--color-fg)]"
  const renderFermentableRow = (f: Fermentable, index: number) => (
    <tr key={index} className="border-t border-[var(--color-border)]">
      <td className="px-3 py-2">
        <input
          type="number"
          value={f.step_order}
          onChange={(e) => updateFermentable(index, 'step_order', e.target.value)}
          className={`${inputCls} w-16`}
        />
      </td>
      <td className="px-3 py-2">
        <input
          type="text"
          value={f.name}
          onChange={(e) => updateFermentable(index, 'name', e.target.value)}
          placeholder="Name"
          className={`${inputCls} w-40`}
        />
      </td>
      <td className="px-3 py-2">
        <input
          type="number"
          value={f.amount}
          onChange={(e) => updateFermentable(index, 'amount', e.target.value)}
          className={`${inputCls} w-20`}
        />
      </td>
      <td className="px-3 py-2">
        <select
          value={f.unit}
          onChange={(e) => updateFermentable(index, 'unit', e.target.value)}
          className={`${inputCls} w-20`}
        >
          {FERMENTABLE_UNITS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </td>
      <td className="px-3 py-2">
        <input
          type="number"
          value={f.color_ebc ?? ''}
          onChange={(e) => updateFermentable(index, 'color_ebc', e.target.value)}
          placeholder="EBC"
          className={`${inputCls} w-20`}
        />
      </td>
      <td className="px-3 py-2">
        <input
          type="number"
          value={f.potential_ppg ?? ''}
          onChange={(e) => updateFermentable(index, 'potential_ppg', e.target.value)}
          placeholder="PPG"
          className={`${inputCls} w-20`}
        />
      </td>
      <td className="px-3 py-2">
        <input
          type="text"
          value={f.type ?? ''}
          onChange={(e) => updateFermentable(index, 'type', e.target.value)}
          placeholder="Type"
          className={`${inputCls} w-24`}
        />
      </td>
      <td className="px-3 py-2">
        <input
          type="text"
          value={f.addition ?? ''}
          onChange={(e) => updateFermentable(index, 'addition', e.target.value)}
          placeholder="Addition"
          className={`${inputCls} w-24`}
        />
      </td>
      <td className="px-3 py-2">
        <button onClick={() => removeFermentable(index)} className="text-red-600 hover:underline">
          Remove
        </button>
      </td>
    </tr>
  )

  const renderHopRow = (h: Hop, index: number) => (
    <tr key={index} className="border-t border-[var(--color-border)]">
      <td className="px-3 py-2">
        <input
          type="number"
          value={h.step_order}
          onChange={(e) => updateHop(index, 'step_order', e.target.value)}
          className={`${inputCls} w-16`}
        />
      </td>
      <td className="px-3 py-2">
        <input
          type="text"
          value={h.name}
          onChange={(e) => updateHop(index, 'name', e.target.value)}
          placeholder="Name"
          className={`${inputCls} w-40`}
        />
      </td>
      <td className="px-3 py-2">
        <input
          type="number"
          value={h.amount}
          onChange={(e) => updateHop(index, 'amount', e.target.value)}
          className={`${inputCls} w-20`}
        />
      </td>
      <td className="px-3 py-2">
        <select
          value={h.unit}
          onChange={(e) => updateHop(index, 'unit', e.target.value)}
          className={`${inputCls} w-20`}
        >
          {HOP_UNITS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </td>
      <td className="px-3 py-2">
        <input
          type="number"
          value={h.alpha_acid_pct}
          onChange={(e) => updateHop(index, 'alpha_acid_pct', e.target.value)}
          placeholder="AA%"
          className={`${inputCls} w-20`}
        />
      </td>
      <td className="px-3 py-2">
        <input
          type="number"
          value={h.boil_time_minutes}
          onChange={(e) => updateHop(index, 'boil_time_minutes', e.target.value)}
          placeholder="Minutes"
          className={`${inputCls} w-20`}
        />
      </td>
      <td className="px-3 py-2">
        <select
          value={h.form ?? ''}
          onChange={(e) => updateHop(index, 'form', e.target.value)}
          className={`${inputCls} w-24`}
        >
          <option value="">- Form -</option>
          {HOP_FORMS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </td>
      <td className="px-3 py-2">
        <select
          value={h.use ?? ''}
          onChange={(e) => updateHop(index, 'use', e.target.value)}
          className={`${inputCls} w-24`}
        >
          <option value="">- Use -</option>
          {HOP_USES.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </td>
      <td className="px-3 py-2 text-sm text-[var(--color-muted)] tabular-nums text-right">
        {(() => {
          const ibu = calcHopIBU(
            ibuMethod,
            h.unit === 'kg' ? h.amount * 1000 : h.amount,
            h.alpha_acid_pct,
            h.boil_time_minutes,
            batchSizeLiters || 20,
            calcOg ?? 1.050,
          )
          return ibu > 0 ? ibu.toFixed(1) : '—'
        })()}
      </td>
      <td className="px-3 py-2">
        <button onClick={() => removeHop(index)} className="text-red-600 hover:underline">
          Remove
        </button>
      </td>
    </tr>
  )

  const renderYeastRow = (y: Yeast, index: number) => (
    <tr key={index} className="border-t border-[var(--color-border)]">
      <td className="px-3 py-2">
        <input
          type="text"
          value={y.name}
          onChange={(e) => updateYeast(index, 'name', e.target.value)}
          placeholder="Name"
          className={`${inputCls} w-40`}
        />
      </td>
      <td className="px-3 py-2">
        <input
          type="number"
          value={y.amount}
          onChange={(e) => updateYeast(index, 'amount', e.target.value)}
          className={`${inputCls} w-20`}
        />
      </td>
      <td className="px-3 py-2">
        <select
          value={y.unit}
          onChange={(e) => updateYeast(index, 'unit', e.target.value)}
          className={`${inputCls} w-24`}
        >
          {YEAST_UNITS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </td>
      <td className="px-3 py-2">
        <input
          type="number"
          value={y.attenuation_pct ?? ''}
          onChange={(e) => updateYeast(index, 'attenuation_pct', e.target.value)}
          placeholder="Atten %"
          className={`${inputCls} w-20`}
        />
      </td>
      <td className="px-3 py-2">
        <button onClick={() => removeYeast(index)} className="text-red-600 hover:underline">
          Remove
        </button>
      </td>
    </tr>
  )

  const renderMashStepRow = (m: MashStep, index: number) => (
    <tr key={index} className="border-t border-[var(--color-border)]">
      <td className="px-3 py-2">
        <input
          type="number"
          value={m.step_order}
          onChange={(e) => updateMashStep(index, 'step_order', e.target.value)}
          className={`${inputCls} w-16`}
        />
      </td>
      <td className="px-3 py-2">
        <select
          value={m.step_type}
          onChange={(e) => updateMashStep(index, 'step_type', e.target.value)}
          className={`${inputCls} w-24`}
        >
          {MASH_STEP_TYPES.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </td>
      <td className="px-3 py-2">
        <input
          type="number"
          value={m.target_temp_c}
          onChange={(e) => updateMashStep(index, 'target_temp_c', e.target.value)}
          placeholder="Temp C"
          className={`${inputCls} w-20`}
        />
      </td>
      <td className="px-3 py-2">
        <input
          type="number"
          value={m.hold_minutes}
          onChange={(e) => updateMashStep(index, 'hold_minutes', e.target.value)}
          placeholder="Minutes"
          className={`${inputCls} w-20`}
        />
      </td>
      <td className="px-3 py-2">
        <input
          type="number"
          value={m.infusion_volume_liters ?? ''}
          onChange={(e) => updateMashStep(index, 'infusion_volume_liters', e.target.value)}
          placeholder="Volume L"
          className={`${inputCls} w-20`}
        />
      </td>
      <td className="px-3 py-2">
        <button onClick={() => removeMashStep(index)} className="text-red-600 hover:underline">
          Remove
        </button>
      </td>
    </tr>
  )

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
    <div className="p-4">
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
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              <div>
                <label className="block text-sm font-medium text-[var(--color-fg)] mb-1">Name *</label>
                <input
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="Recipe name"
                  className="border border-[var(--color-border)] rounded px-3 py-2 w-full bg-[var(--color-surface)] text-[var(--color-fg)]"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-[var(--color-fg)] mb-1">Type</label>
                <select
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
                <label className="block text-sm font-medium text-[var(--color-fg)] mb-1">Batch Size (L) *</label>
                <input
                  type="number"
                  value={batchSizeLiters}
                  onChange={(e) => setBatchSizeLiters(Number(e.target.value))}
                  className="border border-[var(--color-border)] rounded px-3 py-2 w-full bg-[var(--color-surface)] text-[var(--color-fg)]"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-[var(--color-fg)] mb-1">Boil Size (L)</label>
                <input
                  type="number"
                  value={boilSizeLiters ?? ''}
                  onChange={(e) => setBoilSizeLiters(e.target.value === '' ? null : Number(e.target.value))}
                  placeholder="Boil size"
                  className="border border-[var(--color-border)] rounded px-3 py-2 w-full bg-[var(--color-surface)] text-[var(--color-fg)]"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-[var(--color-fg)] mb-1">Boil Time (min)</label>
                <input
                  type="number"
                  value={boilTimeMinutes ?? ''}
                  onChange={(e) => setBoilTimeMinutes(e.target.value === '' ? null : Number(e.target.value))}
                  placeholder="Boil time"
                  className="border border-[var(--color-border)] rounded px-3 py-2 w-full bg-[var(--color-surface)] text-[var(--color-fg)]"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-[var(--color-fg)] mb-1">Efficiency %</label>
                <input
                  type="number"
                  value={efficiencyPct ?? ''}
                  onChange={(e) => setEfficiencyPct(e.target.value === '' ? null : Number(e.target.value))}
                  placeholder="Efficiency"
                  className="border border-[var(--color-border)] rounded px-3 py-2 w-full bg-[var(--color-surface)] text-[var(--color-fg)]"
                />
              </div>
              <div className="md:col-span-3">
                <label className="block text-sm font-medium text-[var(--color-fg)] mb-1">Notes</label>
                <textarea
                  value={notes}
                  onChange={(e) => setNotes(e.target.value)}
                  placeholder="Additional notes..."
                  rows={4}
                  className="border border-[var(--color-border)] rounded px-3 py-2 w-full bg-[var(--color-surface)] text-[var(--color-fg)]"
                />
              </div>
            </div>
          </div>

          {/* Fermentables */}
          <div className="bg-[var(--color-surface)] p-6 rounded shadow mb-4">
            <div className="flex justify-between items-center mb-4">
              <h2 className="text-lg font-semibold text-[var(--color-fg)]">Fermentables</h2>
              <button onClick={addFermentable} className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700">
                Add Fermentable
              </button>
            </div>
            <div className="overflow-x-auto">
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
                <tbody>{fermentables.map(renderFermentableRow)}</tbody>
              </table>
            </div>
          </div>

          {/* Hops */}
          <div className="bg-[var(--color-surface)] p-6 rounded shadow mb-4">
            <div className="flex justify-between items-center mb-4">
              <h2 className="text-lg font-semibold text-[var(--color-fg)]">Hops</h2>
              <button onClick={addHop} className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700">
                Add Hop
              </button>
            </div>
            <div className="overflow-x-auto">
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
                <tbody>{hops.map(renderHopRow)}</tbody>
              </table>
            </div>
          </div>

          {/* Yeasts */}
          <div className="bg-[var(--color-surface)] p-6 rounded shadow mb-4">
            <div className="flex justify-between items-center mb-4">
              <h2 className="text-lg font-semibold text-[var(--color-fg)]">Yeasts</h2>
              <button onClick={addYeast} className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700">
                Add Yeast
              </button>
            </div>
            <div className="overflow-x-auto">
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
                <tbody>{yeasts.map(renderYeastRow)}</tbody>
              </table>
            </div>
          </div>

          {/* Mash Steps */}
          <div className="bg-[var(--color-surface)] p-6 rounded shadow mb-4">
            <div className="flex justify-between items-center mb-4">
              <h2 className="text-lg font-semibold text-[var(--color-fg)]">Mash Steps</h2>
              <button onClick={addMashStep} className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700">
                Add Mash Step
              </button>
            </div>
            <div className="overflow-x-auto">
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
                <tbody>{mashSteps.map(renderMashStepRow)}</tbody>
              </table>
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
                <RecipeWaterChemistry recipeId={id} fermentables={fermentables} />
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
                  <label className="block text-sm font-medium text-[var(--color-muted)] mb-1">OG</label>
                  <div className="border border-[var(--color-border)] rounded px-3 py-2 bg-[var(--color-bg)] text-[var(--color-fg)] tabular-nums">{liveCalcs.calcOg.toFixed(3)}</div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-[var(--color-muted)] mb-1">FG</label>
                  <div className="border border-[var(--color-border)] rounded px-3 py-2 bg-[var(--color-bg)] text-[var(--color-fg)] tabular-nums">{liveCalcs.calcFg.toFixed(3)}</div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-[var(--color-muted)] mb-1">ABV %</label>
                  <div className="border border-[var(--color-border)] rounded px-3 py-2 bg-[var(--color-bg)] text-[var(--color-fg)] tabular-nums">{liveCalcs.calcAbvPct.toFixed(1)}%</div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-[var(--color-muted)] mb-1">IBU</label>
                  <div className="border border-[var(--color-border)] rounded px-3 py-2 bg-[var(--color-bg)] text-[var(--color-fg)] tabular-nums">{liveCalcs.calcIbu.toFixed(1)}</div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-[var(--color-muted)] mb-1">Color (EBC)</label>
                  <div className="border border-[var(--color-border)] rounded px-3 py-2 bg-[var(--color-bg)] text-[var(--color-fg)] flex items-center gap-2 tabular-nums">
                    <EBCSwatch ebc={liveCalcs.calcColorEbc} />
                    {liveCalcs.calcColorEbc.toFixed(0)}
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
                  <label className="block text-sm font-medium text-[var(--color-muted)] mb-1">OG</label>
                  <div className="border border-[var(--color-border)] rounded px-3 py-2 bg-[var(--color-bg)] text-[var(--color-fg)]">{calcOg?.toFixed(3) ?? '-'}</div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-[var(--color-muted)] mb-1">FG</label>
                  <div className="border border-[var(--color-border)] rounded px-3 py-2 bg-[var(--color-bg)] text-[var(--color-fg)]">{calcFg?.toFixed(3) ?? '-'}</div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-[var(--color-muted)] mb-1">ABV %</label>
                  <div className="border border-[var(--color-border)] rounded px-3 py-2 bg-[var(--color-bg)] text-[var(--color-fg)]">{calcAbvPct?.toFixed(1) ?? '-'}%</div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-[var(--color-muted)] mb-1">IBU</label>
                  <div className="border border-[var(--color-border)] rounded px-3 py-2 bg-[var(--color-bg)] text-[var(--color-fg)]">{calcIbu?.toFixed(1) ?? '-'}</div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-[var(--color-muted)] mb-1">Color (EBC)</label>
                  <div className="border border-[var(--color-border)] rounded px-3 py-2 bg-[var(--color-bg)] text-[var(--color-fg)] flex items-center gap-2">{calcColorEbc != null && <EBCSwatch ebc={calcColorEbc} />}{calcColorEbc?.toFixed(0) ?? '-'}</div>
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
            <button
              onClick={handleSave}
              disabled={!name || !batchSizeLiters || isSaving}
              className="bg-green-600 text-white px-6 py-2 rounded hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isSaving ? 'Saving...' : 'Save'}
            </button>
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
