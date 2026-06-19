import { useState } from 'react'
import { Link } from 'react-router-dom'
import { useRecipesList } from './hooks/useRecipes'
import { APIError } from '../../api/error'
import { formatEbc } from '../../utils/ebc'

type RecipeType = 'all_grain' | 'extract' | 'partial_mash' | 'cider' | 'mead' | 'other'

const TYPE_OPTIONS: { value: RecipeType | ''; label: string }[] = [
  { value: '', label: 'All Types' },
  { value: 'all_grain', label: 'All Grain' },
  { value: 'extract', label: 'Extract' },
  { value: 'partial_mash', label: 'Partial Mash' },
  { value: 'cider', label: 'Cider' },
  { value: 'mead', label: 'Mead' },
  { value: 'other', label: 'Other' },
]

function ebcToSrmClass(ebc: number | null | undefined): string {
  if (ebc === null || ebc === undefined) return 'var(--srm-1)'
  const srm = Math.max(1, Math.min(10, Math.round(ebc / 8)))
  return `var(--srm-${srm})`
}

export default function RecipesListPage() {
  const [nameFilter, setNameFilter] = useState('')
  const [typeFilter, setTypeFilter] = useState<RecipeType | ''>('')
  const [page, setPage] = useState(1)
  const pageSize = 20

  const { data, isLoading, isError, error } = useRecipesList({
    name: nameFilter || undefined,
    type: typeFilter || undefined,
    page,
    page_size: pageSize,
  })

  const recipes = data?.items ?? []
  const totalPages = data?.total_pages ?? 1

  const handleNameChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setNameFilter(e.target.value)
    setPage(1)
  }

  const handleTypeChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    setTypeFilter(e.target.value as RecipeType | '')
    setPage(1)
  }

  if (isError) {
    return (
      <div className="p-4">
        <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded">
          Error loading recipes: {(error as APIError).message}
        </div>
      </div>
    )
  }

  return (
    <div className="p-4">
      <div className="flex justify-between items-center mb-4">
        <h1 className="text-2xl font-bold">Recipes</h1>
        <div className="flex gap-2">
          <Link to="/recipes/new" className="bg-green-600 text-white px-4 py-2 rounded hover:bg-green-700">
            New recipe
          </Link>
          <Link to="/recipes/import" className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700">
            Import
          </Link>
        </div>
      </div>

      <div className="bg-[var(--color-surface)] p-4 rounded shadow mb-4">
        <div className="flex gap-4 items-center">
          <div>
            <label className="block text-sm font-medium text-[var(--color-fg)] mb-1">Name</label>
            <input
              type="text"
              value={nameFilter}
              onChange={handleNameChange}
              placeholder="Filter by name..."
              className="border border-[var(--color-border)] rounded px-3 py-2 w-64 bg-[var(--color-surface)] text-[var(--color-fg)]"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-[var(--color-fg)] mb-1">Type</label>
            <select
              value={typeFilter}
              onChange={handleTypeChange}
              className="border border-[var(--color-border)] rounded px-3 py-2 w-48 bg-[var(--color-surface)] text-[var(--color-fg)]"
            >
              {TYPE_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
          </div>
        </div>
      </div>

      {isLoading ? (
        <div className="text-center py-8">Loading...</div>
      ) : (
        <>
          <div className="bg-[var(--color-surface)] rounded shadow overflow-hidden">
            <table className="w-full">
              <thead className="bg-[var(--color-bg)] border-b border-[var(--color-border)]">
                <tr>
                  <th className="px-4 py-3 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Name</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Type</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Batch Size (L)</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-[var(--color-muted)] uppercase">OG</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-[var(--color-muted)] uppercase">FG</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-[var(--color-muted)] uppercase">ABV %</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-[var(--color-muted)] uppercase">IBU</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-[var(--color-muted)] uppercase">Color</th>
                  <th className="px-4 py-3"></th>
                </tr>
              </thead>
              <tbody>
                {recipes.map((recipe) => (
                  <tr key={recipe.id} className="border-t border-[var(--color-border)] hover:bg-[var(--color-bg)]">
                    <td className="px-4 py-3">
                      <Link
                        to={`/recipes/${recipe.id}`}
                        className="text-blue-600 hover:underline font-medium"
                      >
                        {recipe.name}
                      </Link>
                    </td>
                    <td className="px-4 py-3 text-[var(--color-fg)]">{recipe.type ?? ''}</td>
                    <td className="px-4 py-3 text-[var(--color-fg)]">{recipe.batch_size_liters}</td>
                    <td className="px-4 py-3 text-[var(--color-fg)]">{recipe.calc_og?.toFixed(3) ?? ''}</td>
                    <td className="px-4 py-3 text-[var(--color-fg)]">{recipe.calc_fg?.toFixed(3) ?? ''}</td>
                    <td className="px-4 py-3 text-[var(--color-fg)]">{recipe.calc_abv_pct?.toFixed(1) ?? ''}%</td>
                    <td className="px-4 py-3 text-[var(--color-fg)]">{recipe.calc_ibu?.toFixed(1) ?? ''}</td>
                    <td className="px-4 py-3 text-[var(--color-fg)]">
                      {recipe.calc_color_ebc !== null && recipe.calc_color_ebc !== undefined ? (
                        <div className="flex items-center gap-2">
                          <div
                            className="w-3 h-3 rounded-full"
                            style={{
                              background: ebcToSrmClass(recipe.calc_color_ebc),
                            }}
                          />
                          <span>{formatEbc(recipe.calc_color_ebc)}</span>
                        </div>
                      ) : (
                        ''
                      )}
                    </td>
                    <td className="px-4 py-3 text-right">
                      <Link
                        to={`/recipes/${recipe.id}`}
                        className="text-blue-600 hover:underline text-sm"
                      >
                        View
                      </Link>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {totalPages > 1 && (
            <div className="flex justify-center items-center gap-2 mt-4">
              <button
                onClick={() => setPage((p) => Math.max(1, p - 1))}
                disabled={page <= 1}
                className="px-4 py-2 border rounded disabled:opacity-50"
              >
                Previous
              </button>
              <span className="px-4">
                Page {page} of {totalPages}
              </span>
              <button
                onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
                disabled={page >= totalPages}
                className="px-4 py-2 border rounded disabled:opacity-50"
              >
                Next
              </button>
            </div>
          )}
        </>
      )}
    </div>
  )
}
