import React from 'react'
import { Link } from 'react-router-dom'
import { useMaintenanceDue } from './hooks/useEquipment'

function fmtDate(s: string | null | undefined): string {
  if (!s) return '—'
  return String(s).slice(0, 10)
}

export default function MaintenanceDuePage() {
  const [windowDays, setWindowDays] = React.useState(30)
  const [overdueOnly, setOverdueOnly] = React.useState(false)
  const { data, isLoading, error } = useMaintenanceDue({ window_days: windowDays, overdue_only: overdueOnly })

  return (
    <div className="p-6 max-w-5xl mx-auto">
      <div className="flex justify-between items-center mb-4">
        <h1 className="text-xl font-semibold">Maintenance Due</h1>
        <div className="flex gap-3 items-center text-sm">
          <label className="flex items-center gap-1">
            <span className="text-[var(--color-muted)]">Window</span>
            <select
              className="border rounded px-2 py-1 text-sm"
              value={windowDays}
              onChange={(e) => setWindowDays(Number(e.target.value))}
            >
              <option value={7}>7 days</option>
              <option value={30}>30 days</option>
              <option value={90}>90 days</option>
              <option value={365}>1 year</option>
            </select>
          </label>
          <label className="flex items-center gap-1">
            <input type="checkbox" checked={overdueOnly} onChange={(e) => setOverdueOnly(e.target.checked)} />
            <span className="text-[var(--color-muted)]">Overdue only</span>
          </label>
        </div>
      </div>

      {isLoading && <p className="text-[var(--color-muted)]">Loading…</p>}
      {error && <p className="text-[var(--color-danger)]">Failed to load maintenance feed.</p>}

      {data && data.items && data.items.length === 0 && (
        <p className="text-[var(--color-muted)] text-sm">Nothing due in this window.</p>
      )}

      {data && data.items && data.items.length > 0 && (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-[var(--color-muted)] border-b">
                <th className="py-2 pr-3">Equipment</th>
                <th className="pr-3">Type</th>
                <th className="pr-3">Task</th>
                <th className="pr-3">Every</th>
                <th className="pr-3">Last done</th>
                <th className="pr-3">Next due</th>
                <th className="pr-3">Status</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-[var(--color-border)]">
              {data.items.map((it) => (
                <tr key={it.schedule_id}>
                  <td className="py-2 pr-3">
                    <Link className="text-[var(--color-accent)] hover:underline" to="/equipment">
                      {it.equipment_name}
                    </Link>
                  </td>
                  <td className="pr-3">{it.equipment_type}</td>
                  <td className="pr-3">{it.task_name}</td>
                  <td className="pr-3">{it.interval_days}d</td>
                  <td className="pr-3">{fmtDate(it.last_performed_at)}</td>
                  <td className="pr-3">{fmtDate(it.next_due_at)}</td>
                  <td className="pr-3">
                    {it.is_overdue
                      ? <span className="text-[var(--color-danger)] font-medium">Overdue {Math.abs(it.days_until_due ?? 0)}d</span>
                      : <span className="text-[var(--color-muted)]">in {it.days_until_due ?? 0}d</span>}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
