import { useInventorySummary } from './hooks/useInventory'
import { APIError } from '../../api/error'

function SkeletonRow() {
  return (
    <tr className="animate-pulse">
      <td className="p-3 h-10" style={{ borderColor: 'var(--color-border)' }}>
        <div className="h-4 w-20 rounded" style={{ background: 'var(--color-border)' }} />
      </td>
      <td className="p-3 h-10" style={{ borderColor: 'var(--color-border)' }}>
        <div className="h-4 w-24 rounded" style={{ background: 'var(--color-border)' }} />
      </td>
      <td className="p-3 h-10" style={{ borderColor: 'var(--color-border)' }}>
        <div className="h-4 w-12 rounded" style={{ background: 'var(--color-border)' }} />
      </td>
      <td className="p-3 h-10" style={{ borderColor: 'var(--color-border)' }}>
        <div className="h-4 w-16 rounded" style={{ background: 'var(--color-border)' }} />
      </td>
    </tr>
  )
}

export function InventorySummaryPage() {
  const { data, isLoading, isError, error, refetch } = useInventorySummary()

  if (isError) {
    return (
      <div className="p-4 rounded border border-[var(--color-danger)] bg-red-50 text-[var(--color-danger)]">
        <p className="font-semibold">Failed to load inventory summary.</p>
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

  const summaryItems = data?.items ?? []

  return (
    <div>
      <h1 className="text-xl font-bold mb-6 text-[var(--color-fg)]">Inventory Summary</h1>

      <div className="overflow-x-auto border rounded-lg" style={{ borderColor: 'var(--color-border)' }}>
        <table className="w-full">
          <thead>
            <tr className="text-left text-xs text-[var(--color-muted)] uppercase tracking-wide">
              <th className="p-3" style={{ borderColor: 'var(--color-border)' }}>Type</th>
              <th className="p-3" style={{ borderColor: 'var(--color-border)' }}>Name</th>
              <th className="p-3" style={{ borderColor: 'var(--color-border)' }}>Unit</th>
              <th className="p-3" style={{ borderColor: 'var(--color-border)' }}>Total Amount</th>
            </tr>
          </thead>
          <tbody>
            {isLoading ? (
              Array.from({ length: 5 }).map((_, i) => <SkeletonRow key={i} />)
            ) : summaryItems.length === 0 ? (
              <tr>
                <td colSpan={4} className="p-6 text-center text-[var(--color-muted)]">
                  No inventory summary data available.
                </td>
              </tr>
            ) : (
              summaryItems.map((item) => (
                <tr key={`${item.type}-${item.name}`} style={{ borderColor: 'var(--color-border)' }}>
                  <td className="p-3" style={{ borderColor: 'var(--color-border)' }}>
                    {item.type ?? '-'}
                  </td>
                  <td className="p-3" style={{ borderColor: 'var(--color-border)' }}>
                    {item.name ?? '-'}
                  </td>
                  <td className="p-3" style={{ borderColor: 'var(--color-border)' }}>
                    {item.unit ?? '-'}
                  </td>
                  <td className="p-3" style={{ borderColor: 'var(--color-border)' }}>
                    {item.total_amount ?? 0}
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  )
}
