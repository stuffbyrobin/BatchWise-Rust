import React from 'react'
import { APIError } from '../../api/error'
import {
  useEquipmentList, useCreateEquipment, usePatchEquipment, useDeleteEquipment,
  useSchedules, useCreateSchedule, usePatchSchedule, useDeleteSchedule,
  useEvents, useCreateEvent, useDeleteEvent,
} from './hooks/useEquipment'
import type { components } from '../../api/generated'

type Equipment = components['schemas']['Equipment']
type MaintenanceSchedule = components['schemas']['MaintenanceSchedule']
type MaintenanceEvent = components['schemas']['MaintenanceEvent']

const EVENT_TYPES = ['service', 'calibration', 'repair', 'inspection', 'cleaning', 'other'] as const

function fmtDate(s: string | null | undefined): string {
  if (!s) return '—'
  return String(s).slice(0, 10)
}

function DueBadge({ schedule }: { schedule: MaintenanceSchedule }) {
  const days = schedule.days_until_due ?? 0
  if (schedule.is_overdue) {
    return <span className="text-[var(--color-danger)] font-medium">Overdue {Math.abs(days)}d</span>
  }
  return <span className="text-[var(--color-muted)]">in {days}d</span>
}

// ——— Schedules panel —————————————————————————————————————————————————————————

function SchedulesPanel({ equipment }: { equipment: Equipment }) {
  const equipmentID = equipment.id ?? ''
  const { data, isLoading } = useSchedules(equipmentID)
  const create = useCreateSchedule(equipmentID)
  const [showForm, setShowForm] = React.useState(false)
  const [form, setForm] = React.useState({ task_name: '', interval_days: '', last_performed_at: '' })
  const [err, setErr] = React.useState<string | null>(null)

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault()
    setErr(null)
    try {
      await create.mutateAsync({
        task_name: form.task_name,
        interval_days: Number(form.interval_days),
        ...(form.last_performed_at ? { last_performed_at: new Date(form.last_performed_at).toISOString() } : {}),
      })
      setForm({ task_name: '', interval_days: '', last_performed_at: '' })
      setShowForm(false)
    } catch (e) {
      setErr(e instanceof APIError ? e.message : 'Failed to create schedule.')
    }
  }

  return (
    <div className="mt-2 pl-4 border-l-2 border-[var(--color-border)]">
      <div className="text-xs font-medium text-[var(--color-muted)] mb-1">Maintenance schedules</div>
      {isLoading ? (
        <p className="text-xs text-[var(--color-muted)]">Loading schedules…</p>
      ) : data && data.items && data.items.length > 0 ? (
        <table className="w-full text-xs mb-2">
          <thead>
            <tr className="text-left text-[var(--color-muted)]">
              <th className="pr-3">Task</th>
              <th className="pr-3">Every</th>
              <th className="pr-3">Last done</th>
              <th className="pr-3">Next due</th>
              <th className="pr-3">Status</th>
              <th className="pr-3">Active</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {data.items.map((s) => (
              <ScheduleRow key={s.id} schedule={s} equipmentID={equipmentID} />
            ))}
          </tbody>
        </table>
      ) : (
        <p className="text-xs text-[var(--color-muted)] mb-2">No schedules yet.</p>
      )}

      {!showForm ? (
        <button
          className="text-xs px-2 py-1 rounded border border-[var(--color-border)] hover:bg-[var(--color-surface-alt)]"
          onClick={() => setShowForm(true)}
        >
          + Add Schedule
        </button>
      ) : (
        <form onSubmit={handleCreate} className="flex flex-wrap gap-2 items-end text-xs mt-1">
          <div>
            <label className="block text-[var(--color-muted)] mb-0.5">Task *</label>
            <input className="border rounded px-2 py-1 text-xs" placeholder="Calibrate load cell" required
              value={form.task_name} onChange={(e) => setForm((f) => ({ ...f, task_name: e.target.value }))} />
          </div>
          <div>
            <label className="block text-[var(--color-muted)] mb-0.5">Interval (days) *</label>
            <input className="border rounded px-2 py-1 w-20 text-xs" type="number" min={1} placeholder="90" required
              value={form.interval_days} onChange={(e) => setForm((f) => ({ ...f, interval_days: e.target.value }))} />
          </div>
          <div>
            <label className="block text-[var(--color-muted)] mb-0.5">Last performed</label>
            <input className="border rounded px-2 py-1 text-xs" type="date"
              value={form.last_performed_at} onChange={(e) => setForm((f) => ({ ...f, last_performed_at: e.target.value }))} />
          </div>
          <button type="submit"
            className="px-2 py-1 rounded bg-[var(--color-accent)] text-white text-xs disabled:opacity-50"
            disabled={create.isPending}>
            {create.isPending ? 'Saving…' : 'Save'}
          </button>
          <button type="button" className="px-2 py-1 rounded border text-xs" onClick={() => setShowForm(false)}>
            Cancel
          </button>
          {err && <span className="text-[var(--color-danger)] w-full">{err}</span>}
        </form>
      )}
    </div>
  )
}

