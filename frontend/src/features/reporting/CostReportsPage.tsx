import React, { useState } from 'react';
import { APIError } from '../../api/error';
import { useCostReportsList, useGenerateCostReport, useDeleteCostReport, REPORT_TYPES } from './hooks/useCostReports';

const formatPence = (p: number | null | undefined): string => p == null ? '-' : '£' + (p / 100).toFixed(2);

const CostReportRow: React.FC<{
  item: any;
  viewingId: string | null;
  onToggleView: (id: string | null) => void;
  onRefetch: () => void;
}> = ({ item, viewingId, onToggleView, onRefetch }) => {
  const [isDeleting, setIsDeleting] = useState(false);
  const deleteMutation = useDeleteCostReport();

  const isViewing = viewingId === item.id;

  const reportData = item.report_data || {};

  const handleViewToggle = () => {
    onToggleView(isViewing ? null : item.id);
  };

  const handleDelete = () => {
    setIsDeleting(true);
    deleteMutation.mutate(item.id!, {
      onSuccess: () => {
        setIsDeleting(false);
        onRefetch();
      },
      onError: () => {
        setIsDeleting(false);
      },
    });
  };

  const renderViewer = () => {
    if (item.report_type === 'period') {
      const fields = {
        ingredient: reportData.ingredients_pence,
        energy: reportData.energy_pence,
        labor: reportData.labor_pence,
        water: reportData.water_pence,
        overhead: reportData.overhead_pence,
        duty: reportData.duty_pence,
      };
      const colors: Record<string, string> = {
        ingredient: 'bg-green-500',
        energy: 'bg-yellow-500',
        labor: 'bg-blue-500',
        water: 'bg-cyan-500',
        overhead: 'bg-purple-500',
        duty: 'bg-orange-500',
      };
      const labels: Record<string, string> = {
        ingredient: 'Ingredients',
        energy: 'Energy',
        labor: 'Labor',
        water: 'Water',
        overhead: 'Overhead',
        duty: 'Duty',
      };

      const validFields = Object.entries(fields).filter(([_, v]) => v != null);
      const total = validFields.reduce((sum, [_, v]) => sum + (v || 0), 0);

      if (total === 0) {
        return <pre className="text-xs bg-[var(--color-surface)] p-4 rounded border overflow-auto">No cost data available</pre>;
      }

      return (
        <div className="p-4 bg-[var(--color-surface)] rounded border space-y-4">
          <div className="flex h-8 w-full rounded overflow-hidden">
            {validFields.map(([key, value]) => {
              const width = (value! / total) * 100;
              const showLabel = width > 8;
              return (
                <div
                  key={key}
                  style={{ width: `${width}%` }}
                  className={`${colors[key]} h-full flex items-center justify-center text-white text-xs font-medium`}
                  title={`${labels[key]}: ${formatPence(value)} (${width.toFixed(1)}%)`}
                >
                  {showLabel && `${labels[key]} ${width.toFixed(0)}%`}
                </div>
              );
            })}
          </div>
          <div className="flex flex-wrap gap-4 text-xs">
            {validFields.map(([key, value]) => (
              <div key={key} className="flex items-center gap-2">
                <span className={`w-3 h-3 rounded-full ${colors[key]}`} />
                <span className="text-[var(--color-muted)]">{labels[key]}: {formatPence(value)} ({((value! / total) * 100).toFixed(1)}%)</span>
              </div>
            ))}
          </div>
        </div>
      );
    }

    return (
      <pre className="text-xs bg-[var(--color-surface)] p-4 rounded border overflow-auto">
        {JSON.stringify(item.report_data, null, 2)}
      </pre>
    );
  };

  return (
    <>
      <tr className="border-b">
        <td className="py-2">{item.report_type}</td>
        <td className="py-2">{item.period_start || '-'}</td>
        <td className="py-2">{item.period_end || '-'}</td>
        <td className="py-2">{new Date(item.generated_at).toLocaleString()}</td>
        <td className="py-2 space-x-2">
          <button onClick={handleViewToggle} className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90">
            {isViewing ? 'Hide' : 'View'}
          </button>
          <button onClick={handleDelete} disabled={isDeleting} className="px-4 py-2 rounded text-sm bg-[var(--color-danger)] text-white hover:opacity-90 disabled:opacity-50">
            {isDeleting ? 'Deleting...' : 'Delete'}
          </button>
        </td>
      </tr>
      {isViewing && (
        <tr>
          <td colSpan={5} className="pb-4">
            {renderViewer()}
          </td>
        </tr>
      )}
    </>
  );
};

