import { useDashboardStats } from './useDashboardStats'
import { useAuth } from '../../auth/useAuth'

const STATUS_COLORS: Record<string, string> = {
  planned: 'var(--srm-4)',
  brewing: 'var(--srm-5)',
  fermenting: 'var(--srm-7)',
  conditioning: 'var(--srm-8)',
  packaging: 'var(--color-muted)',
  completed: 'var(--color-success)',
  cancelled: 'var(--color-border)',
}

function StatCard({ label, value }: { label: string; value: number | null | undefined }) {
  return (
    <div
      className="rounded-lg border p-5 flex flex-col gap-1"
      style={{ background: 'var(--color-surface)', borderColor: 'var(--color-border)' }}
    >
      <span className="text-sm text-[var(--color-muted)]">{label}</span>
      <span className="text-2xl font-bold text-[var(--color-fg)]">
        {value ?? 0}
      </span>
    </div>
  )
}

function SkeletonCard() {
  return (
    <div
      className="rounded-lg border p-5 animate-pulse"
      style={{ background: 'var(--color-surface)', borderColor: 'var(--color-border)' }}
    >
      <div className="h-4 w-24 rounded mb-2" style={{ background: 'var(--color-border)' }} />
      <div className="h-8 w-16 rounded" style={{ background: 'var(--color-border)' }} />
    </div>
  )
}

export function DashboardPage() {
  const { data, isLoading, isError, error, refetch } = useDashboardStats()
  const { user } = useAuth()
  const flags = user?.feature_flags ?? {}

  if (isLoading) {
    return (
      <div>
        <h1 className="text-xl font-bold mb-6 text-[var(--color-fg)]">Dashboard</h1>
        <div className="grid grid-cols-2 md:grid-cols-3 gap-4 mb-6">
          {Array.from({ length: 6 }).map((_, i) => <SkeletonCard key={i} />)}
        </div>
      </div>
    )
  }

  if (isError) {
    return (
      <div className="p-4 rounded border border-[var(--color-danger)] bg-red-50 text-[var(--color-danger)]">
        <p className="font-semibold">Failed to load dashboard.</p>
        <p className="text-sm mt-1">{error instanceof Error ? error.message : 'Unknown error'}</p>
        <button
          onClick={() => refetch()}
          className="mt-3 px-3 py-1 text-sm rounded bg-[var(--color-danger)] text-white"
        >
          Retry
        </button>
      </div>
    )
  }

  const breakdown = data?.batch_status_breakdown
  const breakdownEntries = breakdown
    ? Object.entries(breakdown).filter(([, v]) => (v ?? 0) > 0)
    : []
  const totalBatches = breakdownEntries.reduce((sum, [, v]) => sum + (v ?? 0), 0)

  return (
    <div>
      <h1 className="text-xl font-bold mb-6 text-[var(--color-fg)]">Dashboard</h1>

      <div className="grid grid-cols-2 md:grid-cols-3 gap-4 mb-6">
        <StatCard label="Active batches" value={data?.active_batches_count} />
        <StatCard label="Recipes" value={data?.recipes_count} />
        <StatCard label="Low stock lots" value={data?.low_stock_count} />
        <StatCard label="Expiring soon" value={data?.expiring_soon_count} />
        <StatCard label="Upcoming events (7d)" value={data?.upcoming_events_count} />

        {flags['tracking'] === true && (
          <StatCard label="Containers in use" value={data?.containers_in_use_count} />
        )}
        {flags['reporting'] === true && (
          <StatCard
            label="Est. duty last 30d (p)"
            value={data?.last_30d_estimated_duty_pence}
          />
        )}
      </div>

      {totalBatches > 0 && (
        <div>
          <h2 className="text-sm font-semibold text-[var(--color-muted)] mb-2 uppercase tracking-wide">
            Batch status
          </h2>
          <div className="flex rounded overflow-hidden h-6 w-full border border-[var(--color-border)]">
            {breakdownEntries.map(([status, count]) => (
              <div
                key={status}
                title={`${status}: ${count}`}
                style={{
                  width: `${((count ?? 0) / totalBatches) * 100}%`,
                  background: STATUS_COLORS[status] ?? 'var(--color-muted)',
                }}
              />
            ))}
          </div>
          <div className="flex flex-wrap gap-3 mt-2">
            {breakdownEntries.map(([status, count]) => (
              <span key={status} className="flex items-center gap-1 text-xs text-[var(--color-muted)]">
                <span
                  className="inline-block w-3 h-3 rounded-sm"
                  style={{ background: STATUS_COLORS[status] ?? 'var(--color-muted)' }}
                />
                {status} ({count})
              </span>
            ))}
          </div>
        </div>
      )}

      {!isLoading && (
        <p className="mt-6 text-xs text-[var(--color-muted)]">
          {data?.generated_at ? `Updated ${new Date(data.generated_at).toLocaleString()}` : ''}
        </p>
      )}
    </div>
  )
}
