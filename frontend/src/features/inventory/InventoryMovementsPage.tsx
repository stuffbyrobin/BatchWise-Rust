import React from 'react'
import { useStockMovements } from './hooks/useInventory'
import { APIError } from '../../api/error'

const REFERENCE_TYPES = ['all', 'batch', 'manual', 'waste', 'transfer'] as const

function SkeletonRow() {
  return (
    <tr className="animate-pulse">
      <td className="p-3 h-10" style={{ borderColor: 'var(--color-border)' }}>
        <div className="h-4 w-24 rounded" style={{ background: 'var(--color-border)' }} />
      </td>
      <td className="p-3 h-10" style={{ borderColor: 'var(--color-border)' }}>
        <div className="h-4 w-16 rounded" style={{ background: 'var(--color-border)' }} />
      </td>
      <td className="p-3 h-10" style={{ borderColor: 'var(--color-border)' }}>
        <div className="h-4 w-16 rounded" style={{ background: 'var(--color-border)' }} />
      </td>
      <td className="p-3 h-10" style={{ borderColor: 'var(--color-border)' }}>
        <div className="h-4 w-20 rounded" style={{ background: 'var(--color-border)' }} />
      </td>
      <td className="p-3 h-10" style={{ borderColor: 'var(--color-border)' }}>
        <div className="h-4 w-24 rounded" style={{ background: 'var(--color-border)' }} />
      </td>
    </tr>
  )
}

export function InventoryMovementsPage() {
  const [ingredientId, setIngredientId] = React.useState<string>('')
  const [referenceType, setReferenceType] = React.useState<string>('all')
  const [page, setPage] = React.useState<number>(1)

  const params = React.useMemo(() => ({
    ingredient_id: ingredientId || undefined,
    reference_type: referenceType === 'all' ? undefined : referenceType,
    page,
    page_size: 20,
  }), [ingredientId, referenceType, page])

  const { data, isLoading, isError, error, refetch } = useStockMovements(params)

  const movements = data?.items ?? []
  const totalPages = data?.total_pages ?? 1

  const handlePrev = () => setPage((p) => Math.max(1, p - 1))
  const handleNext = () => setPage((p) => Math.min(totalPages, p + 1))

  if (isError) {
    return (
      <div className="p-4 rounded border border-[var(--color-danger)] bg-red-50 text-[var(--color-danger)]">
        <p className="font-semibold">Failed to load stock movements.</p>
        <p className="text-sm mt-1">
          {error instanceof APIError ? error.message : error instanceof Error ? error.message : 'Unknown error'}
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

  return (
    <div>
      <h1 className="text-xl font-bold mb-6 text-[var(--color-fg)]">Stock Movements</h1>

      <div className="flex flex-col md:flex-row gap-4 mb-6">
        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Ingredient ID</label>
          <input
            type="text"
            value={ingredientId}
            onChange={(e) => { setIngredientId(e.target.value); setPage(1) }}
            placeholder="Filter by ingredient ID"
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
          />
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Reference Type</label>
          <select
            value={referenceType}
            onChange={(e) => { setReferenceType(e.target.value); setPage(1) }}
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
          >
            {REFERENCE_TYPES.map((t) => (
              <option key={t} value={t}>
                {t === 'all' ? 'All types' : t}
              </option>
            ))}
          </select>
        </div>
      </div>

      <div className="overflow-x-auto border rounded-lg" style={{ borderColor: 'var(--color-border)' }}>
        <table className="w-full">
          <thead>
            <tr className="text-left text-xs text-[var(--color-muted)] uppercase tracking-wide">
              <th className="p-3" style={{ borderColor: 'var(--color-border)' }}>Date</th>
              <th className="p-3" style={{ borderColor: 'var(--color-border)' }}>Delta</th>
              <th className="p-3" style={{ borderColor: 'var(--color-border)' }}>Balance After</th>
              <th className="p-3" style={{ borderColor: 'var(--color-border)' }}>Reference Type</th>
              <th className="p-3" style={{ borderColor: 'var(--color-border)' }}>Notes</th>
            </tr>
          </thead>
          <tbody>
            {isLoading ? (
              Array.from({ length: 5 }).map((_, i) => <SkeletonRow key={i} />)
            ) : movements.length === 0 ? (
              <tr>
                <td colSpan={5} className="p-6 text-center text-[var(--color-muted)]">
                  No stock movements found.
                </td>
              </tr>
            ) : (
              movements.map((movement) => (
                <tr key={movement.id} style={{ borderColor: 'var(--color-border)' }}>
                  <td className="p-3" style={{ borderColor: 'var(--color-border)' }}>
                    {new Date(movement.created_at).toLocaleString()}
                  </td>
                  <td className="p-3" style={{ borderColor: 'var(--color-border)' }}>
                    <span className={movement.amount_delta >= 0 ? 'text-[var(--color-success)]' : 'text-[var(--color-danger)]'}>
                      {movement.amount_delta >= 0 ? '+' : ''}{movement.amount_delta}
                    </span>
                  </td>
                  <td className="p-3" style={{ borderColor: 'var(--color-border)' }}>
                    {movement.balance_after}
                  </td>
                  <td className="p-3" style={{ borderColor: 'var(--color-border)' }}>
                    {movement.reference_type}
                  </td>
                  <td className="p-3" style={{ borderColor: 'var(--color-border)' }}>
                    {movement.notes ?? '-'}
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      {totalPages > 1 && (
        <div className="flex items-center justify-between mt-4">
          <button
            onClick={handlePrev}
            disabled={page <= 1}
            className="px-4 py-2 rounded text-sm border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] disabled:opacity-50"
          >
            Previous
          </button>
          <span className="text-sm text-[var(--color-muted)]">
            Page {page} of {totalPages}
          </span>
          <button
            onClick={handleNext}
            disabled={page >= totalPages}
            className="px-4 py-2 rounded text-sm border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] disabled:opacity-50"
          >
            Next
          </button>
        </div>
      )}
    </div>
  )
}
