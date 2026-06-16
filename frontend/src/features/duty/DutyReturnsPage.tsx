import React from 'react'
import { APIError } from '../../api/error'
import { useDutyReturns, useCompileDutyReturn, usePatchDutyReturn } from './hooks/useDuty'
import type { components } from '../../api/generated'

type DutyReturn = components['schemas']['DutyReturn']

function fmtPence(p: number | null | undefined): string {
  if (p == null) return '—'
  return '£' + (p / 100).toFixed(2)
}

function fmtDate(s: string | undefined): string {
  if (!s) return '—'
  return s.slice(0, 10)
}

function prevMonthRange(): { start: string; end: string } {
  const now = new Date()
  const y = now.getFullYear()
  const m = now.getMonth() // 0-indexed, so this is last month's index when we subtract
  const first = new Date(y, m - 1, 1)
  const last = new Date(y, m, 0)
  const pad = (n: number) => String(n).padStart(2, '0')
  const fmt = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
  return { start: fmt(first), end: fmt(last) }
}

function SubmitButton({ ret }: { ret: DutyReturn }) {
  const patch = usePatchDutyReturn(ret.id ?? '')
  const [err, setErr] = React.useState<string | null>(null)

  if (ret.status === 'submitted') {
    return <span className="text-xs text-[var(--color-muted)]">Submitted</span>
  }

  return (
    <div className="flex items-center gap-2">
      <button
        className="px-2 py-1 text-xs rounded bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
        disabled={patch.isPending || (ret.event_count ?? 0) === 0}
        title={(ret.event_count ?? 0) === 0 ? 'No duty events in this period' : undefined}
        onClick={async () => {
          setErr(null)
          try {
            await patch.mutateAsync({ status: 'submitted' })
          } catch (e) {
            setErr(e instanceof APIError ? e.message : 'Submit failed.')
          }
        }}
      >
        {patch.isPending ? 'Submitting…' : 'Submit'}
      </button>
      {err && <span className="text-xs text-[var(--color-danger)]">{err}</span>}
    </div>
  )
}

export function DutyReturnsPage() {
  const prev = prevMonthRange()
  const [periodStart, setPeriodStart] = React.useState(prev.start)
  const [periodEnd, setPeriodEnd] = React.useState(prev.end)
  const [compileErr, setCompileErr] = React.useState<string | null>(null)

  const { data, isLoading, isError, error } = useDutyReturns({ page_size: 50 })
  const compile = useCompileDutyReturn()

  const handleCompile = async (e: React.FormEvent) => {
    e.preventDefault()
    setCompileErr(null)
    try {
      await compile.mutateAsync({ period_start: periodStart, period_end: periodEnd })
    } catch (err) {
      setCompileErr(err instanceof APIError ? err.message : 'Compile failed.')
    }
  }

  const inputCls =
    'p-2 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)] focus:outline-none'

  return (
    <div className="p-6 max-w-4xl mx-auto">
      <h1 className="text-xl font-bold mb-6" style={{ color: 'var(--color-fg)' }}>
        Beer Duty Returns
      </h1>

      {/* Compile form */}
      <div
        className="rounded-xl border p-5 mb-6"
        style={{ borderColor: 'var(--color-border)', background: 'var(--color-surface)' }}
      >
        <h2 className="text-base font-semibold mb-1" style={{ color: 'var(--color-fg)' }}>
          Compile Return
        </h2>
        <p className="text-xs mb-4" style={{ color: 'var(--color-muted)' }}>
          Aggregate duty events for a period and calculate Small Producer Relief. Recompiling an
          existing draft will update it in place.
        </p>
        <form onSubmit={handleCompile} className="flex flex-wrap items-end gap-3">
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Period Start</label>
            <input
              type="date"
              required
              className={inputCls}
              value={periodStart}
              onChange={(e) => setPeriodStart(e.target.value)}
            />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Period End</label>
            <input
              type="date"
              required
              className={inputCls}
              value={periodEnd}
              onChange={(e) => setPeriodEnd(e.target.value)}
            />
          </div>
          <button
            type="submit"
            disabled={compile.isPending}
            className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
          >
            {compile.isPending ? 'Compiling…' : 'Compile'}
          </button>
          {compileErr && (
            <p className="w-full text-xs text-[var(--color-danger)]">{compileErr}</p>
          )}
        </form>
      </div>

      {/* Returns list */}
      {isLoading && <p className="text-sm text-[var(--color-muted)]">Loading…</p>}
      {isError && (
        <p className="text-sm text-[var(--color-danger)]">
          {error instanceof APIError ? error.message : 'Failed to load duty returns.'}
        </p>
      )}
      {!isLoading && !isError && (
        <>
          {(data?.items?.length ?? 0) === 0 ? (
            <p className="text-sm text-[var(--color-muted)]">
              No duty returns yet. Compile one above.
            </p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-sm border-collapse">
                <thead>
                  <tr className="text-left text-xs text-[var(--color-muted)] border-b"
                      style={{ borderColor: 'var(--color-border)' }}>
                    <th className="pb-2 pr-4">Period</th>
                    <th className="pb-2 pr-4">Events</th>
                    <th className="pb-2 pr-4">Gross Duty</th>
                    <th className="pb-2 pr-4">SPR Relief</th>
                    <th className="pb-2 pr-4">Net Duty</th>
                    <th className="pb-2 pr-4">Status</th>
                    <th className="pb-2"></th>
                  </tr>
                </thead>
                <tbody>
                  {data?.items?.map((r) => (
                    <tr
                      key={r.id}
                      className="border-b"
                      style={{ borderColor: 'var(--color-border)' }}
                    >
                      <td className="py-2 pr-4 font-mono text-xs">
                        {fmtDate(r.period_start as unknown as string)} – {fmtDate(r.period_end as unknown as string)}
                      </td>
                      <td className="py-2 pr-4">{r.event_count ?? 0}</td>
                      <td className="py-2 pr-4">{fmtPence(r.gross_duty_pence)}</td>
                      <td className="py-2 pr-4">
                        {fmtPence(r.sbr_relief_pence)}
                        {r.sbr_relief_rate_pct != null && r.sbr_relief_rate_pct > 0 && (
                          <span className="text-xs text-[var(--color-muted)] ml-1">
                            ({(r.sbr_relief_rate_pct).toFixed(1)}%)
                          </span>
                        )}
                      </td>
                      <td className="py-2 pr-4 font-semibold">{fmtPence(r.net_duty_pence)}</td>
                      <td className="py-2 pr-4">
                        <span
                          className={`px-2 py-0.5 rounded text-xs font-medium ${
                            r.status === 'submitted'
                              ? 'bg-green-100 text-green-700'
                              : 'bg-yellow-100 text-yellow-700'
                          }`}
                        >
                          {r.status}
                        </span>
                      </td>
                      <td className="py-2">
                        <SubmitButton ret={r} />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
          <p className="text-xs mt-3 text-[var(--color-muted)]">
            {data?.total ?? 0} return{(data?.total ?? 0) !== 1 ? 's' : ''} total
          </p>
        </>
      )}
    </div>
  )
}