export const CostReportsPage: React.FC = () => {
  const [page, setPage] = useState(1);
  const [showGenerateForm, setShowGenerateForm] = useState(false);
  const [viewingId, setViewingId] = useState<string | null>(null);
  const [generateForm, setGenerateForm] = useState({
    report_type: 'period',
    period_start: '',
    period_end: '',
    batch_id: '',
    recipe_id: '',
  });

  const { data, isLoading, isError, error, refetch } = useCostReportsList({ page, page_size: 20 });
  const generateMutation = useGenerateCostReport();

  const handleGenerate = () => {
    generateMutation.mutate(
      {
        report_type: generateForm.report_type as 'batch' | 'recipe' | 'period' | 'inventory',
        period_start: generateForm.period_start || null,
        period_end: generateForm.period_end || null,
        batch_id: generateForm.batch_id || null,
        recipe_id: generateForm.recipe_id || null,
      },
      {
        onSuccess: () => {
          setGenerateForm({
            report_type: 'period',
            period_start: '',
            period_end: '',
            batch_id: '',
            recipe_id: '',
          });
          setShowGenerateForm(false);
          refetch();
        },
      }
    );
  };

  const handleFormChange = (field: string, value: string) => {
    setGenerateForm((prev) => ({ ...prev, [field]: value }));
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
        <h1 className="text-2xl font-bold text-[var(--color-fg)]">Cost Reports</h1>
        <button onClick={() => setShowGenerateForm((v) => !v)} className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90">
          Generate report
        </button>
      </div>

      {showGenerateForm && (
        <div className="p-4 border border-[var(--color-border)] rounded bg-[var(--color-surface)] space-y-4">
          <h2 className="text-lg font-semibold text-[var(--color-fg)]">Generate Cost Report</h2>
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm text-[var(--color-muted)]">Report Type</label>
              <select value={generateForm.report_type} onChange={(e) => handleFormChange('report_type', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm">
                {REPORT_TYPES.map((t) => (
                  <option key={t} value={t}>{t}</option>
                ))}
              </select>
            </div>
            <div />
            {generateForm.report_type === 'period' && (
              <>
                <div>
                  <label className="block text-sm text-[var(--color-muted)]">Period Start</label>
                  <input type="date" value={generateForm.period_start} onChange={(e) => handleFormChange('period_start', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm" />
                </div>
                <div>
                  <label className="block text-sm text-[var(--color-muted)]">Period End</label>
                  <input type="date" value={generateForm.period_end} onChange={(e) => handleFormChange('period_end', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm" />
                </div>
              </>
            )}
            {generateForm.report_type === 'batch' && (
              <div className="col-span-2">
                <label className="block text-sm text-[var(--color-muted)]">Batch ID</label>
                <input type="text" value={generateForm.batch_id} onChange={(e) => handleFormChange('batch_id', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm" />
              </div>
            )}
            {generateForm.report_type === 'recipe' && (
              <div className="col-span-2">
                <label className="block text-sm text-[var(--color-muted)]">Recipe ID</label>
                <input type="text" value={generateForm.recipe_id} onChange={(e) => handleFormChange('recipe_id', e.target.value)} className="w-full p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)] text-sm" />
              </div>
            )}
          </div>
          <button onClick={handleGenerate} className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90">
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
        <p className="text-[var(--color-muted)]">No cost reports found</p>
      )}

      {data && (data.items ?? []).length > 0 && (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b">
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Type</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Period Start</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Period End</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Generated At</th>
                <th className="text-left py-2 font-semibold text-[var(--color-muted)]">Actions</th>
              </tr>
            </thead>
            <tbody>
              {(data.items ?? []).map((item: any) => (
                <CostReportRow
                  key={item.id}
                  item={item}
                  viewingId={viewingId}
                  onToggleView={setViewingId}
                  onRefetch={refetch}
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
