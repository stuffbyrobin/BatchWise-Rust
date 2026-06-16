import { useState, useMemo } from 'react'
import { useNavigate } from 'react-router-dom'
import { useImportRecipe } from '../recipes/hooks/useRecipes'
import { useCreateBatch } from './hooks/useBatches'
import { APIError } from '../../api/error'
import type { components } from '../../api/generated'

type ItemStatus = 'pending' | 'importing' | 'done' | 'error'

type BatchItem = {
  recipeJson: string
  name: string
  batchNo: string
  bfStatus: string
  brewDate: string
}

type SingleMode = {
  mode: 'single'
  recipeJson: string
  recipeName: string
  suggestedName: string
  suggestedNumber: string
  suggestedBrewDate: string
}

type MultiMode = {
  mode: 'multi'
  batches: BatchItem[]
}

type ParsedFile = SingleMode | MultiMode

function bfMsToDate(ts: unknown): string {
  if (ts == null || ts === 0 || ts === false) return ''
  const n = Number(ts)
  if (!n || n <= 0) return ''
  const d = new Date(n > 1e10 ? n : n * 1000)
  return d.toISOString().split('T')[0]
}

const BF_STATUS_MAP: Record<string, string> = {
  planning: 'planned',
  planned: 'planned',
  brewing: 'brewing',
  fermenting: 'fermenting',
  conditioning: 'conditioning',
  packaging: 'packaging',
  completed: 'completed',
  archived: 'completed',
  cancelled: 'cancelled',
}

function mapBfStatus(raw: string): string {
  return BF_STATUS_MAP[raw.toLowerCase()] ?? 'planned'
}

function parseBrewfatherFile(text: string): ParsedFile | string {
  let json: Record<string, unknown>
  try { json = JSON.parse(text) } catch { return 'Not valid JSON.' }

  // Export All format
  if (json._type === 'Brewfather_Export_User_1') {
    const data = json.data as Record<string, unknown>
    const rawBatches = data?.batches
    if (!Array.isArray(rawBatches) || rawBatches.length === 0) {
      return 'No batches found in this export.'
    }
    const batches: BatchItem[] = (rawBatches as Record<string, unknown>[])
      .filter((b) => b.recipe && typeof b.recipe === 'object' && !Array.isArray(b.recipe))
      .map((b) => {
        const recipe = b.recipe as Record<string, unknown>
        return {
          recipeJson: JSON.stringify(recipe),
          name: String(recipe.name ?? b.name ?? ''),
          batchNo: String(b.batchNo ?? b.number ?? ''),
          bfStatus: String(b.status ?? ''),
          brewDate: bfMsToDate(b.brewDate),
        }
      })
    if (batches.length === 0) return 'No valid batches (with recipes) found.'
    return { mode: 'multi', batches }
  }

  // Single batch or recipe export
  const isBatch = json.recipe && typeof json.recipe === 'object' && !Array.isArray(json.recipe)
  const recipe = isBatch ? json.recipe as Record<string, unknown> : json
  const recipeName = String(recipe.name ?? '')
  if (!recipeName) return 'No recipe name found in this file.'

  let suggestedName = recipeName
  let suggestedNumber = ''
  const suggestedBrewDate = isBatch ? bfMsToDate(json.brewDate) : ''

  if (isBatch) {
    if (json.name) suggestedName = String(json.name)
    if (json.batchNo != null) suggestedNumber = String(json.batchNo)
    else if (json.number != null) suggestedNumber = String(json.number)
  }

  return { mode: 'single', recipeJson: JSON.stringify(recipe), recipeName, suggestedName, suggestedNumber, suggestedBrewDate }
}

