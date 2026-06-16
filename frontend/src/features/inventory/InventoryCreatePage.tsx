import React from 'react'
import { useNavigate } from 'react-router-dom'
import { useInventoryCreate } from './hooks/useInventory'
import { APIError } from '../../api/error'

const INGREDIENT_TYPES = ['fermentable', 'hop', 'yeast', 'adjunct', 'chemical', 'other'] as const
const UNITS = ['kg', 'g', 'L', 'mL', 'count'] as const

export function InventoryCreatePage() {
  const navigate = useNavigate()
  const { mutate: createLot, isPending, isError, error } = useInventoryCreate()

  const [type, setType] = React.useState<typeof INGREDIENT_TYPES[number]>('fermentable')
  const [name, setName] = React.useState<string>('')
  const [amount, setAmount] = React.useState<number | ''>('')
  const [unit, setUnit] = React.useState<typeof UNITS[number]>('kg')
  const [lotNumber, setLotNumber] = React.useState<string>('')
  const [bestBeforeDate, setBestBeforeDate] = React.useState<string>('')
  const [costPence, setCostPence] = React.useState<number | ''>('')
  const [supplier, setSupplier] = React.useState<string>('')
  const [notes, setNotes] = React.useState<string>('')
  const [allergens, setAllergens] = React.useState<string[]>([])
  const [colorEbc, setColorEbc] = React.useState<number | ''>('')
  const [alphaAcidPct, setAlphaAcidPct] = React.useState<number | ''>('')
  const [attenuationPct, setAttenuationPct] = React.useState<number | ''>('')

  const handleSave = () => {
    if (!name || !lotNumber || amount === '' || !type || !unit) return

    createLot({
      type,
      name,
      amount: Number(amount),
      unit,
      lot_number: lotNumber,
      best_before_date: bestBeforeDate || undefined,
      cost_pence: costPence === '' ? undefined : Number(costPence),
      supplier: supplier || undefined,
      notes: notes || undefined,
      allergens: allergens.length > 0 ? allergens : undefined,
      color_ebc: colorEbc === '' ? undefined : Number(colorEbc),
      alpha_acid_pct: alphaAcidPct === '' ? undefined : Number(alphaAcidPct),
      attenuation_pct: attenuationPct === '' ? undefined : Number(attenuationPct),
    }, {
      onSuccess: () => navigate('/inventory'),
    })
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-bold text-[var(--color-fg)]">New Lot</h1>
        <button
          onClick={() => navigate('/inventory')}
          className="text-sm text-[var(--color-muted)] hover:text-[var(--color-fg)]"
        >
          Cancel
        </button>
      </div>

      {isError && (
        <div className="mb-4 p-4 rounded border border-[var(--color-danger)] bg-red-50 text-[var(--color-danger)]">
          <p className="font-semibold">Failed to create lot.</p>
          <p className="text-sm mt-1">
            {error instanceof APIError ? error.message : error instanceof Error ? error.message : 'Unknown error'}
          </p>
        </div>
      )}

      <div className="space-y-4 max-w-2xl">
        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">
            Type <span className="text-[var(--color-danger)]">*</span>
          </label>
          <select
            value={type}
            onChange={(e) => setType(e.target.value as typeof INGREDIENT_TYPES[number])}
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
          >
            {INGREDIENT_TYPES.map((t) => (
              <option key={t} value={t}>{t}</option>
            ))}
          </select>
        </div>

        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">
            Name <span className="text-[var(--color-danger)]">*</span>
          </label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Ingredient name"
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
          />
        </div>

        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">
            Lot Number <span className="text-[var(--color-danger)]">*</span>
          </label>
          <input
            type="text"
            value={lotNumber}
            onChange={(e) => setLotNumber(e.target.value)}
            placeholder="Lot number"
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
          />
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="flex flex-col gap-1">
            <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">
              Amount <span className="text-[var(--color-danger)]">*</span>
            </label>
            <input
              type="number"
              value={amount}
              onChange={(e) => setAmount(e.target.value === '' ? '' : Number(e.target.value))}
              placeholder="Amount"
              min="0"
              step="0.001"
              className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">
              Unit <span className="text-[var(--color-danger)]">*</span>
            </label>
            <select
              value={unit}
              onChange={(e) => setUnit(e.target.value as typeof UNITS[number])}
              className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
            >
              {UNITS.map((u) => (
                <option key={u} value={u}>{u}</option>
              ))}
            </select>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="flex flex-col gap-1">
            <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">
              Best Before Date
            </label>
            <input
              type="date"
              value={bestBeforeDate}
              onChange={(e) => setBestBeforeDate(e.target.value)}
              className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">
              Cost (pence)
            </label>
            <input
              type="number"
              value={costPence}
              onChange={(e) => setCostPence(e.target.value === '' ? '' : Number(e.target.value))}
              placeholder="Cost in pence"
              min="0"
              className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
            />
          </div>
        </div>

        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Supplier</label>
          <input
            type="text"
            value={supplier}
            onChange={(e) => setSupplier(e.target.value)}
            placeholder="Supplier name"
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
          />
        </div>

        {type === 'fermentable' && (
          <div className="flex flex-col gap-1">
            <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Colour (EBC)</label>
            <input
              type="number"
              value={colorEbc}
              onChange={(e) => setColorEbc(e.target.value === '' ? '' : Number(e.target.value))}
              placeholder="e.g. 5.5"
              min="0"
              step="0.1"
              className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
            />
          </div>
        )}

        {type === 'hop' && (
          <div className="flex flex-col gap-1">
            <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Alpha Acid %</label>
            <input
              type="number"
              value={alphaAcidPct}
              onChange={(e) => setAlphaAcidPct(e.target.value === '' ? '' : Number(e.target.value))}
              placeholder="e.g. 12.8"
              min="0"
              max="100"
              step="0.1"
              className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
            />
          </div>
        )}

        {type === 'yeast' && (
          <div className="flex flex-col gap-1">
            <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Attenuation %</label>
            <input
              type="number"
              value={attenuationPct}
              onChange={(e) => setAttenuationPct(e.target.value === '' ? '' : Number(e.target.value))}
              placeholder="e.g. 75"
              min="0"
              max="100"
              step="0.1"
              className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
            />
          </div>
        )}

        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Allergens</label>
          <input
            type="text"
            value={allergens.join(', ')}
            onChange={(e) => setAllergens(e.target.value.split(',').map(s => s.trim()).filter(Boolean))}
            placeholder="Comma-separated allergens"
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
          />
        </div>

        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Notes</label>
          <textarea
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            placeholder="Additional notes"
            rows={3}
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
          />
        </div>

        <button
          onClick={handleSave}
          disabled={isPending || !name || !lotNumber || amount === ''}
          className="px-6 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50 self-start"
        >
          {isPending ? 'Saving...' : 'Save'}
        </button>
      </div>
    </div>
  )
}