function ScheduleRow({ schedule, equipmentID }: { schedule: MaintenanceSchedule; equipmentID: string }) {
  const patch = usePatchSchedule(equipmentID, schedule.id ?? '')
  const del = useDeleteSchedule(equipmentID)

  return (
    <tr>
      <td className="pr-3">{schedule.task_name}</td>
      <td className="pr-3">{schedule.interval_days}d</td>
      <td className="pr-3">{fmtDate(schedule.last_performed_at)}</td>
      <td className="pr-3">{fmtDate(schedule.next_due_at)}</td>
      <td className="pr-3"><DueBadge schedule={schedule} /></td>
      <td className="pr-3">
        <button
          className="text-xs hover:underline"
          onClick={() => patch.mutate({ active: !schedule.active })}
          disabled={patch.isPending}
        >
          {schedule.active ? 'Active' : 'Paused'}
        </button>
      </td>
      <td>
        <button
          className="text-[var(--color-danger)] hover:underline text-xs"
          onClick={() => del.mutate(schedule.id ?? '')}
          disabled={del.isPending}
        >
          Delete
        </button>
      </td>
    </tr>
  )
}

// ——— Events panel ————————————————————————————————————————————————————————————

function EventsPanel({ equipment }: { equipment: Equipment }) {
  const equipmentID = equipment.id ?? ''
  const { data, isLoading } = useEvents(equipmentID)
  const { data: schedData } = useSchedules(equipmentID)
  const create = useCreateEvent(equipmentID)
  const [showForm, setShowForm] = React.useState(false)
  const [form, setForm] = React.useState({ event_type: 'service', schedule_id: '', performed_at: '', performed_by: '', cost_pence: '', notes: '' })
  const [err, setErr] = React.useState<string | null>(null)

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault()
    setErr(null)
    try {
      await create.mutateAsync({
        event_type: form.event_type as NonNullable<MaintenanceEvent['event_type']>,
        ...(form.schedule_id ? { schedule_id: form.schedule_id } : {}),
        ...(form.performed_at ? { performed_at: new Date(form.performed_at).toISOString() } : {}),
        ...(form.performed_by ? { performed_by: form.performed_by } : {}),
        ...(form.cost_pence ? { cost_pence: Math.round(Number(form.cost_pence) * 100) } : {}),
        ...(form.notes ? { notes: form.notes } : {}),
      })
      setForm({ event_type: 'service', schedule_id: '', performed_at: '', performed_by: '', cost_pence: '', notes: '' })
      setShowForm(false)
    } catch (e) {
      setErr(e instanceof APIError ? e.message : 'Failed to log event.')
    }
  }

  return (
    <div className="mt-3 pl-4 border-l-2 border-[var(--color-border)]">
      <div className="text-xs font-medium text-[var(--color-muted)] mb-1">Maintenance log</div>
      {isLoading ? (
        <p className="text-xs text-[var(--color-muted)]">Loading events…</p>
      ) : data && data.items && data.items.length > 0 ? (
        <table className="w-full text-xs mb-2">
          <thead>
            <tr className="text-left text-[var(--color-muted)]">
              <th className="pr-3">Type</th>
              <th className="pr-3">Performed</th>
              <th className="pr-3">By</th>
              <th className="pr-3">Cost</th>
              <th className="pr-3">Notes</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {data.items.map((ev) => <EventRow key={ev.id} event={ev} equipmentID={equipmentID} />)}
          </tbody>
        </table>
      ) : (
        <p className="text-xs text-[var(--color-muted)] mb-2">No events logged yet.</p>
      )}

      {!showForm ? (
        <button
          className="text-xs px-2 py-1 rounded border border-[var(--color-border)] hover:bg-[var(--color-surface-alt)]"
          onClick={() => setShowForm(true)}
        >
          + Log Event
        </button>
      ) : (
        <form onSubmit={handleCreate} className="flex flex-wrap gap-2 items-end text-xs mt-1">
          <div>
            <label className="block text-[var(--color-muted)] mb-0.5">Type *</label>
            <select className="border rounded px-2 py-1 text-xs" value={form.event_type}
              onChange={(e) => setForm((f) => ({ ...f, event_type: e.target.value }))}>
              {EVENT_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
            </select>
          </div>
          <div>
            <label className="block text-[var(--color-muted)] mb-0.5">Against schedule</label>
            <select className="border rounded px-2 py-1 text-xs" value={form.schedule_id}
              onChange={(e) => setForm((f) => ({ ...f, schedule_id: e.target.value }))}>
              <option value="">None (ad-hoc)</option>
              {(schedData?.items ?? []).map((s) => <option key={s.id} value={s.id}>{s.task_name}</option>)}
            </select>
          </div>
          <div>
            <label className="block text-[var(--color-muted)] mb-0.5">Performed</label>
            <input className="border rounded px-2 py-1 text-xs" type="date"
              value={form.performed_at} onChange={(e) => setForm((f) => ({ ...f, performed_at: e.target.value }))} />
          </div>
          <div>
            <label className="block text-[var(--color-muted)] mb-0.5">By</label>
            <input className="border rounded px-2 py-1 text-xs w-24" placeholder="Sam"
              value={form.performed_by} onChange={(e) => setForm((f) => ({ ...f, performed_by: e.target.value }))} />
          </div>
          <div>
            <label className="block text-[var(--color-muted)] mb-0.5">Cost (£)</label>
            <input className="border rounded px-2 py-1 w-20 text-xs" type="number" min={0} step="0.01" placeholder="45.00"
              value={form.cost_pence} onChange={(e) => setForm((f) => ({ ...f, cost_pence: e.target.value }))} />
          </div>
          <div>
            <label className="block text-[var(--color-muted)] mb-0.5">Notes</label>
            <input className="border rounded px-2 py-1 text-xs" placeholder="Optional"
              value={form.notes} onChange={(e) => setForm((f) => ({ ...f, notes: e.target.value }))} />
          </div>
          <button type="submit"
            className="px-2 py-1 rounded bg-[var(--color-accent)] text-white text-xs disabled:opacity-50"
            disabled={create.isPending}>
            {create.isPending ? 'Saving…' : 'Save'}
          </button>
          <button type="button" className="px-2 py-1 rounded border text-xs" onClick={() => setShowForm(false)}>
            Cancel
          </button>
          {err && <span className="text-[var(--color-danger)] w-full">{err}</span>}
        </form>
      )}
    </div>
  )
}

