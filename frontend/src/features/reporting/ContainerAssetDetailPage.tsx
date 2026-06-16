import React, { useState } from 'react'
import { useParams, Link } from 'react-router-dom'
import {
  useContainerAsset,
  usePatchContainerAsset,
  useFillContainer,
  useDeliverContainer,
  useReturnContainer,
  useContainerLogs,
  CONTAINER_TYPES,
} from './hooks/useContainerAssets'
import { APIError } from '../../api/error'

type ContainerType = typeof CONTAINER_TYPES[number]

export function ContainerAssetDetailPage() {
  const { id } = useParams<{ id: string }>()
  const [activeAction, setActiveAction] = useState<'fill' | 'deliver' | 'return' | null>(null)
  const [isEditing, setIsEditing] = useState(false)
  const [logsPage, setLogsPage] = useState(1)

  const { data: asset, isLoading, isError, error, refetch } = useContainerAsset(id!)
  const patchMutation = usePatchContainerAsset(id!)
  const fillMutation = useFillContainer(id!)
  const deliverMutation = useDeliverContainer(id!)
  const returnMutation = useReturnContainer(id!)
  const { data: logsData, isLoading: isLoadingLogs } = useContainerLogs(id!, { page: logsPage, page_size: 10 })

  const [editForm, setEditForm] = useState({
    asset_number: '',
    container_type: '' as ContainerType,
    capacity_liters: 0,
    deposit_pence: 0,
    notes: '',
  })
  const [fillForm, setFillForm] = useState({ batch_id: '', notes: '' })
  const [deliverForm, setDeliverForm] = useState({ customer_name: '', notes: '' })
  const [returnForm, setReturnForm] = useState({ notes: '' })

  const formatDeposit = (pence: number | null | undefined) => {
    if (!pence) return '-'
    return '£' + (pence / 100).toFixed(2)
  }

  const handleEdit = () => {
    if (!asset) return
    setEditForm({
      asset_number: asset.asset_number || '',
      container_type: (asset.container_type ?? 'keg') as ContainerType,
      capacity_liters: asset.capacity_liters || 0,
      deposit_pence: asset.deposit_pence || 0,
      notes: asset.notes || '',
    })
    setIsEditing(true)
  }

  const handleEditSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    await patchMutation.mutateAsync({
      asset_number: editForm.asset_number,
      container_type: editForm.container_type,
      capacity_liters: editForm.capacity_liters,
      deposit_pence: editForm.deposit_pence,
      notes: editForm.notes,
    })
    setIsEditing(false)
  }

  const handleFillSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    await fillMutation.mutateAsync({
      batch_id: fillForm.batch_id || undefined,
      notes: fillForm.notes || undefined,
    })
    setActiveAction(null)
    setFillForm({ batch_id: '', notes: '' })
  }

  const handleDeliverSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    await deliverMutation.mutateAsync({
      customer_name: deliverForm.customer_name,
      notes: deliverForm.notes || undefined,
    })
    setActiveAction(null)
    setDeliverForm({ customer_name: '', notes: '' })
  }

  const handleReturnSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    await returnMutation.mutateAsync({
      notes: returnForm.notes || undefined,
    })
    setActiveAction(null)
    setReturnForm({ notes: '' })
  }

  if (isLoading) {
    return (
      <div className="p-6 space-y-2 animate-pulse">
        <div className="h-8 rounded bg-[var(--color-border)/20] w-64" />
        {[...Array(3)].map((_, i) => (
          <div key={i} className="h-12 rounded bg-[var(--color-border)/20]" />
        ))}
      </div>
    )
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

  if (!asset) {
    return <div className="p-6 text-[var(--color-muted)]">Asset not found</div>
  }

  const logsTotalPages = logsData?.total_pages || 1

  return (
    <div className="p-6 space-y-6">
      <div className="flex justify-between items-center">
        <h1 className="text-2xl font-bold text-[var(--color-fg)]">{asset.asset_number}</h1>
        <Link
          to={'/container-assets/' + id + '/qr'}
          className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90"
        >
          View QR
        </Link>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <div className="p-4 border border-[var(--color-border)] rounded bg-[var(--color-surface)]">
          <p className="text-[var(--color-muted)] text-sm">Asset Number</p>
          <p className="text-[var(--color-fg)]">{asset.asset_number}</p>
        </div>
        <div className="p-4 border border-[var(--color-border)] rounded bg-[var(--color-surface)]">
          <p className="text-[var(--color-muted)] text-sm">Type</p>
          <p className="text-[var(--color-fg)]">{asset.container_type}</p>
        </div>
        <div className="p-4 border border-[var(--color-border)] rounded bg-[var(--color-surface)]">
          <p className="text-[var(--color-muted)] text-sm">Capacity (L)</p>
          <p className="text-[var(--color-fg)]">{asset.capacity_liters}</p>
        </div>
        <div className="p-4 border border-[var(--color-border)] rounded bg-[var(--color-surface)]">
          <p className="text-[var(--color-muted)] text-sm">Status</p>
          <p className="text-[var(--color-fg)]">{asset.status}</p>
        </div>
        <div className="p-4 border border-[var(--color-border)] rounded bg-[var(--color-surface)]">
          <p className="text-[var(--color-muted)] text-sm">Deposit</p>
          <p className="text-[var(--color-fg)]">{formatDeposit(asset.deposit_pence)}</p>
        </div>
        <div className="p-4 border border-[var(--color-border)] rounded bg-[var(--color-surface)]">
          <p className="text-[var(--color-muted)] text-sm">Customer</p>
          <p className="text-[var(--color-fg)]">{asset.current_customer_name || '-'}</p>
        </div>
        <div className="p-4 border border-[var(--color-border)] rounded bg-[var(--color-surface)]">
          <p className="text-[var(--color-muted)] text-sm">Last Fill Date</p>
          <p className="text-[var(--color-fg)]">{asset.last_fill_date || '-'}</p>
        </div>
        <div className="p-4 border border-[var(--color-border)] rounded bg-[var(--color-surface)]">
          <p className="text-[var(--color-muted)] text-sm">Last Return Date</p>
          <p className="text-[var(--color-fg)]">{asset.last_return_date || '-'}</p>
        </div>
        <div className="p-4 border border-[var(--color-border)] rounded bg-[var(--color-surface)] md:col-span-2 lg:col-span-3">
          <p className="text-[var(--color-muted)] text-sm">Notes</p>
          <p className="text-[var(--color-fg)]">{asset.notes || '-'}</p>
        </div>
        <div className="p-4 border border-[var(--color-border)] rounded bg-[var(--color-surface)]">
          <p className="text-[var(--color-muted)] text-sm">Created At</p>
          <p className="text-[var(--color-fg)]">{asset.created_at}</p>
        </div>
      </div>

      <div className="pt-4 border-t border-[var(--color-border)]">
        <h2 className="text-xl font-semibold text-[var(--color-fg)] mb-4">Actions</h2>
        <div className="flex gap-2 mb-4">
          <button
            onClick={() => setActiveAction('fill')}
            className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90"
          >
            Fill
          </button>
          <button
            onClick={() => setActiveAction('deliver')}
            className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90"
          >
            Deliver
          </button>
          <button
            onClick={() => setActiveAction('return')}
            className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90"
          >
            Return
          </button>
          <button
            onClick={handleEdit}
            className="px-4 py-2 rounded text-sm bg-[var(--color-border)] text-[var(--color-fg)] hover:bg-[var(--color-border)/50]"
          >
            Edit asset
          </button>
        </div>

        {activeAction === 'fill' && (
          <form onSubmit={handleFillSubmit} className="flex gap-2 items-end mb-4">
            <div>
              <label className="block text-[var(--color-muted)] text-sm">Batch ID (optional)</label>
              <input
                type="text"
                value={fillForm.batch_id}
                onChange={(e) => setFillForm({ ...fillForm, batch_id: e.target.value })}
                className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm"
              />
            </div>
            <div>
              <label className="block text-[var(--color-muted)] text-sm">Notes (optional)</label>
              <input
                type="text"
                value={fillForm.notes}
                onChange={(e) => setFillForm({ ...fillForm, notes: e.target.value })}
                className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm"
              />
            </div>
            <button type="submit" className="px-4 py-2 rounded text-sm bg-[var(--color-success)] text-white hover:opacity-90">
              Submit
            </button>
            <button type="button" onClick={() => setActiveAction(null)} className="px-4 py-2 rounded text-sm bg-[var(--color-border)] text-[var(--color-fg)]">
              Cancel
            </button>
          </form>
        )}

        {activeAction === 'deliver' && (
          <form onSubmit={handleDeliverSubmit} className="flex gap-2 items-end mb-4">
            <div>
              <label className="block text-[var(--color-muted)] text-sm">Customer Name *</label>
              <input
                type="text"
                required
                value={deliverForm.customer_name}
                onChange={(e) => setDeliverForm({ ...deliverForm, customer_name: e.target.value })}
                className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm"
              />
            </div>
            <div>
              <label className="block text-[var(--color-muted)] text-sm">Notes (optional)</label>
              <input
                type="text"
                value={deliverForm.notes}
                onChange={(e) => setDeliverForm({ ...deliverForm, notes: e.target.value })}
                className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm"
              />
            </div>
            <button type="submit" className="px-4 py-2 rounded text-sm bg-[var(--color-success)] text-white hover:opacity-90">
              Submit
            </button>
            <button type="button" onClick={() => setActiveAction(null)} className="px-4 py-2 rounded text-sm bg-[var(--color-border)] text-[var(--color-fg)]">
              Cancel
            </button>
          </form>
        )}

        {activeAction === 'return' && (
          <form onSubmit={handleReturnSubmit} className="flex gap-2 items-end mb-4">
            <div>
              <label className="block text-[var(--color-muted)] text-sm">Notes (optional)</label>
              <input
                type="text"
                value={returnForm.notes}
                onChange={(e) => setReturnForm({ ...returnForm, notes: e.target.value })}
                className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm"
              />
            </div>
            <button type="submit" className="px-4 py-2 rounded text-sm bg-[var(--color-success)] text-white hover:opacity-90">
              Submit
            </button>
            <button type="button" onClick={() => setActiveAction(null)} className="px-4 py-2 rounded text-sm bg-[var(--color-border)] text-[var(--color-fg)]">
              Cancel
            </button>
          </form>
        )}

        {isEditing && (
          <form onSubmit={handleEditSubmit} className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 p-4 border border-[var(--color-border)] rounded bg-[var(--color-surface)] mb-4">
            <div>
              <label className="block text-[var(--color-muted)] text-sm">Asset Number</label>
              <input
                type="text"
                value={editForm.asset_number}
                onChange={(e) => setEditForm({ ...editForm, asset_number: e.target.value })}
                className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm"
              />
            </div>
            <div>
              <label className="block text-[var(--color-muted)] text-sm">Type</label>
              <select
                value={editForm.container_type}
                onChange={(e) => setEditForm({ ...editForm, container_type: e.target.value as ContainerType })}
                className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm"
              >
                {CONTAINER_TYPES.map((t) => (
                  <option key={t} value={t}>{t}</option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-[var(--color-muted)] text-sm">Capacity (L)</label>
              <input
                type="number"
                value={editForm.capacity_liters}
                onChange={(e) => setEditForm({ ...editForm, capacity_liters: Number(e.target.value) })}
                className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm"
              />
            </div>
            <div>
              <label className="block text-[var(--color-muted)] text-sm">Deposit (pence)</label>
              <input
                type="number"
                value={editForm.deposit_pence}
                onChange={(e) => setEditForm({ ...editForm, deposit_pence: Number(e.target.value) })}
                className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm"
              />
            </div>
            <div className="md:col-span-2 lg:col-span-3">
              <label className="block text-[var(--color-muted)] text-sm">Notes</label>
              <textarea
                value={editForm.notes}
                onChange={(e) => setEditForm({ ...editForm, notes: e.target.value })}
                className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm"
              />
            </div>
            <div className="flex gap-2 md:col-span-2 lg:col-span-3">
              <button type="submit" className="px-4 py-2 rounded text-sm bg-[var(--color-success)] text-white hover:opacity-90">
                Save
              </button>
              <button type="button" onClick={() => setIsEditing(false)} className="px-4 py-2 rounded text-sm bg-[var(--color-border)] text-[var(--color-fg)]">
                Cancel
              </button>
            </div>
          </form>
        )}
      </div>

      <div className="pt-4 border-t border-[var(--color-border)]">
        <h2 className="text-xl font-semibold text-[var(--color-fg)] mb-4">Activity Log</h2>
        {isLoadingLogs && (
          <div className="space-y-2 animate-pulse">
            {[...Array(5)].map((_, i) => (
              <div key={i} className="h-12 rounded bg-[var(--color-border)/20]" />
            ))}
          </div>
        )}
        {!isLoadingLogs && logsData && (logsData.items ?? []).length > 0 && (
          <>
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-[var(--color-border)]">
                    <th className="text-left p-2 text-[var(--color-muted)]">Date</th>
                    <th className="text-left p-2 text-[var(--color-muted)]">Event</th>
                    <th className="text-left p-2 text-[var(--color-muted)]">Status change</th>
                    <th className="text-left p-2 text-[var(--color-muted)]">Notes</th>
                  </tr>
                </thead>
                <tbody>
                  {(logsData.items ?? []).map((log) => (
                    <tr key={log.id} className="border-b border-[var(--color-border)] hover:bg-[var(--color-border)/30]">
                      <td className="p-2 text-[var(--color-fg)]">{log.created_at ? new Date(log.created_at).toLocaleString() : '-'}</td>
                      <td className="p-2 text-[var(--color-fg)]">{log.event_type}</td>
                      <td className="p-2 text-[var(--color-fg)]">
                        {log.from_status} → {log.to_status}
                      </td>
                      <td className="p-2 text-[var(--color-fg)]">{log.notes || '-'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="flex justify-between items-center mt-4">
              <span className="text-[var(--color-muted)] text-sm">
                Page {logsPage} of {logsTotalPages}
              </span>
              <div className="flex gap-2">
                <button
                  onClick={() => setLogsPage((p) => Math.max(1, p - 1))}
                  disabled={logsPage <= 1}
                  className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
                >
                  Prev
                </button>
                <button
                  onClick={() => setLogsPage((p) => Math.min(logsTotalPages, p + 1))}
                  disabled={logsPage >= logsTotalPages}
                  className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
                >
                  Next
                </button>
              </div>
            </div>
          </>
        )}
        {!isLoadingLogs && logsData && (logsData.items ?? []).length === 0 && (
          <p className="text-[var(--color-muted)]">No activity logs yet.</p>
        )}
      </div>
    </div>
  )
}
