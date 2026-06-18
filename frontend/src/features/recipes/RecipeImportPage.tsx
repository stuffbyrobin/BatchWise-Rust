import { useState, useMemo } from 'react'
import { useNavigate } from 'react-router-dom'
import { useImportRecipe } from './hooks/useRecipes'
import { APIError } from '../../api/error'

type ImportFormat = 'beerxml' | 'brewfather'

const FORMATS: { value: ImportFormat; label: string; accept: string; hint: string }[] = [
  {
    value: 'beerxml',
    label: 'BeerXML',
    accept: '.xml,application/xml,text/xml',
    hint: 'Exports from BeerSmith, BrewMate, and other BeerXML-compatible apps.',
  },
  {
    value: 'brewfather',
    label: 'Brewfather JSON',
    accept: '.json,application/json',
    hint: 'Single recipe export or the full "Export All" JSON from Brewfather.',
  },
]

type RecipeItem = { json: string; name: string; type: string; batchSize: number }
type ItemStatus = 'pending' | 'importing' | 'done' | 'error'

function extractRecipes(raw: Record<string, unknown>): RecipeItem[] | null {
  // Export All format
  if (raw._type === 'Brewfather_Export_User_1') {
    const recipes = (raw.data as Record<string, unknown>)?.recipes
    if (!Array.isArray(recipes)) return null
    return recipes.map((r: Record<string, unknown>) => ({
      json: JSON.stringify(r),
      name: String(r.name ?? ''),
      type: String(r.type ?? ''),
      batchSize: Number(r.batchSize ?? 0),
    }))
  }
  return null
}

