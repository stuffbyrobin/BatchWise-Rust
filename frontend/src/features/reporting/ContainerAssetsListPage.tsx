import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useContainerAssetsList, CONTAINER_TYPES } from './hooks/useContainerAssets'
import { APIError } from '../../api/error'

export function ContainerAssetsListPage() {
  const navigate = useNavigate()
  const [page, setPage] = useState(1)
  const [containerType, setContainerType] = useState('')
  const { data, isLoading, isError, error, refetch } = useContainerAssetsList({
    page,
    page_size: 20,
    container_type: containerType || undefined,
  })

  const totalPages = data?.total_pages || 1

  const getStatusBadge = (status: string) => {
    const baseClass = 'px-2 py-1 rounded text-xs'
    if (status === 'lost') return <span className={baseClass + ' bg-[var(--color-danger)] text-white'}>{status}</span>
    if (status === 'filled' || status === 'delivered') return <span className={baseClass + ' bg-[var(--color-success)] text-white'}>{status}</span>
    return <span className={baseClass + ' bg-[var(--color-border)] text-[var(--color-muted)]'}>{status}</span>
  }

  const formatDeposit = (pence: number | null | undefined) => {
    if (!pence) return '-'
    return '£' + (pence / 100).toFixed(2)
  }

  if (isError) {
    return (
      <div className="p-6">
        <div className="p-4 border border-[var(--color-danger)] rounded bg-[var(--color-danger)/10]">
          <p className="text-[var(--color-danger)]">{error instanceof APIError ? error.message : 'An error occurred'}</p>
          <button onClick={() => refetch()} className="mt-2 px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90">
            Retry
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex justify-between items-center">
        <h1 className="text-2xl font-bold text-[var(--color-fg)]">Container Assets</h1>
        <button
          onClick={() => navigate('/container-assets/new')}
          className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90"
        >
          New asset
        </button>
      </div>

      <div className="flex gap-4 items-center">
        <label className="text-[var(--color-muted)] text-sm">Type:</label>
        <select
          value={containerType}
          onChange={(e) => setContainerType(e.target.value)}
          className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm"
        >
          <option value="">All types</option>
          {CONTAINER_TYPES.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
      </div>

      {isLoading && (
        <div className="space-y-2 animate-pulse">
          {[...Array(5)].map((_, i) => (
            <div key={i} className="h-12 rounded bg-[var(--color-border)/20]" />
          ))}
        </div>
      )}

      {!isLoading && !isError && data && (data.items ?? []).length === 0 && (
        <p className="text-[var(--color-muted)]">No container assets yet.</p>
      )}

      {!isLoading && !isError && data && (data.items ?? []).length > 0 && (
        <>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-[var(--color-border)]">
                  <th className="text-left p-2 text-[var(--color-muted)]">Asset #</th>
                  <th className="text-left p-2 text-[var(--color-muted)]">Type</th>
                  <th className="text-left p-2 text-[var(--color-muted)]">Capacity (L)</th>
                  <th className="text-left p-2 text-[var(--color-muted)]">Status</th>
                  <th className="text-left p-2 text-[var(--color-muted)]">Deposit</th>
                  <th className="text-left p-2 text-[var(--color-muted)]">Actions</th>
                </tr>
              </thead>
              <tbody>
                {(data.items ?? []).map((item) => (
                  <tr key={item.id} className="border-b border-[var(--color-border)] hover:bg-[var(--color-border)/30]">
                    <td className="p-2 text-[var(--color-fg)]">{item.asset_number}</td>
                    <td className="p-2 text-[var(--color-fg)]">{item.container_type}</td>
                    <td className="p-2 text-[var(--color-fg)]">{item.capacity_liters}</td>
                    <td className="p-2">{getStatusBadge(item.status ?? '')}</td>
                    <td className="p-2 text-[var(--color-fg)]">{formatDeposit(item.deposit_pence)}</td>
                    <td className="p-2">
                      <button
                        onClick={() => navigate('/container-assets/' + (item.id ?? ''))}
                        className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90"
                      >
                        View
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="flex justify-between items-center">
            <span className="text-[var(--color-muted)] text-sm">
              Page {page} of {totalPages}
            </span>
            <div className="flex gap-2">
              <button
                onClick={() => setPage((p) => Math.max(1, p - 1))}
                disabled={page <= 1}
                className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
              >
                Prev
              </button>
              <button
                onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
                disabled={page >= totalPages}
                className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
              >
                Next
              </button>
            </div>
          </div>
        </>
      )}
    </div>
  )
}