function EventRow({ event, equipmentID }: { event: MaintenanceEvent; equipmentID: string }) {
  const del = useDeleteEvent(equipmentID)
  const cost = event.cost_pence != null ? `£${(event.cost_pence / 100).toFixed(2)}` : '—'
  return (
    <tr>
      <td className="pr-3">{event.event_type}</td>
      <td className="pr-3">{fmtDate(event.performed_at)}</td>
      <td className="pr-3">{event.performed_by || '—'}</td>
      <td className="pr-3">{cost}</td>
      <td className="pr-3">{event.notes || '—'}</td>
      <td>
        <button
          className="text-[var(--color-danger)] hover:underline text-xs"
          onClick={() => del.mutate(event.id ?? '')}
          disabled={del.isPending}
        >
          Delete
        </button>
      </td>
    </tr>
  )
}

// ——— Equipment row ————————————————————————————————————————————————————————————

function EquipmentRow({ equipment }: { equipment: Equipment }) {
  const [expanded, setExpanded] = React.useState(false)
  const patch = usePatchEquipment(equipment.id ?? '')
  const del = useDeleteEquipment()
  const [delErr, setDelErr] = React.useState<string | null>(null)
  const overdue = equipment.overdue_schedule_count ?? 0

  return (
    <>
      <tr>
        <td className="py-2 pr-3">
          <button
            className="text-xs text-[var(--color-accent)] hover:underline mr-1"
            onClick={() => setExpanded((x) => !x)}
          >
            {expanded ? '▲' : '▼'}
          </button>
          {equipment.name}
        </td>
        <td className="pr-3">{equipment.equipment_type}</td>
        <td className="pr-3">
          <span className={equipment.status === 'retired' ? 'text-[var(--color-muted)]' : 'text-green-600'}>
            {equipment.status}
          </span>
        </td>
        <td className="pr-3">{equipment.location || '—'}</td>
        <td className="pr-3">{fmtDate(equipment.next_maintenance_due_at)}</td>
        <td className="pr-3">
          {overdue > 0
            ? <span className="text-[var(--color-danger)] font-medium">{overdue} overdue</span>
            : <span className="text-[var(--color-muted)]">none</span>}
        </td>
        <td className="pr-3 text-xs flex gap-2 items-center flex-wrap">
          <button
            className="hover:underline"
            onClick={() => patch.mutate({ status: equipment.status === 'retired' ? 'active' : 'retired' })}
            disabled={patch.isPending}
          >
            {equipment.status === 'retired' ? 'Reactivate' : 'Retire'}
          </button>
          <button
            className="text-[var(--color-danger)] hover:underline disabled:opacity-50"
            disabled={del.isPending}
            onClick={async () => {
              setDelErr(null)
              try { await del.mutateAsync(equipment.id ?? '') }
              catch (e) { setDelErr(e instanceof APIError ? e.message : 'Delete failed.') }
            }}
          >
            Delete
          </button>
          {delErr && <span className="text-[var(--color-danger)]">{delErr}</span>}
        </td>
      </tr>
      {expanded && (
        <tr>
          <td colSpan={7}>
            <SchedulesPanel equipment={equipment} />
            <EventsPanel equipment={equipment} />
          </td>
        </tr>
      )}
    </>
  )
}

