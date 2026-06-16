import React, { useState } from 'react';
import { Link } from 'react-router-dom';
import { APIError } from '../../api/error';
import { useBatchCostsList, useComputeBatchCost } from './hooks/useBatchCosts';

const formatPence = (p: number | null | undefined): string => p == null ? '-' : '£' + (p / 100).toFixed(2);

export const BatchCostsPage: React.FC = () => {
  const [page, setPage] = useState(1);
  const [showComputeForm, setShowComputeForm] = useState(false);
  const [computeForm, setComputeForm] = useState({
    batch_id: '',
    energy_kwh: '',
    labor_hours: '',
    water_liters: '',
    overhead_pence: '',
  });

  const { data, isLoading, isError, error, refetch } = useBatchCostsList({ page, page_size: 20 });
  const computeMutation = useComputeBatchCost();

  const handleCompute = () => {
    computeMutation.mutate(
      {
        batch_id: computeForm.batch_id,
        energy_kwh: computeForm.energy_kwh ? Number(computeForm.energy_kwh) : null,
        labor_hours: computeForm.labor_hours ? Number(computeForm.labor_hours) : null,
        water_liters: computeForm.water_liters ? Number(computeForm.water_liters) : null,
        overhead_pence: computeForm.overhead_pence ? Number(computeForm.overhead_pence) : null,
      },
      {
        onSuccess: () => {
          setComputeForm({
            batch_id: '',
            energy_kwh: '',
            labor_hours: '',
            water_liters: '',
            overhead_pence: '',
          });
          setShowComputeForm(false);
          refetch();
        },
      }
    );
  };

  const handleFormChange = (field: string, value: string) => {
    setComputeForm((prev) => ({ ...prev, [field]: value }));
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
        <h1 className="text-2xl font-bold text-[var(--color-fg)]">Batch Costs</h1>
        <button onClick={() => setShowComputeForm((v) => !v)} className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90">
          Compute cost
        </button>
      </div>

      {showComputeForm && (
        <div className="p-4 border border-[var(--color-border)] rounded bg-[var(--color-surface)] space-y-4">
          <h2 className="text-lg font-semibold text-[var(--color-fg)]">Compute Batch Cost</h2>
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm text-[var(--color-muted)]">Batch ID *</label>
              <input
                type="text"
                value={computeForm.batch_id}
                onChange={(e) => handleFormChange('batch_id', e.target.value)}
                required
                className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm"
              />
            </div>
            <div>
              <label className="block text-sm text-[var(--color-muted)]">Energy (kWh)</label>
              <input
                type="number"
                value={computeForm.energy_kwh}
                onChange={(e) => handleFormChange('energy_kwh', e.target.value)}
                className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm"
              />
            </div>
            <div>
              <label className="block text-sm text-[var(--color-muted)]">Labor (hours)</label>
              <input
                type="number"
                value={computeForm.labor_hours}
                onChange={(e) => handleFormChange('labor_hours', e.target.value)}
                className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm"
              />
            </div>
            <div>
              <label className="block text-sm text-[var(--color-muted)]">Water (liters)</label>
              <input
                type="number"
                value={computeForm.water_liters}
                onChange={(e) => handleFormChange('water_liters', e.target.value)}
                className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm"
              />
            </div>
            <div className="col-span-2">
              <label className="block text-sm text-[var(--color-muted)]">Overhead (pence)</label>
              <input
                type="number"
                value={computeForm.overhead_pence}
                onChange={(e) => handleFormChange('overhead_pence', e.target.value)}
                className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm"
              />
            </div>
          </div>
          <button onClick={handleCompute} className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90">
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
        <p className="text-[var(--color-muted)]">No batch costs found</p>
      )}

      {data && (data.items ?? []).length > 0 && (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b">
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Batch</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Ingredients</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Energy</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Labor</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Water</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Overhead</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Est. Duty</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Total</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Per Liter</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Per Unit</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Computed At</th>
              </tr>
            </thead>
            <tbody>
              {(data.items ?? []).map((item: any) => (
                <tr key={item.id} className="border-b">
                  <td className="py-2">
                    <Link to={`/batches/${item.batch_id}`} className="text-[var(--color-accent)] hover:underline">
                      {item.batch_id?.substring(0, 8)}
                    </Link>
                  </td>
                  <td className="py-2">{formatPence(item.ingredients_pence)}</td>
                  <td className="py-2">{formatPence(item.energy_pence)}</td>
                  <td className="py-2">{formatPence(item.labor_pence)}</td>
                  <td className="py-2">{formatPence(item.water_pence)}</td>
                  <td className="py-2">{formatPence(item.overhead_pence)}</td>
                  <td className="py-2">{formatPence(item.duty_pence)}</td>
                  <td className="py-2">{formatPence(item.total_pence)}</td>
                  <td className="py-2">{formatPence(item.per_liter_pence)}</td>
                  <td className="py-2">{formatPence(item.per_unit_pence)}</td>
                  <td className="py-2">{new Date(item.computed_at).toLocaleString()}</td>
                </tr>
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
