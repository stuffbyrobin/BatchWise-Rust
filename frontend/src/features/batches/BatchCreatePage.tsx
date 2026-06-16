import React from 'react'
import { useNavigate } from 'react-router-dom'
import { useCreateBatch } from './hooks/useBatches'
import { useRecipesList } from '../recipes/hooks/useRecipes'
import { useTenant } from '../account/hooks/useTenant'
import { APIError } from '../../api/error'

export function BatchCreatePage() {
  const navigate = useNavigate()
  const { mutate: createBatch, isPending, isError, error } = useCreateBatch()
  const { data: recipesData } = useRecipesList({ page_size: 100 })
  const { data: tenant } = useTenant()

  const [recipeId, setRecipeId] = React.useState('')
  const [batchNumber, setBatchNumber] = React.useState('')

  React.useEffect(() => {
    if (tenant?.next_batch_number != null && batchNumber === '') {
      setBatchNumber(String(tenant.next_batch_number))
    }
  }, [tenant?.next_batch_number])
  const [name, setName] = React.useState('')
  const [brewDate, setBrewDate] = React.useState('')
  const [notes, setNotes] = React.useState('')

  const handleRecipeChange = (id: string) => {
    setRecipeId(id)
    if (id) {
      const recipe = recipesData?.items.find((r) => r.id === id)
      if (recipe?.name) setName(recipe.name)
    }
  }

  const handleSave = () => {
    if (!recipeId || !batchNumber || !name) return
    createBatch(
      {
        recipe_id: recipeId,
        batch_number: batchNumber,
        name,
        brew_date: brewDate || undefined,
        notes: notes || undefined,
      },
      {
        onSuccess: (res) => navigate(`/batches/${res.batch?.id}`),
      },
    )
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-bold text-[var(--color-fg)]">New Batch</h1>
        <button
          onClick={() => navigate('/batches')}
          className="text-sm text-[var(--color-muted)] hover:text-[var(--color-fg)]"
        >
          Cancel
        </button>
      </div>

      {isError && (
        <div className="mb-4 p-4 rounded border border-[var(--color-danger)] bg-red-50 text-[var(--color-danger)]">
          <p className="font-semibold">Failed to create batch.</p>
          <p className="text-sm mt-1">
            {error instanceof APIError ? error.message : error instanceof Error ? error.message : 'Unknown error'}
          </p>
        </div>
      )}

      <div className="space-y-4 max-w-2xl">
        <div className="flex flex-col gap-1">
          <label htmlFor="batch-recipe" className="text-xs text-[var(--color-muted)] uppercase tracking-wide">
            Recipe <span className="text-[var(--color-danger)]">*</span>
          </label>
          <select
            id="batch-recipe"
            value={recipeId}
            onChange={(e) => handleRecipeChange(e.target.value)}
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
          >
            <option value="">Select a recipe…</option>
            {recipesData?.items.map((r) => (
              <option key={r.id} value={r.id}>{r.name}</option>
            ))}
          </select>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="flex flex-col gap-1">
            <label htmlFor="batch-number" className="text-xs text-[var(--color-muted)] uppercase tracking-wide">
              Batch Number <span className="text-[var(--color-danger)]">*</span>
            </label>
            <input
              id="batch-number"
              type="text"
              value={batchNumber}
              onChange={(e) => setBatchNumber(e.target.value)}
              placeholder="e.g. 2026-001"
              className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
            />
          </div>
          <div className="flex flex-col gap-1">
            <label htmlFor="batch-name" className="text-xs text-[var(--color-muted)] uppercase tracking-wide">
              Name <span className="text-[var(--color-danger)]">*</span>
            </label>
            <input
              id="batch-name"
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Batch name"
              className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
            />
          </div>
        </div>

        <div className="flex flex-col gap-1">
          <label htmlFor="batch-brew-date" className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Brew Date</label>
          <input
            id="batch-brew-date"
            type="date"
            value={brewDate}
            onChange={(e) => setBrewDate(e.target.value)}
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

        <button
          onClick={handleSave}
          disabled={isPending || !recipeId || !batchNumber || !name}
          className="px-6 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
        >
          {isPending ? 'Creating…' : 'Create batch'}
        </button>
      </div>
    </div>
  )
}