export function BatchImportPage() {
  const navigate = useNavigate()
  const importRecipe = useImportRecipe()
  const createBatch = useCreateBatch()

  const [parsed, setParsed] = useState<ParsedFile | null>(null)
  const [fileError, setFileError] = useState<string | null>(null)

  // Single mode state
  const [batchName, setBatchName] = useState('')
  const [batchNumber, setBatchNumber] = useState('')
  const [brewDate, setBrewDate] = useState('')
  const [singleError, setSingleError] = useState<string | null>(null)
  const [singleImporting, setSingleImporting] = useState(false)

  // Multi mode state
  const [selected, setSelected] = useState<Set<number>>(new Set())
  const [statuses, setStatuses] = useState<Record<number, ItemStatus>>({})
  const [statusErrors, setStatusErrors] = useState<Record<number, string>>({})
  const [multiImporting, setMultiImporting] = useState(false)
  const [search, setSearch] = useState('')

  function handleFile(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0]
    if (!file) return
    setParsed(null)
    setFileError(null)
    setSingleError(null)
    setStatuses({})
    setStatusErrors({})
    setSearch('')
    const reader = new FileReader()
    reader.onload = (ev) => {
      const result = parseBrewfatherFile(ev.target?.result as string)
      if (typeof result === 'string') { setFileError(result); return }
      setParsed(result)
      if (result.mode === 'single') {
        setBatchName(result.suggestedName)
        setBatchNumber(result.suggestedNumber)
        setBrewDate(result.suggestedBrewDate)
      } else {
        setSelected(new Set(result.batches.map((_, i) => i)))
      }
    }
    reader.readAsText(file)
  }

  // Single import
  async function handleSingleImport() {
    if (parsed?.mode !== 'single' || !batchNumber || !batchName) return
    setSingleImporting(true)
    setSingleError(null)
    try {
      const recipe = await importRecipe.mutateAsync({ format: 'brewfather', data: parsed.recipeJson })
      if (!recipe.id) throw new Error('Recipe import did not return an ID')
      const response = await createBatch.mutateAsync({
        recipe_id: recipe.id,
        batch_number: batchNumber,
        name: batchName,
        brew_date: brewDate || undefined,
      })
      navigate(`/batches/${response.batch?.id}`)
    } catch (err) {
      setSingleError(err instanceof APIError ? err.message : err instanceof Error ? err.message : 'Import failed')
      setSingleImporting(false)
    }
  }

  // Multi import
  async function handleMultiImport() {
    if (parsed?.mode !== 'multi') return
    setMultiImporting(true)
    const indices = [...selected].sort((a, b) => a - b)

    for (const i of indices) {
      if (statuses[i] === 'done') continue
      setStatuses((prev) => ({ ...prev, [i]: 'importing' }))
      const batch = parsed.batches[i]
      try {
        const recipe = await importRecipe.mutateAsync({ format: 'brewfather', data: batch.recipeJson })
        if (!recipe.id) throw new Error('Recipe import did not return an ID')
        const batchNo = batch.batchNo || String(i + 1)
        await createBatch.mutateAsync({
          recipe_id: recipe.id,
          batch_number: batchNo,
          name: batch.name,
          brew_date: batch.brewDate || undefined,
          initial_status: (mapBfStatus(batch.bfStatus) as components['schemas']['CreateBatchRequest']['initial_status']) || undefined,
        })
        setStatuses((prev) => ({ ...prev, [i]: 'done' }))
      } catch (err) {
        const msg = err instanceof APIError ? err.message : err instanceof Error ? err.message : 'Failed'
        setStatuses((prev) => ({ ...prev, [i]: 'error' }))
        setStatusErrors((prev) => ({ ...prev, [i]: msg }))
      }
    }
    setMultiImporting(false)
  }

  function toggleAll() {
    if (parsed?.mode !== 'multi') return
    if (selected.size === parsed.batches.length) setSelected(new Set())
    else setSelected(new Set(parsed.batches.map((_, i) => i)))
  }

  function toggleOne(i: number) {
    setSelected((prev) => {
      const next = new Set(prev)
      next.has(i) ? next.delete(i) : next.add(i)
      return next
    })
  }

  const filteredList = useMemo(() => {
    if (parsed?.mode !== 'multi') return []
    if (!search) return parsed.batches.map((b, i) => ({ ...b, i }))
    const q = search.toLowerCase()
    return parsed.batches.map((b, i) => ({ ...b, i })).filter((b) => b.name.toLowerCase().includes(q))
  }, [parsed, search])

  const doneCount = Object.values(statuses).filter((s) => s === 'done').length
  const isMulti = parsed?.mode === 'multi'

  return (
    <div className="max-w-3xl">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-bold text-[var(--color-fg)]">Import Batch from Brewfather</h1>
        <button
          onClick={() => navigate('/batches')}
          className="px-4 py-1.5 rounded text-sm border border-[var(--color-border)] text-[var(--color-fg)] hover:bg-[var(--color-border)]"
        >
          Cancel
        </button>
      </div>

      <div className="mb-6 p-4 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-sm text-[var(--color-muted)]">
        Upload a Brewfather recipe, batch, or "Export All" JSON. For single exports, the recipe is imported and a new batch is created linked to it.
      </div>

      {fileError && (
        <div className="mb-4 p-3 rounded text-sm" style={{ background: 'var(--color-danger-bg, #fff5f5)', color: 'var(--color-danger)' }}>
          {fileError}
        </div>
      )}

      <div className="mb-6">
        <input
          type="file"
          accept=".json,application/json"
          onChange={handleFile}
          disabled={multiImporting || singleImporting}
          className="block w-full text-sm text-[var(--color-fg)] file:mr-4 file:py-2 file:px-4 file:rounded file:border-0 file:text-sm file:bg-[var(--color-accent)] file:text-white hover:file:opacity-90"
        />
      </div>

      {/* Single mode */}
      {parsed?.mode === 'single' && (
        <div className="space-y-4">
          {singleError && (
            <div className="p-3 rounded text-sm" style={{ background: 'var(--color-danger-bg, #fff5f5)', color: 'var(--color-danger)' }}>
              {singleError}
            </div>
          )}

          <div className="p-3 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-sm text-[var(--color-muted)]">
            Recipe: <span className="text-[var(--color-fg)] font-medium">{parsed.recipeName}</span>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="flex flex-col gap-1">
              <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">
                Batch Number <span className="text-[var(--color-danger)]">*</span>
              </label>
              <input
                type="text"
                value={batchNumber}
                onChange={(e) => setBatchNumber(e.target.value)}
                placeholder="e.g. 2026-001"
                disabled={singleImporting}
                className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
              />
            </div>
            <div className="flex flex-col gap-1">
              <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">
                Batch Name <span className="text-[var(--color-danger)]">*</span>
              </label>
              <input
                type="text"
                value={batchName}
                onChange={(e) => setBatchName(e.target.value)}
                disabled={singleImporting}
                className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
              />
            </div>
          </div>

          <div className="flex flex-col gap-1">
            <label className="text-xs text-[var(--color-muted)] uppercase tracking-wide">Brew Date</label>
            <input
              type="date"
              value={brewDate}
              onChange={(e) => setBrewDate(e.target.value)}
              disabled={singleImporting}
              className="p-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-fg)]"
            />
          </div>

          <button
            onClick={handleSingleImport}
            disabled={!batchNumber.trim() || !batchName.trim() || singleImporting}
            className="px-6 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
          >
            {singleImporting ? 'Importing…' : 'Import & create batch'}
          </button>
        </div>
      )}

      {/* Multi mode (Export All) */}
      {isMulti && parsed.mode === 'multi' && (
        <>
          <div className="flex items-center justify-between mb-3 gap-4">
            <div className="flex items-center gap-3 text-sm text-[var(--color-muted)]">
              <span>{parsed.batches.length} batches</span>
              {doneCount > 0 && <span className="text-green-600">{doneCount} imported</span>}
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
                {selected.size === parsed.batches.length ? 'Deselect all' : 'Select all'}
              </button>
              <button
                onClick={handleMultiImport}
                disabled={multiImporting || selected.size === 0}
                className="px-5 py-1.5 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
              >
                {multiImporting ? 'Importing…' : `Import ${selected.size}`}
              </button>
            </div>
          </div>

          <div className="border rounded-lg overflow-hidden" style={{ borderColor: 'var(--color-border)', background: 'var(--color-surface)' }}>
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left text-xs text-[var(--color-muted)] uppercase tracking-wide border-b" style={{ borderColor: 'var(--color-border)', background: 'var(--color-bg)' }}>
                  <th className="p-2 w-8">
                    <input type="checkbox" checked={selected.size === parsed.batches.length} onChange={toggleAll} />
                  </th>
                  <th className="p-2">Batch #</th>
                  <th className="p-2">Name</th>
                  <th className="p-2">Status</th>
                  <th className="p-2">Brew Date</th>
                  <th className="p-2 w-8"></th>
                </tr>
              </thead>
              <tbody>
                {filteredList.map(({ i, name, batchNo, bfStatus, brewDate: bd }) => {
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
                          disabled={status === 'done' || multiImporting}
                          onChange={() => toggleOne(i)}
                        />
                      </td>
                      <td className="p-2 font-mono text-xs text-[var(--color-muted)]">{batchNo || '—'}</td>
                      <td className="p-2 font-medium text-[var(--color-fg)]">{name}</td>
                      <td className="p-2 text-xs text-[var(--color-muted)] capitalize">{mapBfStatus(bfStatus)}</td>
                      <td className="p-2 text-[var(--color-muted)]">{bd || '—'}</td>
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

          {doneCount > 0 && doneCount === selected.size && !multiImporting && (
            <div className="mt-4">
              <button
                onClick={() => navigate('/batches')}
                className="px-6 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90"
              >
                Done — go to batches
              </button>
            </div>
          )}
        </>
      )}
    </div>
  )
}
