import React from 'react'
import { useNavigate } from 'react-router-dom'
import { useInventoryList } from './hooks/useInventory'
import { APIError } from '../../api/error'

const INGREDIENT_TYPES = ['all', 'fermentable', 'hop', 'yeast', 'adjunct', 'chemical', 'other'] as const

function SkeletonRow() {
  return (
    <tr className="animate-pulse">
      <td className="p-3 h-10" style={{ borderColor: 'var(--color-border)' }}>
        <div className="h-4 w-32 rounded" style={{ background: 'var(--color-border)' }} />
      </td>
      <td className="p-3 h-10" style={{ borderColor: 'var(--color-border)' }}>
        <div className="h-4 w-20 rounded" style={{ background: 'var(--color-border)' }} />
      </td>
      <td className="p-3 h-10" style={{ borderColor: 'var(--color-border)' }}>
        <div className="h-4 w-16 rounded" style={{ background: 'var(--color-border)' }} />
      </td>
      <td className="p-3 h-10" style={{ borderColor: 'var(--color-border)' }}>
        <div className="h-4 w-24 rounded" style={{ background: 'var(--color-border)' }} />
      </td>
      <td className="p-3 h-10" style={{ borderColor: 'var(--color-border)' }}>
        <div className="h-4 w-20 rounded" style={{ background: 'var(--color-border)' }} />
      </td>
      <td className="p-3 h-10" style={{ borderColor: 'var(--color-border)' }}>
        <div className="h-4 w-20 rounded" style={{ background: 'var(--color-border)' }} />
      </td>
    </tr>
  )
}

export function InventoryListPage() {
  const navigate = useNavigate()
  const [typeFilter, setTypeFilter] = React.useState<string>('all')
  const [nameFilter, setNameFilter] = React.useState<string>('')
  const [expiringWithinDays, setExpiringWithinDays] = React.useState<number | ''>('')
  const [showOutOfStock, setShowOutOfStock] = React.useState<boolean>(false)
  const [page, setPage] = React.useState<number>(1)

  const params = React.useMemo(() => ({
    type: typeFilter === 'all' ? undefined : typeFilter,
    name: nameFilter || undefined,
    expiring_within_days: expiringWithinDays === '' ? undefined : Number(expiringWithinDays),
    out_of_stock: showOutOfStock || undefined,
    page,
    page_size: 20,
  }), [typeFilter, nameFilter, expiringWithinDays, showOutOfStock, page])

  const { data, isLoading, isError, error, refetch } = useInventoryList(params)

  const filteredTypes = data?.items ?? []
  const totalPages = data?.total_pages ?? 1

  const handlePrev = () => setPage((p) => Math.max(1, p - 1))
  const handleNext = () => setPage((p) => Math.min(totalPages, p + 1))

  if (isError) {
    return (
      <div className="p-4 rounded border border-[var(--color-danger)] bg-red-50 text-[var(--color-danger)]">
        <p className="font-semibold">Failed to load inventory.</p>
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
      <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-4 mb-6">
        <h1 className="text-xl font-bold text-[var(--color-fg)]">Inventory Lots</h1>
        <div className="flex gap-2">
          <button
            onClick={() => navigate('/inventory/import')}
            className="px-4 py-2 rounded text-sm border border-[var(--color-border)] text-[var(--color-fg)] hover:bg-[var(--color-border)]"
          >
            Import
          </button>
          <button
            onClick={() => navigate('/inventory/new')}
            className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90"
          >
            New lot
          </button>
        </div>
      </div>

      <div className="flex flex-col md:flex-row gap-4 mb-6">
        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Type</label>
          <select
            value={typeFilter}
            onChange={(e) => { setTypeFilter(e.target.value); setPage(1) }}
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
          >
            {INGREDIENT_TYPES.map((t) => (
              <option key={t} value={t}>
                {t === 'all' ? 'All types' : t}
              </option>
            ))}
          </select>
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Name</label>
          <input
            type="text"
            value={nameFilter}
            onChange={(e) => { setNameFilter(e.target.value); setPage(1) }}
            placeholder="Filter by name"
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
          />
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">
            Expiring within (days)
          </label>
          <input
            type="number"
            value={expiringWithinDays}
            onChange={(e) => { setExpiringWithinDays(e.target.value === '' ? '' : Number(e.target.value)); setPage(1) }}
            placeholder="Days"
            min="0"
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
          />
        </div>
        <div className="flex flex-col gap-1 justify-end">
          <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Stock</label>
          <label className="flex items-center gap-2 p-2 text-sm text-[var(--color-fg)] cursor-pointer select-none">
            <input
              type="checkbox"
              checked={showOutOfStock}
              onChange={(e) => { setShowOutOfStock(e.target.checked); setPage(1) }}
            />
            Show out of stock
          </label>
        </div>
      </div>

      <div className="overflow-x-auto border rounded-lg" style={{ borderColor: 'var(--color-border)' }}>
        <table className="w-full">
          <thead>
            <tr className="text-left text-xs text-[var(--color-muted)] uppercase tracking-wide">
              <th className="p-3" style={{ borderColor: 'var(--color-border)' }}>Name</th>
              <th className="p-3" style={{ borderColor: 'var(--color-border)' }}>Type</th>
              <th className="p-3" style={{ borderColor: 'var(--color-border)' }}>Amount</th>
              <th className="p-3" style={{ borderColor: 'var(--color-border)' }}>Lot Number</th>
              <th className="p-3" style={{ borderColor: 'var(--color-border)' }}>Best Before</th>
              <th className="p-3" style={{ borderColor: 'var(--color-border)' }}>Supplier</th>
            </tr>
          </thead>
          <tbody>
            {isLoading ? (
              Array.from({ length: 5 }).map((_, i) => <SkeletonRow key={i} />)
            ) : filteredTypes.length === 0 ? (
              <tr>
                <td colSpan={6} className="p-6 text-center text-[var(--color-muted)]">
                  No lots found. Add your first lot.
                </td>
              </tr>
            ) : (
              filteredTypes.map((item) => (
                <tr
                  key={item.id}
                  className="hover:bg-[var(--color-surface)] cursor-pointer"
                  onClick={() => navigate(`/inventory/${item.id}`)}
                  style={{ borderColor: 'var(--color-border)' }}
                >
                  <td className="p-3" style={{ borderColor: 'var(--color-border)' }}>
                    {item.name}
                  </td>
                  <td className="p-3" style={{ borderColor: 'var(--color-border)' }}>
                    {item.type}
                  </td>
                  <td className="p-3" style={{ borderColor: 'var(--color-border)' }}>
                    {item.amount} {item.unit}
                  </td>
                  <td className="p-3" style={{ borderColor: 'var(--color-border)' }}>
                    {item.lot_number}
                  </td>
                  <td className="p-3" style={{ borderColor: 'var(--color-border)' }}>
                    {item.best_before_date ? new Date(item.best_before_date).toLocaleDateString() : '-'}
                  </td>
                  <td className="p-3" style={{ borderColor: 'var(--color-border)' }}>
                    {item.supplier ?? '-'}
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
