import React from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import {
  useInventoryItem,
  useInventoryUpdate,
  useInventoryDelete,
  useStockIn,
} from './hooks/useInventory'
import { APIError } from '../../api/error'

const INGREDIENT_TYPES = ['fermentable', 'hop', 'yeast', 'adjunct', 'chemical', 'other'] as const
const UNITS = ['kg', 'g', 'L', 'mL', 'count'] as const

export function InventoryDetailPage() {
  const navigate = useNavigate()
  const { id } = useParams<{ id: string }>()

  const { data: item, isLoading, isError: isLoadError, error: loadError, refetch } = useInventoryItem(id ?? '')
  const { mutate: updateLot, isPending: isUpdating, isError: isUpdateError, error: updateError } = useInventoryUpdate(id ?? '')
  const { mutate: deleteLot, isPending: isDeleting } = useInventoryDelete(id ?? '')
  const { mutate: stockIn, isPending: isStockingIn, isError: isStockInError, error: stockInError } = useStockIn(id ?? '')

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

  const [stockInAmount, setStockInAmount] = React.useState<number | ''>('')

  React.useEffect(() => {
    if (item) {
      setType(item.type)
      setName(item.name)
      setAmount(item.amount)
      setUnit(item.unit)
      setLotNumber(item.lot_number)
      setBestBeforeDate(item.best_before_date ?? '')
      setCostPence(item.cost_pence)
      setSupplier(item.supplier ?? '')
      setNotes(item.notes ?? '')
      setAllergens(item.allergens ?? [])
      setColorEbc(item.color_ebc ?? '')
      setAlphaAcidPct(item.alpha_acid_pct ?? '')
      setAttenuationPct(item.attenuation_pct ?? '')
    }
  }, [item])

  const handleSave = () => {
    if (!id || !name || !lotNumber || amount === '' || !type || !unit) return

    updateLot({
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
      onSuccess: () => refetch(),
    })
  }

  const handleStockIn = () => {
    if (!id || stockInAmount === '' || stockInAmount <= 0) return

    stockIn({
      amount: Number(stockInAmount),
      notes: undefined,
      cost_pence: undefined,
    }, {
      onSuccess: () => {
        setStockInAmount('')
        refetch()
      },
    })
  }

  const handleDelete = () => {
    if (!id) return
    if (window.confirm('Are you sure you want to delete this lot? This action cannot be undone.')) {
      deleteLot(undefined, {
        onSuccess: () => navigate('/inventory'),
      })
    }
  }

  if (isLoadError) {
    return (
      <div className="p-4 rounded border border-[var(--color-danger)] bg-red-50 text-[var(--color-danger)]">
        <p className="font-semibold">Failed to load lot.</p>
        <p className="text-sm mt-1">
          {loadError instanceof APIError ? loadError.message : loadError instanceof Error ? loadError.message : 'Unknown error'}
        </p>
        <button
          onClick={() => refetch()}
          className="mt-3 px-3 py-1 text-sm rounded bg-[var(--color-danger)] text-white"
        >
          Retry
        </button>
      </div>
    )
  }

  if (isLoading) {
    return (
      <div className="animate-pulse space-y-4 max-w-2xl">
        <div className="h-8 w-48 rounded" style={{ background: 'var(--color-border)' }} />
        <div className="grid grid-cols-2 gap-4">
          <div className="h-10 w-full rounded" style={{ background: 'var(--color-border)' }} />
          <div className="h-10 w-full rounded" style={{ background: 'var(--color-border)' }} />
        </div>
        <div className="h-10 w-full rounded" style={{ background: 'var(--color-border)' }} />
        <div className="h-10 w-full rounded" style={{ background: 'var(--color-border)' }} />
        <div className="h-20 w-full rounded" style={{ background: 'var(--color-border)' }} />
      </div>
    )
  }

  if (!item) {
    return (
      <div className="p-4 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-muted)]">
        Lot not found.
      </div>
    )
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-bold text-[var(--color-fg)]">Lot: {item.lot_number}</h1>
        <button
          onClick={() => navigate('/inventory')}
          className="text-sm text-[var(--color-muted)] hover:text-[var(--color-fg)]"
        >
          Back to inventory
        </button>
      </div>

      {(isUpdateError || isStockInError) && (
        <div className="mb-4 p-4 rounded border border-[var(--color-danger)] bg-red-50 text-[var(--color-danger)]">
          <p className="font-semibold">Error</p>
          <p className="text-sm mt-1">
            {updateError instanceof APIError ? updateError.message : updateError instanceof Error ? updateError.message : ''}
            {stockInError instanceof APIError ? stockInError.message : stockInError instanceof Error ? stockInError.message : ''}
          </p>
        </div>
      )}

      <div className="space-y-4 max-w-2xl mb-8">
        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Type</label>
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
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Name</label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
          />
        </div>

        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Lot Number</label>
          <input
            type="text"
            value={lotNumber}
            onChange={(e) => setLotNumber(e.target.value)}
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
          />
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="flex flex-col gap-1">
            <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Amount</label>
            <input
              type="number"
              value={amount}
              onChange={(e) => setAmount(e.target.value === '' ? '' : Number(e.target.value))}
              min="0"
              step="0.001"
              className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Unit</label>
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
            <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Best Before Date</label>
            <input
              type="date"
              value={bestBeforeDate}
              onChange={(e) => setBestBeforeDate(e.target.value)}
              className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Cost (pence)</label>
            <input
              type="number"
              value={costPence}
              onChange={(e) => setCostPence(e.target.value === '' ? '' : Number(e.target.value))}
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
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
          />
        </div>

        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Notes</label>
          <textarea
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            rows={3}
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
          />
        </div>

        <div className="flex gap-2">
          <button
            onClick={handleSave}
            disabled={isUpdating || !name || !lotNumber || amount === ''}
            className="px-6 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
          >
            {isUpdating ? 'Saving...' : 'Save'}
          </button>
          <button
            onClick={handleDelete}
            disabled={isDeleting}
            className="px-6 py-2 rounded text-sm bg-[var(--color-danger)] text-white hover:opacity-90 disabled:opacity-50"
          >
            {isDeleting ? 'Deleting...' : 'Delete lot'}
          </button>
        </div>
      </div>

      <div className="border-t pt-6 mt-8" style={{ borderColor: 'var(--color-border)' }}>
        <h2 className="text-lg font-semibold text-[var(--color-fg)] mb-4">Stock In</h2>
        <div className="flex items-center gap-4 max-w-md">
          <div className="flex-1 flex flex-col gap-1">
            <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Amount</label>
            <input
              type="number"
              value={stockInAmount}
              onChange={(e) => setStockInAmount(e.target.value === '' ? '' : Number(e.target.value))}
              placeholder="Amount to add"
              min="0"
              step="0.001"
              className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
            />
          </div>
          <button
            onClick={handleStockIn}
            disabled={isStockingIn || stockInAmount === '' || stockInAmount <= 0}
            className="px-6 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50 self-end"
          >
            {isStockingIn ? 'Adding...' : 'Add stock'}
          </button>
        </div>
      </div>
    </div>
  )
}