export default function RecipeImportPage() {
  const navigate = useNavigate()
  const [format, setFormat] = useState<ImportFormat>('beerxml')
  const [errorMsg, setErrorMsg] = useState<string | null>(null)
  const [search, setSearch] = useState('')

  // Single-file mode
  const [singleData, setSingleData] = useState<string | null>(null)

  // Export All multi-recipe mode
  const [recipeList, setRecipeList] = useState<RecipeItem[] | null>(null)
  const [selected, setSelected] = useState<Set<number>>(new Set())
  const [statuses, setStatuses] = useState<Record<number, ItemStatus>>({})
  const [statusErrors, setStatusErrors] = useState<Record<number, string>>({})
  const [importing, setImporting] = useState(false)

  const importMutation = useImportRecipe()

  const isMulti = recipeList !== null

  function handleFormatChange(f: ImportFormat) {
    setFormat(f)
    reset()
  }

  function reset() {
    setSingleData(null)
    setRecipeList(null)
    setSelected(new Set())
    setStatuses({})
    setStatusErrors({})
    setErrorMsg(null)
    setSearch('')
  }

  function handleFileChange(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0] ?? null
    setErrorMsg(null)
    setSingleData(null)
    setRecipeList(null)

    if (!file) return
    const reader = new FileReader()
    reader.onload = (ev) => {
      const text = ev.target?.result as string
      if (format === 'brewfather') {
        try {
          const parsed = JSON.parse(text)
          const recipes = extractRecipes(parsed)
          if (recipes) {
            setRecipeList(recipes)
            setSelected(new Set(recipes.map((_, i) => i)))
            setStatuses({})
            setStatusErrors({})
            return
          }
        } catch { /* fall through to single import */ }
      }
      setSingleData(text)
    }
    reader.readAsText(file)
  }

  function toggleAll() {
    if (!recipeList) return
    if (selected.size === recipeList.length) setSelected(new Set())
    else setSelected(new Set(recipeList.map((_, i) => i)))
  }

  function toggleOne(i: number) {
    setSelected((prev) => {
      const next = new Set(prev)
      next.has(i) ? next.delete(i) : next.add(i)
      return next
    })
  }

  // Single recipe import
  function handleSingleImport() {
    if (!singleData) return
    const data = format === 'beerxml' ? btoa(unescape(encodeURIComponent(singleData))) : singleData
    importMutation.mutate(
      { format, data },
      {
        onSuccess: () => navigate('/recipes'),
        onError: (err) => setErrorMsg((err as APIError).message || 'Import failed'),
      },
    )
  }

  // Multi-recipe import
  async function handleMultiImport() {
    if (!recipeList) return
    setImporting(true)
    const indices = [...selected].sort((a, b) => a - b)
    const newStatuses: Record<number, ItemStatus> = {}
    const newErrors: Record<number, string> = {}

    for (const i of indices) {
      if (statuses[i] === 'done') continue
      newStatuses[i] = 'importing'
      setStatuses((prev) => ({ ...prev, [i]: 'importing' }))
      try {
        await importMutation.mutateAsync({ format: 'brewfather', data: recipeList[i].json })
        newStatuses[i] = 'done'
        setStatuses((prev) => ({ ...prev, [i]: 'done' }))
      } catch (err) {
        newStatuses[i] = 'error'
        // Validation failures carry the useful detail in `details.reason`
        // (e.g. "brewfather: no yeasts"); `message` is just "Validation failed."
        const reason =
          err instanceof APIError
            ? typeof err.details?.reason === 'string'
              ? err.details.reason
              : err.message
            : err instanceof Error
              ? err.message
              : 'Failed'
        newErrors[i] = reason
        setStatuses((prev) => ({ ...prev, [i]: 'error' }))
        setStatusErrors((prev) => ({ ...prev, [i]: reason }))
      }
    }
    setImporting(false)
  }

  const filteredList = useMemo(() => {
    if (!recipeList) return []
    if (!search) return recipeList.map((r, i) => ({ ...r, i }))
    const q = search.toLowerCase()
    return recipeList
      .map((r, i) => ({ ...r, i }))
      .filter((r) => r.name.toLowerCase().includes(q))
  }, [recipeList, search])

  const doneCount = Object.values(statuses).filter((s) => s === 'done').length
  const errorCount = Object.values(statuses).filter((s) => s === 'error').length
  const finishedCount = doneCount + errorCount
  const selectedCount = selected.size
  const currentFormat = FORMATS.find((f) => f.value === format)!

  return (
    <div className="max-w-3xl">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-bold text-[var(--color-fg)]">Import Recipe</h1>
        <button
          onClick={() => navigate('/recipes')}
          className="px-4 py-1.5 rounded text-sm border border-[var(--color-border)] text-[var(--color-fg)] hover:bg-[var(--color-border)]"
        >
          Cancel
        </button>
      </div>

      {errorMsg && (
        <div className="mb-4 p-3 rounded text-sm" style={{ background: 'var(--color-danger-bg, #fff5f5)', color: 'var(--color-danger)' }}>
          {errorMsg}
        </div>
      )}

      <div className="flex gap-2 mb-6">
        {FORMATS.map((f) => (
          <button
            key={f.value}
            onClick={() => handleFormatChange(f.value)}
            className={`px-4 py-2 rounded text-sm border transition-colors ${
              format === f.value
                ? 'bg-[var(--color-accent)] text-white border-[var(--color-accent)]'
                : 'border-[var(--color-border)] text-[var(--color-fg)] hover:bg-[var(--color-border)]'
            }`}
          >
            {f.label}
          </button>
        ))}
      </div>

      <div className="mb-6 p-4 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-sm text-[var(--color-muted)]">
        {currentFormat.hint}
      </div>

      <div className="mb-6">
        <input
          key={format}
          type="file"
          accept={currentFormat.accept}
          onChange={handleFileChange}
          disabled={importing}
          className="block w-full text-sm text-[var(--color-fg)] file:mr-4 file:py-2 file:px-4 file:rounded file:border-0 file:text-sm file:bg-[var(--color-accent)] file:text-white hover:file:opacity-90"
        />
      </div>

      {/* Single recipe mode */}
      {singleData && !isMulti && (
        <button
          onClick={handleSingleImport}
          disabled={importMutation.isPending}
          className="px-6 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
        >
          {importMutation.isPending ? 'Importing…' : 'Import'}
        </button>
      )}

      {/* Export All multi-recipe mode */}
      {isMulti && (
        <>
          <div className="flex items-center justify-between mb-3 gap-4">
            <div className="flex items-center gap-3 text-sm text-[var(--color-muted)]">
              <span>{recipeList.length} recipes</span>
              {doneCount > 0 && <span className="text-green-600">{doneCount} imported</span>}
              {errorCount > 0 && (
                <span className="text-[var(--color-danger)]">{errorCount} skipped</span>
              )}
            </div>
            <div className="flex items-center gap-2">
              <input
                type="text"
                placeholder="Search…"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="px-3 py-1.5 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-sm text-[var(--color-fg)] w-48"
              />
              <button
                onClick={toggleAll}
                className="px-3 py-1.5 rounded text-sm border border-[var(--color-border)] text-[var(--color-fg)] hover:bg-[var(--color-border)]"
              >
                {selected.size === recipeList.length ? 'Deselect all' : 'Select all'}
              </button>
              <button
                onClick={handleMultiImport}
                disabled={importing || selectedCount === 0}
                className="px-5 py-1.5 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
              >
                {importing ? 'Importing…' : `Import ${selectedCount}`}
              </button>
            </div>
          </div>

          <div className="border rounded-lg overflow-hidden" style={{ borderColor: 'var(--color-border)', background: 'var(--color-surface)' }}>
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left text-xs text-[var(--color-muted)] uppercase tracking-wide border-b" style={{ borderColor: 'var(--color-border)', background: 'var(--color-bg)' }}>
                  <th className="p-2 w-8">
                    <input type="checkbox" checked={selected.size === recipeList.length} onChange={toggleAll} />
                  </th>
                  <th className="p-2">Name</th>
                  <th className="p-2">Type</th>
                  <th className="p-2 text-right">Batch (L)</th>
                  <th className="p-2 w-20"></th>
                </tr>
              </thead>
              <tbody>
                {filteredList.map(({ i, name, type, batchSize }) => {
                  const status = statuses[i]
                  return (
                    <tr
                      key={i}
                      className="border-t"
                      style={{
                        borderColor: 'var(--color-border)',
                        background: status === 'done' ? 'var(--color-success-bg, #f0fff4)'
                          : status === 'error' ? 'var(--color-danger-bg, #fff5f5)'
                          : undefined,
                      }}
                    >
                      <td className="p-2">
                        <input
                          type="checkbox"
                          checked={selected.has(i)}
                          disabled={status === 'done' || importing}
                          onChange={() => toggleOne(i)}
                        />
                      </td>
                      <td className="p-2 font-medium text-[var(--color-fg)]">{name}</td>
                      <td className="p-2 text-[var(--color-muted)]">{type}</td>
                      <td className="p-2 text-right text-[var(--color-muted)]">{batchSize}</td>
                      <td className="p-2 text-center text-xs">
                        {status === 'done' && <span className="text-green-600">✓</span>}
                        {status === 'importing' && <span className="text-[var(--color-muted)]">…</span>}
                        {status === 'error' && (
                          <span className="text-[var(--color-danger)]" title={statusErrors[i]}>✗</span>
                        )}
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>

          {/* Skip-and-warn summary: list the recipes the importer rejected and
              why, so a partial import (e.g. Brewfather recipes missing a yeast)
              is visible rather than a silent per-row ✗. */}
          {errorCount > 0 && !importing && (
            <div
              className="mt-4 p-3 rounded text-sm"
              style={{ background: 'var(--color-warning-bg, #fffbeb)', color: 'var(--color-warning, #92400e)' }}
            >
              <div className="font-medium mb-1">
                {errorCount} recipe{errorCount > 1 ? 's' : ''} skipped
              </div>
              <ul className="list-disc list-inside space-y-0.5">
                {recipeList.map((r, i) =>
                  statuses[i] === 'error' ? (
                    <li key={i}>
                      <span className="font-medium">{r.name || '(unnamed)'}</span>: {statusErrors[i]}
                    </li>
                  ) : null,
                )}
              </ul>
            </div>
          )}

          {finishedCount > 0 && finishedCount >= selectedCount && !importing && (
            <div className="mt-4">
              <button
                onClick={() => navigate('/recipes')}
                className="px-6 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90"
              >
                Done — go to recipes
              </button>
            </div>
          )}
        </>
      )}
    </div>
  )
}
