import React from 'react'
import { useNavigate } from 'react-router-dom'
import { useBatchesList, BATCH_STATUSES, STATUS_LABELS, STATUS_COLORS } from './hooks/useBatches'
import { APIError } from '../../api/error'

export function BatchesListPage() {
  const navigate = useNavigate()
  const [status, setStatus] = React.useState('')
  const [brewDateFrom, setBrewDateFrom] = React.useState('')
  const [brewDateTo, setBrewDateTo] = React.useState('')
  const [page, setPage] = React.useState(1)

  const { data, isLoading, isError, error, refetch } = useBatchesList({
    status: status || undefined,
    brew_date_from: brewDateFrom || undefined,
    brew_date_to: brewDateTo || undefined,
    page,
    page_size: 20,
    sort: '-brew_date',
  })

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-bold text-[var(--color-fg)]">Batches</h1>
        <div className="flex gap-2">
          <button
            onClick={() => navigate('/batches/import')}
            className="px-4 py-2 rounded text-sm border border-[var(--color-border)] text-[var(--color-fg)] hover:bg-[var(--color-border)]"
          >
            Import from Brewfather
          </button>
          <button
            onClick={() => navigate('/batches/new')}
            className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90"
          >
            New batch
          </button>
        </div>
      </div>

      <div className="flex flex-wrap gap-3 mb-6">
        <select
          value={status}
          onChange={(e) => { setStatus(e.target.value); setPage(1) }}
          className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm"
        >
          <option value="">All statuses</option>
          {BATCH_STATUSES.map((s) => (
            <option key={s} value={s}>{STATUS_LABELS[s]}</option>
          ))}
        </select>

        <div className="flex items-center gap-2">
          <label className="text-xs text-[var(--color-muted)]">Brew date</label>
          <input
            type="date"
            value={brewDateFrom}
            onChange={(e) => { setBrewDateFrom(e.target.value); setPage(1) }}
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm"
          />
          <span className="text-[var(--color-muted)] text-sm">–</span>
          <input
            type="date"
            value={brewDateTo}
            onChange={(e) => { setBrewDateTo(e.target.value); setPage(1) }}
            className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm"
          />
        </div>
      </div>

      {isError && (
        <div className="p-4 rounded border border-[var(--color-danger)] bg-red-50 text-[var(--color-danger)] mb-4">
          <p className="font-semibold">Failed to load batches.</p>
          <p className="text-sm mt-1">
            {error instanceof APIError ? error.message : error instanceof Error ? error.message : 'Unknown error'}
          </p>
          <button onClick={() => refetch()} className="mt-2 px-3 py-1 text-sm rounded bg-[var(--color-danger)] text-white">
            Retry
          </button>
        </div>
      )}

      {isLoading ? (
        <div className="animate-pulse space-y-2">
          {[...Array(5)].map((_, i) => (
            <div key={i} className="h-12 rounded" style={{ background: 'var(--color-border)' }} />
          ))}
        </div>
      ) : (
        <>
          <div className="overflow-x-auto rounded border border-[var(--color-border)]">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-[var(--color-border)] bg-[var(--color-surface)]">
                  <th className="px-4 py-2 text-left text-xs text-[var(--color-muted)] uppercase tracking-wide">Batch #</th>
                  <th className="px-4 py-2 text-left text-xs text-[var(--color-muted)] uppercase tracking-wide">Name</th>
                  <th className="px-4 py-2 text-left text-xs text-[var(--color-muted)] uppercase tracking-wide">Status</th>
                  <th className="px-4 py-2 text-left text-xs text-[var(--color-muted)] uppercase tracking-wide">Brew Date</th>
                  <th className="px-4 py-2 text-left text-xs text-[var(--color-muted)] uppercase tracking-wide">OG / FG</th>
                </tr>
              </thead>
              <tbody>
                {(data?.items?.length ?? 0) === 0 && (
                  <tr>
                    <td colSpan={5} className="px-4 py-8 text-center text-[var(--color-muted)]">
                      No batches found.
                    </td>
                  </tr>
                )}
                {data?.items?.map((batch) => (
                  <tr
                    key={batch.id}
                    onClick={() => navigate(`/batches/${batch.id}`)}
                    className="border-b border-[var(--color-border)] hover:bg-[var(--color-surface)] cursor-pointer"
                  >
                    <td className="px-4 py-3 font-mono text-xs text-[var(--color-muted)]">{batch.batch_number}</td>
                    <td className="px-4 py-3 font-medium text-[var(--color-fg)]">{batch.name}</td>
                    <td className="px-4 py-3">
                      <span
                        className="px-2 py-0.5 rounded text-xs font-medium text-white"
                        style={{ background: STATUS_COLORS[batch.status as keyof typeof STATUS_COLORS] ?? 'var(--color-muted)' }}
                      >
                        {STATUS_LABELS[batch.status as keyof typeof STATUS_LABELS] ?? batch.status}
                      </span>
                    </td>
                    <td className="px-4 py-3 text-[var(--color-muted)]">{batch.brew_date ?? '—'}</td>
                    <td className="px-4 py-3 text-[var(--color-muted)] text-xs">
                      {batch.actual_og ?? batch.batch_recipe_snapshot?.calc_og ?? '—'}
                      {' / '}
                      {batch.actual_fg ?? batch.batch_recipe_snapshot?.calc_fg ?? '—'}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {(data?.total_pages ?? 0) > 1 && (
            <div className="flex items-center justify-between mt-4 text-sm text-[var(--color-muted)]">
              <span>Page {data?.page} of {data?.total_pages} ({data?.total} batches)</span>
              <div className="flex gap-2">
                <button
                  onClick={() => setPage((p) => Math.max(1, p - 1))}
                  disabled={page <= 1}
                  className="px-3 py-1 rounded border border-[var(--color-border)] disabled:opacity-40"
                >
                  Previous
                </button>
                <button
                  onClick={() => setPage((p) => p + 1)}
                  disabled={page >= (data?.total_pages ?? 1)}
                  className="px-3 py-1 rounded border border-[var(--color-border)] disabled:opacity-40"
                >
                  Next
                </button>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  )
}
