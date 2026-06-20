import React from 'react'
import { Link } from 'react-router-dom'
import { useFermenters } from './hooks/useFermenters'
import { useBatchesList, STATUS_COLORS, STATUS_LABELS, type BatchStatus } from '../batches/hooks/useBatches'
import type { components } from '../../api/generated'

type Batch = components['schemas']['Batch']

const DAY = 86_400_000
// When a batch has no package_date yet, estimate the vessel is occupied this long.
const EST_FERMENT_DAYS = 21

function parseDate(s: string | null | undefined): Date | null {
  if (!s) return null
  const d = new Date(`${s}T00:00:00`)
  return Number.isNaN(d.getTime()) ? null : d
}
function addDays(d: Date, n: number): Date {
  return new Date(d.getTime() + n * DAY)
}
function startOfMonth(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth(), 1)
}
function fmtDay(d: Date): string {
  return d.toLocaleDateString(undefined, { day: 'numeric', month: 'short' })
}

interface Bar {
  batch: Batch
  start: Date
  end: Date
  estimated: boolean
}

export default function FermenterSchedulePage() {
  const { data: fermData, isLoading: fLoading } = useFermenters({ sort: 'name', page_size: 100 })
  const { data: batchData, isLoading: bLoading } = useBatchesList({ page_size: 200, sort: '-brew_date' })

  const fermenters = React.useMemo(() => fermData?.items ?? [], [fermData])
  const batches = React.useMemo(() => batchData?.items ?? [], [batchData])

  // Batches assigned to a fermenter, with a usable start date → timeline bars.
  const { barsByFermenter, unscheduled, rangeStart, totalDays, months } = React.useMemo(() => {
    const byFerm = new Map<string, Bar[]>()
    const unsched: Batch[] = []
    let min: Date | null = null
    let max: Date | null = null

    for (const b of batches) {
      if (!b.fermenter_id) continue
      const start = parseDate(b.brew_date)
      if (!start) {
        unsched.push(b)
        continue
      }
      const pkg = parseDate(b.package_date)
      const estimated = !pkg
      const end = pkg ?? addDays(start, EST_FERMENT_DAYS)
      const bar: Bar = { batch: b, start, end, estimated }
      const list = byFerm.get(b.fermenter_id) ?? []
      list.push(bar)
      byFerm.set(b.fermenter_id, list)
      if (!min || start < min) min = start
      if (!max || end > max) max = end
    }

    // Default to a today→+90d window when there's nothing scheduled.
    const today = new Date()
    let rStart = min ? startOfMonth(min) : startOfMonth(today)
    let rEnd = max ? addDays(max, 7) : addDays(today, 90)
    if (rEnd <= rStart) rEnd = addDays(rStart, 30)
    // Round end up to the start of the following month for clean gridlines.
    rEnd = startOfMonth(addDays(startOfMonth(rEnd), 32))
    const total = Math.max(1, Math.round((rEnd.getTime() - rStart.getTime()) / DAY))

    // Month gridline positions.
    const ms: { label: string; leftPct: number }[] = []
    let cur = new Date(rStart)
    while (cur < rEnd) {
      ms.push({
        label: cur.toLocaleDateString(undefined, { month: 'short', year: '2-digit' }),
        leftPct: ((cur.getTime() - rStart.getTime()) / DAY / total) * 100,
      })
      cur = new Date(cur.getFullYear(), cur.getMonth() + 1, 1)
    }

    return { barsByFermenter: byFerm, unscheduled: unsched, rangeStart: rStart, totalDays: total, months: ms }
  }, [batches])

  const todayPct = React.useMemo(() => {
    const p = ((Date.now() - rangeStart.getTime()) / DAY / totalDays) * 100
    return p >= 0 && p <= 100 ? p : null
  }, [rangeStart, totalDays])

  function pct(d: Date): number {
    return ((d.getTime() - rangeStart.getTime()) / DAY / totalDays) * 100
  }

  const usedStatuses = React.useMemo(() => {
    const s = new Set<string>()
    for (const list of barsByFermenter.values()) for (const b of list) if (b.batch.status) s.add(b.batch.status)
    return [...s]
  }, [barsByFermenter])

  if (fLoading || bLoading) return <p className="text-[var(--color-muted)]">Loading…</p>

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-bold text-[var(--color-fg)]">Fermenter Schedule</h1>
        <Link
          to="/fermenters"
          className="px-4 py-2 rounded text-sm border border-[var(--color-border)] text-[var(--color-fg)] hover:bg-[var(--color-border)]"
        >
          Manage fermenters
        </Link>
      </div>

      {fermenters.length === 0 ? (
        <div className="p-6 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-muted)]">
          No fermenters yet. <Link to="/fermenters" className="text-[var(--color-accent)] underline">Add some</Link>, then assign batches to them when planning.
        </div>
      ) : (
        <>
          {/* Legend */}
          {usedStatuses.length > 0 && (
            <div className="flex flex-wrap gap-3 mb-3 text-xs text-[var(--color-muted)]">
              {usedStatuses.map((s) => (
                <span key={s} className="inline-flex items-center gap-1">
                  <span className="inline-block w-3 h-3 rounded-sm" style={{ background: STATUS_COLORS[s as BatchStatus] }} />
                  {STATUS_LABELS[s as BatchStatus] ?? s}
                </span>
              ))}
              <span className="inline-flex items-center gap-1">
                <span className="inline-block w-3 h-3 rounded-sm border border-dashed border-[var(--color-muted)]" />
                est. end (no package date)
              </span>
            </div>
          )}

          <div className="overflow-x-auto border rounded-lg" style={{ borderColor: 'var(--color-border)' }}>
            <div className="min-w-[720px]">
              {/* Month header */}
              <div className="flex border-b" style={{ borderColor: 'var(--color-border)' }}>
                <div className="w-40 shrink-0 p-2 text-xs font-medium text-[var(--color-muted)] uppercase">Fermenter</div>
                <div className="relative flex-1 h-8">
                  {months.map((m, i) => (
                    <div
                      key={i}
                      className="absolute top-0 h-full border-l text-[10px] text-[var(--color-muted)] pl-1 pt-1"
                      style={{ left: `${m.leftPct}%`, borderColor: 'var(--color-border)' }}
                    >
                      {m.label}
                    </div>
                  ))}
                </div>
              </div>

              {/* Rows */}
              {fermenters.map((f) => {
                const bars = barsByFermenter.get(f.id) ?? []
                return (
                  <div key={f.id} className="flex border-b last:border-b-0" style={{ borderColor: 'var(--color-border)' }}>
                    <div className="w-40 shrink-0 p-2 border-r" style={{ borderColor: 'var(--color-border)' }}>
                      <div className="text-sm font-medium text-[var(--color-fg)] truncate">{f.name}</div>
                      {f.capacity_liters != null && <div className="text-[10px] text-[var(--color-muted)]">{f.capacity_liters} L</div>}
                    </div>
                    <div className="relative flex-1 h-12 bg-[var(--color-surface)]">
                      {/* month gridlines */}
                      {months.map((m, i) => (
                        <div key={i} className="absolute top-0 h-full border-l" style={{ left: `${m.leftPct}%`, borderColor: 'var(--color-border)', opacity: 0.5 }} />
                      ))}
                      {/* today marker */}
                      {todayPct != null && (
                        <div className="absolute top-0 h-full border-l-2 border-[var(--color-accent)]" style={{ left: `${todayPct}%` }} title="Today" />
                      )}
                      {/* batch bars */}
                      {bars.map((bar) => {
                        const left = Math.max(0, pct(bar.start))
                        const width = Math.max(1.5, pct(bar.end) - pct(bar.start))
                        return (
                          <Link
                            key={bar.batch.id}
                            to={`/batches/${bar.batch.id}`}
                            title={`${bar.batch.name} (${bar.batch.batch_number}) — ${STATUS_LABELS[bar.batch.status as BatchStatus] ?? bar.batch.status}\n${bar.batch.brew_date ?? '?'} → ${bar.batch.package_date ?? `~${fmtDay(bar.end)} (est.)`}`}
                            className="absolute top-2 h-8 rounded px-1.5 flex items-center text-[11px] text-white overflow-hidden whitespace-nowrap"
                            style={{
                              left: `${left}%`,
                              width: `${width}%`,
                              background: STATUS_COLORS[bar.batch.status as BatchStatus] ?? 'var(--color-accent)',
                              border: bar.estimated ? '1px dashed rgba(255,255,255,0.7)' : 'none',
                            }}
                          >
                            {bar.batch.name}
                          </Link>
                        )
                      })}
                    </div>
                  </div>
                )
              })}
            </div>
          </div>

          {/* Unscheduled assigned batches */}
          {unscheduled.length > 0 && (
            <div className="mt-4 text-sm">
              <div className="text-[var(--color-muted)] mb-1">Assigned but unscheduled (no brew date):</div>
              <ul className="flex flex-wrap gap-2">
                {unscheduled.map((b) => (
                  <li key={b.id}>
                    <Link to={`/batches/${b.id}`} className="px-2 py-1 rounded border border-[var(--color-border)] text-[var(--color-fg)] hover:bg-[var(--color-border)]">
                      {b.name}
                    </Link>
                  </li>
                ))}
              </ul>
            </div>
          )}
        </>
      )}
    </div>
  )
}
