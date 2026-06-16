import { Link } from 'react-router-dom'
import { useLabelDesigns, useDeleteLabelDesign } from './hooks/useLabelDesign'

const KIND_LABEL: Record<string, string> = {
  bottle: 'Bottle label',
  can: 'Can label',
  pump_clip: 'Pump clip',
  cask_lens: 'Cask lens',
}

export function LabelDesignsPage() {
  const { data, isLoading, error } = useLabelDesigns({ page_size: 100 })
  const del = useDeleteLabelDesign()

  return (
    <div className="p-6 max-w-5xl">
      <div className="flex items-center justify-between mb-4">
        <h1 className="text-xl font-bold">Label &amp; Print Design</h1>
        <div className="flex gap-2">
          <Link
            to="/label-design/brands"
            className="px-3 py-1.5 rounded text-sm border"
            style={{ borderColor: 'var(--color-border)' }}
          >
            Brand Profiles
          </Link>
          <Link
            to="/label-design/new"
            className="px-3 py-1.5 rounded text-sm text-white"
            style={{ background: 'var(--color-accent)' }}
          >
            New Design
          </Link>
        </div>
      </div>

      {isLoading && <p className="text-sm text-[var(--color-muted)]">Loading…</p>}
      {error && <p className="text-sm text-red-600">{error.message}</p>}

      {data && data.items && data.items.length === 0 && (
        <p className="text-sm text-[var(--color-muted)]">
          No designs yet. Create one to generate print-ready labels, pump clips, or cask lens labels.
        </p>
      )}

      {data && data.items && data.items.length > 0 && (
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="text-left border-b" style={{ borderColor: 'var(--color-border)' }}>
              <th className="py-2 pr-4">Name</th>
              <th className="py-2 pr-4">Kind</th>
              <th className="py-2 pr-4">Size</th>
              <th className="py-2 pr-4" />
            </tr>
          </thead>
          <tbody>
            {data.items.map((d) => (
              <tr key={d.id} className="border-b" style={{ borderColor: 'var(--color-border)' }}>
                <td className="py-2 pr-4">
                  <Link to={`/label-design/${d.id}`} className="text-[var(--color-accent)]">
                    {d.name}
                  </Link>
                </td>
                <td className="py-2 pr-4">{KIND_LABEL[d.kind ?? ''] ?? d.kind}</td>
                <td className="py-2 pr-4">{d.size_key}</td>
                <td className="py-2 pr-4 text-right">
                  <button
                    onClick={() => {
                      if (d.id && confirm('Delete this design?')) del.mutate(d.id)
                    }}
                    className="text-red-600 text-xs"
                  >
                    Delete
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  )
}

export default LabelDesignsPage