// ——— Page ————————————————————————————————————————————————————————————————————

export default function EquipmentPage() {
  const [statusFilter, setStatusFilter] = React.useState('')
  const [typeFilter, setTypeFilter] = React.useState('')
  const { data, isLoading, error } = useEquipmentList({
    status: statusFilter || undefined,
    equipment_type: typeFilter || undefined,
  })
  const create = useCreateEquipment()
  const [showForm, setShowForm] = React.useState(false)
  const [form, setForm] = React.useState({ name: '', equipment_type: '', serial_number: '', location: '', purchased_at: '', notes: '' })
  const [formErr, setFormErr] = React.useState<string | null>(null)

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault()
    setFormErr(null)
    try {
      await create.mutateAsync({
        name: form.name,
        equipment_type: form.equipment_type,
        ...(form.serial_number ? { serial_number: form.serial_number } : {}),
        ...(form.location ? { location: form.location } : {}),
        ...(form.purchased_at ? { purchased_at: form.purchased_at } : {}),
        ...(form.notes ? { notes: form.notes } : {}),
      })
      setForm({ name: '', equipment_type: '', serial_number: '', location: '', purchased_at: '', notes: '' })
      setShowForm(false)
    } catch (e) {
      setFormErr(e instanceof APIError ? e.message : 'Failed to create equipment.')
    }
  }

  return (
    <div className="p-6 max-w-6xl mx-auto">
      <div className="flex justify-between items-center mb-4">
        <h1 className="text-xl font-semibold">Equipment</h1>
        <div className="flex gap-2 items-center">
          <input
            className="border rounded px-2 py-1 text-sm w-32"
            placeholder="Filter type…"
            value={typeFilter}
            onChange={(e) => setTypeFilter(e.target.value)}
          />
          <select
            className="border rounded px-2 py-1 text-sm"
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value)}
          >
            <option value="">All statuses</option>
            <option value="active">Active</option>
            <option value="retired">Retired</option>
          </select>
          <button
            className="px-3 py-1.5 rounded bg-[var(--color-accent)] text-white text-sm hover:opacity-90"
            onClick={() => setShowForm((x) => !x)}
          >
            {showForm ? 'Cancel' : '+ New Equipment'}
          </button>
        </div>
      </div>

      {showForm && (
        <form onSubmit={handleCreate}
          className="mb-6 p-4 border rounded grid grid-cols-2 md:grid-cols-3 gap-3 text-sm bg-[var(--color-surface)]">
          <div className="col-span-2 md:col-span-3 font-medium">New Equipment</div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Name *</label>
            <input className="w-full border rounded px-2 py-1 text-sm" placeholder="Fermenter FV3" required
              value={form.name} onChange={(e) => setForm((f) => ({ ...f, name: e.target.value }))} />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Type *</label>
            <input className="w-full border rounded px-2 py-1 text-sm" placeholder="fermenter" required
              value={form.equipment_type} onChange={(e) => setForm((f) => ({ ...f, equipment_type: e.target.value }))} />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Serial number</label>
            <input className="w-full border rounded px-2 py-1 text-sm" placeholder="SS-9001"
              value={form.serial_number} onChange={(e) => setForm((f) => ({ ...f, serial_number: e.target.value }))} />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Location</label>
            <input className="w-full border rounded px-2 py-1 text-sm" placeholder="Cellar bay 2"
              value={form.location} onChange={(e) => setForm((f) => ({ ...f, location: e.target.value }))} />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Purchased</label>
            <input className="w-full border rounded px-2 py-1 text-sm" type="date"
              value={form.purchased_at} onChange={(e) => setForm((f) => ({ ...f, purchased_at: e.target.value }))} />
          </div>
          <div>
            <label className="block text-xs text-[var(--color-muted)] mb-1">Notes</label>
            <input className="w-full border rounded px-2 py-1 text-sm"
              value={form.notes} onChange={(e) => setForm((f) => ({ ...f, notes: e.target.value }))} />
          </div>
          {formErr && <div className="col-span-2 md:col-span-3 text-xs text-[var(--color-danger)]">{formErr}</div>}
          <div className="col-span-2 md:col-span-3">
            <button type="submit" disabled={create.isPending}
              className="px-3 py-1.5 rounded bg-[var(--color-accent)] text-white text-sm disabled:opacity-50">
              {create.isPending ? 'Creating…' : 'Create Equipment'}
            </button>
          </div>
        </form>
      )}

      {isLoading && <p className="text-[var(--color-muted)]">Loading…</p>}
      {error && <p className="text-[var(--color-danger)]">Failed to load equipment.</p>}

      {data && data.items && data.items.length === 0 && (
        <p className="text-[var(--color-muted)] text-sm">No equipment yet.</p>
      )}

      {data && data.items && data.items.length > 0 && (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-[var(--color-muted)] border-b">
                <th className="py-2 pr-3">Name</th>
                <th className="pr-3">Type</th>
                <th className="pr-3">Status</th>
                <th className="pr-3">Location</th>
                <th className="pr-3">Next due</th>
                <th className="pr-3">Maintenance</th>
                <th className="pr-3">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-[var(--color-border)]">
              {data.items.map((e) => <EquipmentRow key={e.id} equipment={e} />)}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
