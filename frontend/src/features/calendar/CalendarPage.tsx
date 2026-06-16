import React from 'react'
import { useCalendarEvents, useCreateCalendarEvent, useUpdateCalendarEvent, useDeleteCalendarEvent, useCompleteCalendarEvent } from './hooks/useCalendar'
import { APIError } from '../../api/error'
import type { components } from '../../api/generated'

type CalendarEvent = components['schemas']['CalendarEvent']

const EVENT_TYPE_OPTIONS = [
  'brew_day', 'dry_hop', 'fermentation_complete', 'transfer', 'package', 'condition_complete', 'custom',
] as const

const EVENT_COLORS: Record<string, string> = {
  brew_day: 'var(--srm-5)',
  dry_hop: 'var(--srm-7)',
  fermentation_complete: 'var(--srm-6)',
  transfer: 'var(--srm-4)',
  package: 'var(--srm-8)',
  condition_complete: 'var(--srm-3)',
  custom: 'var(--color-accent)',
}

function getDaysInMonth(year: number, month: number) {
  return new Date(year, month + 1, 0).getDate()
}

function getFirstDayOfWeek(year: number, month: number) {
  return new Date(year, month, 1).getDay()
}

type ModalMode = 'view' | 'create' | 'edit' | null

export function CalendarPage() {
  const today = new Date()
  const [year, setYear] = React.useState(today.getFullYear())
  const [month, setMonth] = React.useState(today.getMonth())
  const [view, setView] = React.useState<'month' | 'list'>('month')
  const [modal, setModal] = React.useState<ModalMode>(null)
  const [selectedEvent, setSelectedEvent] = React.useState<CalendarEvent | null>(null)

  // Form state for create/edit
  const [formTitle, setFormTitle] = React.useState('')
  const [formEventType, setFormEventType] = React.useState<typeof EVENT_TYPE_OPTIONS[number]>('custom')
  const [formStartTime, setFormStartTime] = React.useState('')
  const [formEndTime, setFormEndTime] = React.useState('')
  const [formNotes, setFormNotes] = React.useState('')

  const monthStart = new Date(year, month, 1).toISOString()
  const monthEnd = new Date(year, month + 1, 0, 23, 59, 59).toISOString()

  const { data, refetch } = useCalendarEvents({ from: monthStart, to: monthEnd, page_size: 200 })
  const { mutate: createEvent, isPending: isCreating, isError: isCreateError, error: createError } = useCreateCalendarEvent()
  const { mutate: updateEvent, isPending: isUpdating } = useUpdateCalendarEvent(selectedEvent?.id ?? '')
  const { mutate: deleteEvent, isPending: isDeleting } = useDeleteCalendarEvent(selectedEvent?.id ?? '')
  const { mutate: completeEvent, isPending: isCompleting } = useCompleteCalendarEvent(selectedEvent?.id ?? '')

  const events = data?.items ?? []

  const eventsByDate = React.useMemo(() => {
    const map: Record<string, CalendarEvent[]> = {}
    for (const ev of events) {
      if (!ev.start_time) continue
      const d = new Date(ev.start_time)
      const key = `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`
      if (!map[key]) map[key] = []
      map[key].push(ev)
    }
    return map
  }, [events])

  const prevMonth = () => {
    if (month === 0) { setYear(y => y - 1); setMonth(11) }
    else setMonth(m => m - 1)
  }
  const nextMonth = () => {
    if (month === 11) { setYear(y => y + 1); setMonth(0) }
    else setMonth(m => m + 1)
  }

  const openCreate = (dateStr?: string) => {
    setSelectedEvent(null)
    setFormTitle('')
    setFormEventType('custom')
    setFormStartTime(dateStr ? `${dateStr}T09:00` : '')
    setFormEndTime('')
    setFormNotes('')
    setModal('create')
  }

  const openView = (ev: CalendarEvent) => {
    setSelectedEvent(ev)
    setModal('view')
  }

  const openEdit = (ev: CalendarEvent) => {
    setSelectedEvent(ev)
    setFormTitle(ev.title ?? '')
    setFormEventType((ev.event_type ?? 'custom') as typeof EVENT_TYPE_OPTIONS[number])
    setFormStartTime((ev.start_time ?? '').slice(0, 16))
    setFormEndTime(ev.end_time ? ev.end_time.slice(0, 16) : '')
    setFormNotes(ev.notes ?? '')
    setModal('edit')
  }

  const closeModal = () => {
    setModal(null)
    setSelectedEvent(null)
  }

  const handleCreate = () => {
    if (!formTitle || !formStartTime) return
    createEvent({
      event_type: formEventType,
      title: formTitle,
      start_time: new Date(formStartTime).toISOString(),
      end_time: formEndTime ? new Date(formEndTime).toISOString() : null,
      notes: formNotes || null,
      status: 'pending',
    }, {
      onSuccess: () => { closeModal(); refetch() },
    })
  }

  const handleUpdate = () => {
    if (!formTitle || !formStartTime) return
    updateEvent({
      title: formTitle,
      start_time: new Date(formStartTime).toISOString(),
      end_time: formEndTime ? new Date(formEndTime).toISOString() : null,
      notes: formNotes || null,
    }, {
      onSuccess: () => { closeModal(); refetch() },
    })
  }

  const handleDelete = () => {
    if (!selectedEvent) return
    deleteEvent(undefined, { onSuccess: () => { closeModal(); refetch() } })
  }

  const handleComplete = () => {
    if (!selectedEvent) return
    completeEvent(undefined, { onSuccess: () => { closeModal(); refetch() } })
  }

  const daysInMonth = getDaysInMonth(year, month)
  const firstDow = getFirstDayOfWeek(year, month)
  const monthName = new Date(year, month, 1).toLocaleString('default', { month: 'long' })

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-bold text-[var(--color-fg)]">Calendar</h1>
        <div className="flex items-center gap-3">
          <div className="flex rounded border border-[var(--color-border)] overflow-hidden text-sm">
            <button
              onClick={() => setView('month')}
              className="px-3 py-1"
              style={{ background: view === 'month' ? 'var(--color-accent)' : 'var(--color-surface)', color: view === 'month' ? 'white' : 'var(--color-fg)' }}
            >
              Month
            </button>
            <button
              onClick={() => setView('list')}
              className="px-3 py-1"
              style={{ background: view === 'list' ? 'var(--color-accent)' : 'var(--color-surface)', color: view === 'list' ? 'white' : 'var(--color-fg)' }}
            >
              List
            </button>
          </div>
          <button
            onClick={() => openCreate()}
            className="px-4 py-1.5 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90"
          >
            + New event
          </button>
        </div>
      </div>

      {/* Month nav */}
      <div className="flex items-center gap-4 mb-4">
        <button onClick={prevMonth} className="p-1 rounded hover:bg-[var(--color-surface)] text-[var(--color-muted)]">‹</button>
        <span className="text-lg font-semibold text-[var(--color-fg)] min-w-[160px] text-center">
          {monthName} {year}
        </span>
        <button onClick={nextMonth} className="p-1 rounded hover:bg-[var(--color-surface)] text-[var(--color-muted)]">›</button>
        <button
          onClick={() => { setYear(today.getFullYear()); setMonth(today.getMonth()) }}
          className="text-xs text-[var(--color-muted)] hover:text-[var(--color-fg)] ml-2"
        >
          Today
        </button>
      </div>

      {view === 'month' ? (
        <div className="rounded border border-[var(--color-border)] overflow-hidden">
          {/* Day headers */}
          <div className="grid grid-cols-7 border-b border-[var(--color-border)]">
            {['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'].map((d) => (
              <div key={d} className="py-2 text-center text-xs font-medium text-[var(--color-muted)] bg-[var(--color-surface)]">
                {d}
              </div>
            ))}
          </div>

          {/* Calendar grid */}
          <div className="grid grid-cols-7">
            {[...Array(firstDow)].map((_, i) => (
              <div key={`empty-${i}`} className="min-h-[80px] border-b border-r border-[var(--color-border)]" />
            ))}
            {[...Array(daysInMonth)].map((_, i) => {
              const day = i + 1
              const isToday = year === today.getFullYear() && month === today.getMonth() && day === today.getDate()
              const key = `${year}-${month}-${day}`
              const dayEvents = eventsByDate[key] ?? []
              const dateStr = `${year}-${String(month + 1).padStart(2, '0')}-${String(day).padStart(2, '0')}`

              return (
                <div
                  key={day}
                  className="min-h-[80px] border-b border-r border-[var(--color-border)] p-1 cursor-pointer hover:bg-[var(--color-surface)]"
                  onClick={() => openCreate(dateStr)}
                >
                  <div
                    className="text-xs font-medium mb-1 w-6 h-6 flex items-center justify-center rounded-full"
                    style={isToday ? { background: 'var(--color-accent)', color: 'white' } : { color: 'var(--color-muted)' }}
                  >
                    {day}
                  </div>
                  {dayEvents.slice(0, 3).map((ev) => (
                    <div
                      key={ev.id}
                      onClick={(e) => { e.stopPropagation(); openView(ev) }}
                      className="text-xs px-1 py-0.5 rounded mb-0.5 truncate text-white cursor-pointer"
                      style={{ background: EVENT_COLORS[ev.event_type ?? ''] ?? 'var(--color-accent)' }}
                    >
                      {ev.title}
                    </div>
                  ))}
                  {dayEvents.length > 3 && (
                    <div className="text-xs text-[var(--color-muted)] px-1">+{dayEvents.length - 3} more</div>
                  )}
                </div>
              )
            })}
          </div>
        </div>
      ) : (
        /* List view */
        <div className="space-y-2">
          {events.length === 0 && (
            <p className="text-[var(--color-muted)] text-sm py-8 text-center">No events this month.</p>
          )}
          {events
            .slice()
            .sort((a, b) => (a.start_time ?? '').localeCompare(b.start_time ?? ''))
            .map((ev) => (
              <div
                key={ev.id}
                onClick={() => openView(ev)}
                className="flex items-center gap-4 p-3 rounded border border-[var(--color-border)] bg-[var(--color-surface)] cursor-pointer hover:opacity-90"
              >
                <div className="w-2 h-8 rounded-full flex-shrink-0" style={{ background: EVENT_COLORS[ev.event_type ?? ''] ?? 'var(--color-accent)' }} />
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-[var(--color-fg)] truncate">{ev.title}</p>
                  <p className="text-xs text-[var(--color-muted)]">{ev.event_type}</p>
                </div>
                <div className="text-xs text-[var(--color-muted)] flex-shrink-0">
                  {ev.start_time ? new Date(ev.start_time).toLocaleDateString() : '—'}{' '}
                  {ev.start_time ? new Date(ev.start_time).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) : ''}
                </div>
                <span
                  className="px-1.5 py-0.5 rounded text-xs text-white flex-shrink-0"
                  style={{
                    background: ev.status === 'completed' ? 'var(--color-success)' : ev.status === 'skipped' ? 'var(--color-muted)' : 'var(--color-warning)',
                  }}
                >
                  {ev.status}
                </span>
              </div>
            ))}
        </div>
      )}

      {/* Event modal */}
      {modal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
          <div className="bg-[var(--color-surface)] rounded-lg border border-[var(--color-border)] p-6 max-w-md w-full mx-4 shadow-lg">
            {modal === 'view' && selectedEvent && (
              <>
                <div className="flex items-start justify-between mb-4">
                  <div>
                    <h2 className="text-lg font-semibold text-[var(--color-fg)]">{selectedEvent.title}</h2>
                    <p className="text-xs text-[var(--color-muted)] mt-0.5">{selectedEvent.event_type}</p>
                  </div>
                  <span
                    className="px-2 py-0.5 rounded text-xs text-white"
                    style={{
                      background: selectedEvent.status === 'completed' ? 'var(--color-success)' : selectedEvent.status === 'skipped' ? 'var(--color-muted)' : 'var(--color-warning)',
                    }}
                  >
                    {selectedEvent.status}
                  </span>
                </div>
                <p className="text-sm text-[var(--color-muted)] mb-1">
                  {selectedEvent.start_time ? new Date(selectedEvent.start_time).toLocaleString() : '—'}
                  {selectedEvent.end_time ? ` – ${new Date(selectedEvent.end_time).toLocaleString()}` : ''}
                </p>
                {selectedEvent.notify_minutes_before && (
                  <p className="text-xs text-[var(--color-muted)] mb-2">
                    Reminder: {selectedEvent.notify_minutes_before} min before
                  </p>
                )}
                {selectedEvent.notes && (
                  <p className="text-sm text-[var(--color-fg)] mt-2 whitespace-pre-wrap">{selectedEvent.notes}</p>
                )}
                <div className="flex gap-2 mt-5 justify-end">
                  {selectedEvent.status === 'pending' && (
                    <button
                      onClick={handleComplete}
                      disabled={isCompleting}
                      className="px-3 py-1.5 rounded text-sm bg-[var(--color-success)] text-white disabled:opacity-50"
                    >
                      {isCompleting ? 'Marking…' : 'Mark complete'}
                    </button>
                  )}
                  <button
                    onClick={() => openEdit(selectedEvent)}
                    className="px-3 py-1.5 rounded text-sm border border-[var(--color-border)] text-[var(--color-fg)]"
                  >
                    Edit
                  </button>
                  <button
                    onClick={closeModal}
                    className="px-3 py-1.5 rounded text-sm text-[var(--color-muted)]"
                  >
                    Close
                  </button>
                </div>
              </>
            )}

            {(modal === 'create' || modal === 'edit') && (
              <>
                <h2 className="text-lg font-semibold text-[var(--color-fg)] mb-4">
                  {modal === 'create' ? 'New Event' : 'Edit Event'}
                </h2>

                {isCreateError && (
                  <div className="mb-3 p-3 rounded border border-[var(--color-danger)] text-[var(--color-danger)] text-sm">
                    {createError instanceof APIError ? createError.message : createError instanceof Error ? createError.message : 'Error'}
                  </div>
                )}

                <div className="space-y-3">
                  <div className="flex flex-col gap-1">
                    <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Title *</label>
                    <input
                      type="text"
                      value={formTitle}
                      onChange={(e) => setFormTitle(e.target.value)}
                      className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-bg)] text-[var(--color-fg)]"
                    />
                  </div>
                  <div className="flex flex-col gap-1">
                    <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Type</label>
                    <select
                      value={formEventType}
                      onChange={(e) => setFormEventType(e.target.value as typeof EVENT_TYPE_OPTIONS[number])}
                      className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-bg)] text-[var(--color-fg)]"
                    >
                      {EVENT_TYPE_OPTIONS.map((t) => <option key={t} value={t}>{t}</option>)}
                    </select>
                  </div>
                  <div className="flex flex-col gap-1">
                    <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Start *</label>
                    <input
                      type="datetime-local"
                      value={formStartTime}
                      onChange={(e) => setFormStartTime(e.target.value)}
                      className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-bg)] text-[var(--color-fg)]"
                    />
                  </div>
                  <div className="flex flex-col gap-1">
                    <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">End</label>
                    <input
                      type="datetime-local"
                      value={formEndTime}
                      onChange={(e) => setFormEndTime(e.target.value)}
                      className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-bg)] text-[var(--color-fg)]"
                    />
                  </div>
                  <div className="flex flex-col gap-1">
                    <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Notes</label>
                    <textarea
                      value={formNotes}
                      onChange={(e) => setFormNotes(e.target.value)}
                      rows={2}
                      className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-bg)] text-[var(--color-fg)]"
                    />
                  </div>
                </div>

                <div className="flex gap-2 mt-5 justify-end">
                  {modal === 'edit' && (
                    <button
                      onClick={handleDelete}
                      disabled={isDeleting}
                      className="px-3 py-1.5 rounded text-sm bg-[var(--color-danger)] text-white disabled:opacity-50 mr-auto"
                    >
                      {isDeleting ? 'Deleting…' : 'Delete'}
                    </button>
                  )}
                  <button onClick={closeModal} className="px-3 py-1.5 rounded text-sm text-[var(--color-muted)]">
                    Cancel
                  </button>
                  <button
                    onClick={modal === 'create' ? handleCreate : handleUpdate}
                    disabled={isCreating || isUpdating || !formTitle || !formStartTime}
                    className="px-4 py-1.5 rounded text-sm bg-[var(--color-accent)] text-white disabled:opacity-50"
                  >
                    {isCreating || isUpdating ? 'Saving…' : 'Save'}
                  </button>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </div>
  )
}
