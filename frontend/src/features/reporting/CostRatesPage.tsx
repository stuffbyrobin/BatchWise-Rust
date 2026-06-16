import React, { useState } from 'react';
import { APIError } from '../../api/error';
import { useCostRatesList, useCreateCostRate, usePatchCostRate, useDeleteCostRate, RATE_TYPES } from './hooks/useCostRates';

const formatPence = (p: number | null | undefined): string => p == null ? '-' : '£' + (p / 100).toFixed(2);

const CostRateRow: React.FC<{ item: any; onEditStart: (id: string) => void; onEditCancel: () => void; editingId: string | null }> = ({ item, onEditStart, onEditCancel, editingId }) => {
  const [editForm, setEditForm] = useState({
    rate_type: '',
    rate_name: '',
    unit: '',
    rate_value: '',
    currency: '',
    effective_from: '',
    effective_to: '',
    notes: '',
  });

  const patchMutation = usePatchCostRate(item.id!);
  const deleteMutation = useDeleteCostRate();

  const isEditing = editingId === item.id;

  const handleEdit = () => {
    setEditForm({
      rate_type: item.rate_type || '',
      rate_name: item.rate_name || '',
      unit: item.unit || '',
      rate_value: item.rate_value ? String(item.rate_value) : '',
      currency: item.currency || '',
      effective_from: item.effective_from || '',
      effective_to: item.effective_to || '',
      notes: item.notes || '',
    });
    onEditStart(item.id);
  };

  const handleSave = () => {
    const payload: Record<string, any> = {};
    Object.entries(editForm).forEach(([key, value]) => {
      if (value !== '') {
        payload[key] = key === 'rate_value' ? Number(value) : value;
      }
    });
    patchMutation.mutate(payload, {
      onSuccess: () => onEditCancel(),
    });
  };

  const handleDelete = () => {
    deleteMutation.mutate(item.id!);
  };

  const handleChange = (field: string, value: string) => {
    setEditForm((prev) => ({ ...prev, [field]: value }));
  };

  if (isEditing) {
    return (
      <tr className="border-b">
        <td className="py-2"><input type="text" value={editForm.rate_name} onChange={(e) => handleChange('rate_name', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm" /></td>
        <td className="py-2">
          <select value={editForm.rate_type} onChange={(e) => handleChange('rate_type', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm">
            {RATE_TYPES.map((t) => (
              <option key={t} value={t}>{t}</option>
            ))}
          </select>
        </td>
        <td className="py-2"><input type="text" value={editForm.unit} onChange={(e) => handleChange('unit', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm" /></td>
        <td className="py-2"><input type="number" value={editForm.rate_value} onChange={(e) => handleChange('rate_value', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm" /></td>
        <td className="py-2"><input type="text" value={editForm.currency} onChange={(e) => handleChange('currency', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm" /></td>
        <td className="py-2"><input type="date" value={editForm.effective_from} onChange={(e) => handleChange('effective_from', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm" /></td>
        <td className="py-2"><input type="date" value={editForm.effective_to} onChange={(e) => handleChange('effective_to', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm" /></td>
        <td className="py-2"><input type="text" value={editForm.notes} onChange={(e) => handleChange('notes', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm" /></td>
        <td className="py-2 space-x-2">
          <button onClick={handleSave} className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90">Save</button>
          <button onClick={onEditCancel} className="px-4 py-2 rounded text-sm bg-[var(--color-danger)] text-white hover:opacity-90">Cancel</button>
        </td>
      </tr>
    );
  }

  return (
    <tr className="border-b">
      <td className="py-2">{item.rate_name}</td>
      <td className="py-2">{item.rate_type}</td>
      <td className="py-2">{item.unit}</td>
      <td className="py-2">{formatPence(item.rate_value)}</td>
      <td className="py-2">{item.currency}</td>
      <td className="py-2">{item.effective_from}</td>
      <td className="py-2">{item.effective_to}</td>
      <td className="py-2">{item.notes?.substring(0, 30) || '-'}</td>
      <td className="py-2 space-x-2">
        <button onClick={handleEdit} className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90">Edit</button>
        <button onClick={handleDelete} className="px-4 py-2 rounded text-sm bg-[var(--color-danger)] text-white hover:opacity-90">Delete</button>
      </td>
    </tr>
  );
};

export const CostRatesPage: React.FC = () => {
  const [page, setPage] = useState(1);
  const [filterType, setFilterType] = useState('');
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);

  const [editForm, setEditForm] = useState({
    rate_type: '',
    rate_name: '',
    unit: '',
    rate_value: '',
    currency: '',
    effective_from: '',
    effective_to: '',
    notes: '',
  });

  const { data, isLoading, isError, error, refetch } = useCostRatesList({ page, page_size: 20, rate_type: filterType || undefined });
  const createMutation = useCreateCostRate();

  const handleCreate = () => {
    createMutation.mutate(
      {
        rate_type: editForm.rate_type as 'energy' | 'labor' | 'water' | 'duty' | 'overhead',
        rate_name: editForm.rate_name,
        unit: editForm.unit,
        rate_value: Number(editForm.rate_value),
        currency: editForm.currency,
        effective_from: editForm.effective_from,
        effective_to: editForm.effective_to,
        notes: editForm.notes,
      },
      {
        onSuccess: () => {
          setEditForm({
            rate_type: '',
            rate_name: '',
            unit: '',
            rate_value: '',
            currency: '',
            effective_from: '',
            effective_to: '',
            notes: '',
          });
          setShowCreateForm(false);
          refetch();
        },
      }
    );
  };

  const handleCreateChange = (field: string, value: string) => {
    setEditForm((prev) => ({ ...prev, [field]: value }));
  };

  if (isError) {
    return (
      <div className="p-4 border border-red-500 rounded bg-[var(--color-surface)]">
        <p className="text-[var(--color-danger)]">{error instanceof APIError ? error.message : 'Unknown error'}</p>
        <button onClick={() => refetch()} className="mt-2 px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90">
          Retry
        </button>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex justify-between items-center">
        <h1 className="text-2xl font-bold text-[var(--color-fg)]">Cost Rates</h1>
        <button onClick={() => setShowCreateForm((v) => !v)} className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90">
          New rate
        </button>
      </div>

      <div className="flex items-center space-x-4">
        <label className="text-[var(--color-muted)]">Filter by type:</label>
        <select value={filterType} onChange={(e) => setFilterType(e.target.value)} className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm">
          <option value="">All types</option>
          {RATE_TYPES.map((t) => (
            <option key={t} value={t}>{t}</option>
          ))}
        </select>
      </div>

      {showCreateForm && (
        <div className="p-4 border border-[var(--color-border)] rounded bg-[var(--color-surface)] space-y-4">
          <h2 className="text-lg font-semibold text-[var(--color-fg)]">Create Cost Rate</h2>
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm text-[var(--color-muted)]">Name</label>
              <input type="text" value={editForm.rate_name} onChange={(e) => handleCreateChange('rate_name', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm" />
            </div>
            <div>
              <label className="block text-sm text-[var(--color-muted)]">Type</label>
              <select value={editForm.rate_type} onChange={(e) => handleCreateChange('rate_type', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm">
                <option value="">Select type</option>
                {RATE_TYPES.map((t) => (
                  <option key={t} value={t}>{t}</option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-sm text-[var(--color-muted)]">Unit</label>
              <input type="text" value={editForm.unit} onChange={(e) => handleCreateChange('unit', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm" />
            </div>
            <div>
              <label className="block text-sm text-[var(--color-muted)]">Rate (pence)</label>
              <input type="number" value={editForm.rate_value} onChange={(e) => handleCreateChange('rate_value', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm" />
            </div>
            <div>
              <label className="block text-sm text-[var(--color-muted)]">Currency</label>
              <input type="text" value={editForm.currency} onChange={(e) => handleCreateChange('currency', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm" />
            </div>
            <div>
              <label className="block text-sm text-[var(--color-muted)]">Effective From</label>
              <input type="date" value={editForm.effective_from} onChange={(e) => handleCreateChange('effective_from', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm" />
            </div>
            <div>
              <label className="block text-sm text-[var(--color-muted)]">Effective To</label>
              <input type="date" value={editForm.effective_to} onChange={(e) => handleCreateChange('effective_to', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm" />
            </div>
            <div className="col-span-2">
              <label className="block text-sm text-[var(--color-muted)]">Notes</label>
              <input type="text" value={editForm.notes} onChange={(e) => handleCreateChange('notes', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm" />
            </div>
          </div>
          <button onClick={handleCreate} className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90">
            Submit
          </button>
        </div>
      )}

      {isLoading && (
        <div className="space-y-2">
          {Array.from({ length: 5 }).map((_, i) => (
            <div key={i} className="h-12 rounded bg-[var(--color-surface)] animate-pulse" />
          ))}
        </div>
      )}

      {data && (data.items ?? []).length === 0 && !isLoading && (
        <p className="text-[var(--color-muted)]">No cost rates found</p>
      )}

      {data && (data.items ?? []).length > 0 && (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b">
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Name</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Type</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Unit</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Rate</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Currency</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">From</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">To</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Notes</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Actions</th>
              </tr>
            </thead>
            <tbody>
              {(data.items ?? []).map((item: any) => (
                <CostRateRow
                  key={item.id}
                  item={item}
                  onEditStart={setEditingId}
                  onEditCancel={() => setEditingId(null)}
                  editingId={editingId}
                />
              ))}
            </tbody>
          </table>
        </div>
      )}

      {data && (
        <div className="flex justify-center space-x-4">
          <button onClick={() => setPage((p) => Math.max(1, p - 1))} disabled={page === 1} className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50">
            Previous
          </button>
          <span className="py-2 text-[var(--color-muted)]">Page {page}</span>
          <button onClick={() => setPage((p) => p + 1)} disabled={page >= (data.total_pages ?? 1)} className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50">
            Next
          </button>
        </div>
      )}
    </div>
  );
};
