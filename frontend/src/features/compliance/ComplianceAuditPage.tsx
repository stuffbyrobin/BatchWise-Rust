import { useState } from 'react'
import { useAuditEvents } from './hooks/useAudit'
import type { AuditParams } from './hooks/useAudit'
import type { components } from '../../api/generated'

type AuditEvent = components['schemas']['AuditEvent']

const EVENT_TYPES = [
  'label_record.created',
  'label_record.updated',
  'label_record.approved',
  'label_record.deleted',
  'duty_return.compiled',
  'duty_return.submitted',
  'allergen_result.computed',
  'packaging_run.created',
  'packaging_run.deleted',
  'distribution_movement.created',
  'distribution_movement.deleted',
  'recall.queried',
]

const ENTITY_TYPES = [
  'label_record',
  'duty_return',
  'recipe',
  'packaging_run',
  'distribution_movement',
  'ingredient_lot',
]

function eventBadgeClass(eventType: string): string {
  if (eventType.endsWith('.deleted')) return 'bg-red-100 text-red-800'
  if (eventType.endsWith('.approved') || eventType.endsWith('.submitted')) return 'bg-green-100 text-green-800'
  if (eventType.endsWith('.created')) return 'bg-blue-100 text-blue-800'
  return 'bg-gray-100 text-gray-700'
}

function truncateUUID(id: string | null | undefined): string {
  if (!id) return '—'
  return id.slice(0, 8) + '…'
}

function EventRow({ event }: { event: AuditEvent }) {
  const [expanded, setExpanded] = useState(false)

  return (
    <>
      <tr
        className="hover:bg-amber-50 cursor-pointer border-b border-stone-100"
        onClick={() => setExpanded((e) => !e)}
      >
        <td className="px-4 py-2 text-xs text-stone-500 whitespace-nowrap">
          {event.created_at ? new Date(event.created_at).toLocaleString() : '—'}
        </td>
        <td className="px-4 py-2">
          <span className={`inline-block px-2 py-0.5 rounded text-xs font-medium ${eventBadgeClass(event.event_type ?? '')}`}>
            {event.event_type}
          </span>
        </td>
        <td className="px-4 py-2 text-sm text-stone-700">{event.entity_type}</td>
        <td className="px-4 py-2 text-xs text-stone-500 font-mono">{truncateUUID(event.entity_id)}</td>
        <td className="px-4 py-2 text-xs text-stone-400">
          {expanded ? '▲ hide' : '▼ show'}
        </td>
      </tr>
      {expanded && (
        <tr className="bg-stone-50">
          <td colSpan={5} className="px-6 py-3">
            <pre className="text-xs text-stone-700 whitespace-pre-wrap break-all max-h-64 overflow-y-auto">
              {JSON.stringify(event.event_data, null, 2)}
            </pre>
          </td>
        </tr>
      )}
    </>
  )
}

export default function ComplianceAuditPage() {
  const [filters, setFilters] = useState<AuditParams>({ page: 1, page_size: 50 })
  const [fromInput, setFromInput] = useState('')
  const [toInput, setToInput] = useState('')

  const { data, isLoading, error } = useAuditEvents(filters)

  function applyFilters() {
    setFilters((f) => ({
      ...f,
      from: fromInput ? new Date(fromInput).toISOString() : undefined,
      to: toInput ? new Date(toInput).toISOString() : undefined,
      page: 1,
    }))
  }

  return (
    <div className="p-6 max-w-6xl mx-auto">
      <h1 className="text-2xl font-bold text-stone-800 mb-6">Compliance Audit Log</h1>

      {/* Filter bar */}
      <div className="bg-white border border-stone-200 rounded-lg p-4 mb-6 flex flex-wrap gap-3 items-end">
        <div>
          <label className="block text-xs text-stone-500 mb-1">Event Type</label>
          <select
            className="border border-stone-300 rounded px-2 py-1.5 text-sm"
            value={filters.event_type ?? ''}
            onChange={(e) => setFilters((f) => ({ ...f, event_type: e.target.value || undefined, page: 1 }))}
          >
            <option value="">All events</option>
            {EVENT_TYPES.map((et) => (
              <option key={et} value={et}>{et}</option>
            ))}
          </select>
        </div>

        <div>
          <label className="block text-xs text-stone-500 mb-1">Entity Type</label>
          <select
            className="border border-stone-300 rounded px-2 py-1.5 text-sm"
            value={filters.entity_type ?? ''}
            onChange={(e) => setFilters((f) => ({ ...f, entity_type: e.target.value || undefined, page: 1 }))}
          >
            <option value="">All entities</option>
            {ENTITY_TYPES.map((et) => (
              <option key={et} value={et}>{et}</option>
            ))}
          </select>
        </div>

        <div>
          <label className="block text-xs text-stone-500 mb-1">From</label>
          <input
            type="datetime-local"
            className="border border-stone-300 rounded px-2 py-1.5 text-sm"
            value={fromInput}
            onChange={(e) => setFromInput(e.target.value)}
          />
        </div>

        <div>
          <label className="block text-xs text-stone-500 mb-1">To</label>
          <input
            type="datetime-local"
            className="border border-stone-300 rounded px-2 py-1.5 text-sm"
            value={toInput}
            onChange={(e) => setToInput(e.target.value)}
          />
        </div>

        <button
          className="px-4 py-1.5 bg-amber-600 text-white rounded text-sm hover:bg-amber-700"
          onClick={applyFilters}
        >
          Apply
        </button>

        <button
          className="px-4 py-1.5 border border-stone-300 text-stone-600 rounded text-sm hover:bg-stone-50"
          onClick={() => {
            setFilters({ page: 1, page_size: 50 })
            setFromInput('')
            setToInput('')
          }}
        >
          Clear
        </button>
      </div>

      {/* Table */}
      {isLoading && <p className="text-stone-500">Loading…</p>}
      {error && <p className="text-red-600">Failed to load audit log.</p>}

      {data && (
        <>
          <div className="bg-white border border-stone-200 rounded-lg overflow-hidden">
            <table className="w-full text-sm">
              <thead className="bg-stone-50 border-b border-stone-200">
                <tr>
                  <th className="px-4 py-2 text-left text-xs font-medium text-stone-500 uppercase">Time</th>
                  <th className="px-4 py-2 text-left text-xs font-medium text-stone-500 uppercase">Event</th>
                  <th className="px-4 py-2 text-left text-xs font-medium text-stone-500 uppercase">Entity</th>
                  <th className="px-4 py-2 text-left text-xs font-medium text-stone-500 uppercase">ID</th>
                  <th className="px-4 py-2 text-left text-xs font-medium text-stone-500 uppercase">Data</th>
                </tr>
              </thead>
              <tbody>
                {data.items?.length === 0 && (
                  <tr>
                    <td colSpan={5} className="px-4 py-8 text-center text-stone-400">
                      No audit events found.
                    </td>
                  </tr>
                )}
                {data.items?.map((event) => (
                  <EventRow key={event.id} event={event} />
                ))}
              </tbody>
            </table>
          </div>

          {/* Pagination */}
          {(data.total_pages ?? 0) > 1 && (
            <div className="flex items-center gap-2 mt-4">
              <button
                className="px-3 py-1 border border-stone-300 rounded text-sm disabled:opacity-40"
                disabled={(filters.page ?? 1) <= 1}
                onClick={() => setFilters((f) => ({ ...f, page: (f.page ?? 1) - 1 }))}
              >
                Previous
              </button>
              <span className="text-sm text-stone-600">
                Page {data.page} of {data.total_pages}
              </span>
              <button
                className="px-3 py-1 border border-stone-300 rounded text-sm disabled:opacity-40"
                disabled={(filters.page ?? 1) >= (data.total_pages ?? 1)}
                onClick={() => setFilters((f) => ({ ...f, page: (f.page ?? 1) + 1 }))}
              >
                Next
              </button>
              <span className="text-sm text-stone-400 ml-2">{data.total} total events</span>
            </div>
          )}
        </>
      )}
    </div>
  )
}
