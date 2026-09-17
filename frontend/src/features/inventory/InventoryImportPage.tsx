import React from 'react'
import { useNavigate } from 'react-router-dom'
import { useInventoryCreate } from './hooks/useInventory'
import { APIError } from '../../api/error'
import { parseBeerXML, parseBrewfather, parseCSV, TYPES, UNITS, validateRow, type Format, type Row } from './importers'
import { fileTooLarge, MAX_IMPORT_BYTES } from '../../utils/files'

type RowStatus = 'pending' | 'ok' | 'error'

const FORMAT_CONFIG: Record<Format, { label: string; accept: string }> = {
  csv: { label: 'CSV', accept: '.csv,text/csv' },
  beerxml: { label: 'BeerXML', accept: '.xml,application/xml,text/xml' },
  brewfather: { label: 'Brewfather JSON', accept: '.json,application/json' },
}

const cellInput = 'w-full bg-transparent border border-transparent rounded px-1 py-0.5 focus:outline-none focus:border-[var(--color-accent)] hover:border-[var(--color-border)] text-[var(--color-fg)] text-sm'

export function InventoryImportPage() {
  const navigate = useNavigate()
  const create = useInventoryCreate()

  const [format, setFormat] = React.useState<Format>('csv')
  const [rows, setRows] = React.useState<Row[]>([])
  const [statuses, setStatuses] = React.useState<RowStatus[]>([])
  const [rowErrors, setRowErrors] = React.useState<string[]>([])
  const [fileError, setFileError] = React.useState<string | null>(null)
  const [importing, setImporting] = React.useState(false)
  const [done, setDone] = React.useState(false)

  function resetRows() {
    setRows([]); setStatuses([]); setRowErrors([]); setDone(false)
  }

  function handleFormatChange(f: Format) {
    setFormat(f); resetRows()
  }

  function handleFile(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0]
    if (!file) return
    setFileError(null)
    const tooLarge = fileTooLarge(file, MAX_IMPORT_BYTES)
    if (tooLarge) {
      setFileError(tooLarge)
      return
    }
    const reader = new FileReader()
    reader.onload = (ev) => {
      const content = ev.target?.result as string
      let parsed: Row[] = []
      if (format === 'csv') parsed = parseCSV(content)
      else if (format === 'beerxml') parsed = parseBeerXML(content)
      else if (format === 'brewfather') parsed = parseBrewfather(content)
      setRows(parsed)
      setStatuses(parsed.map(() => 'pending'))
      setRowErrors(parsed.map(() => ''))
      setDone(false)
    }
    reader.readAsText(file)
  }

  function updateRow(i: number, field: keyof Omit<Row, 'error'>, value: string) {
    setRows((prev) => {
      const updated = [...prev]
      const next = { ...updated[i], [field]: value }
      next.error = validateRow(next)
      updated[i] = next
      return updated
    })
    setStatuses((prev) => {
      if (prev[i] === 'ok') return prev
      const next = [...prev]; next[i] = 'pending'; return next
    })
    setRowErrors((prev) => {
      const next = [...prev]; next[i] = ''; return next
    })
  }

  async function handleImport() {
    setImporting(true)
    const newStatuses = [...statuses]
    const newErrors = [...rowErrors]
    for (let i = 0; i < rows.length; i++) {
      if (newStatuses[i] === 'ok') continue
      const row = rows[i]
      if (row.error) { newStatuses[i] = 'error'; newErrors[i] = row.error; continue }
      try {
        await create.mutateAsync({
          name: row.name,
          type: row.type as typeof TYPES[number],
          amount: Number(row.amount),
          unit: row.unit as typeof UNITS[number],
          lot_number: row.lot_number,
          best_before_date: row.best_before_date || undefined,
          supplier: row.supplier || undefined,
          notes: row.notes || undefined,
        })
        newStatuses[i] = 'ok'
        newErrors[i] = ''
      } catch (err) {
        newStatuses[i] = 'error'
        // Validation failures put the useful detail in `details.reason`
        // (e.g. "lot_number already exists"); `message` is just "Validation failed."
        newErrors[i] =
          err instanceof APIError
            ? typeof err.details?.reason === 'string'
              ? err.details.reason
              : err.message
            : err instanceof Error
              ? err.message
              : 'Failed'
      }
      setStatuses([...newStatuses])
      setRowErrors([...newErrors])
    }
    setImporting(false)
    setDone(true)
  }

  const validRows = rows.filter((r) => !r.error).length
  const invalidRows = rows.length - validRows
  const imported = statuses.filter((s) => s === 'ok').length
  const failed = statuses.filter((s, i) => s === 'error' && !rows[i]?.error).length

  return (
    <div className="max-w-5xl">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-bold text-[var(--color-fg)]">Import Inventory Lots</h1>
        <button
          onClick={() => navigate('/inventory')}
          className="px-4 py-2 rounded text-sm border border-[var(--color-border)] text-[var(--color-fg)] hover:bg-[var(--color-border)]"
        >
          Back to inventory
        </button>
      </div>

      <div className="flex gap-2 mb-6">
        {(Object.keys(FORMAT_CONFIG) as Format[]).map((f) => (
          <button
            key={f}
            onClick={() => handleFormatChange(f)}
            className={`px-4 py-2 rounded text-sm border transition-colors ${
              format === f
                ? 'bg-[var(--color-accent)] text-white border-[var(--color-accent)]'
                : 'border-[var(--color-border)] text-[var(--color-fg)] hover:bg-[var(--color-border)]'
            }`}
          >
            {FORMAT_CONFIG[f].label}
          </button>
        ))}
      </div>

      <div className="mb-6 p-4 rounded border border-[var(--color-border)] bg-[var(--color-surface)]">
        {format === 'csv' && (
          <>
            <p className="text-sm font-semibold text-[var(--color-fg)] mb-2">CSV format</p>
            <code className="text-xs text-[var(--color-muted)] block whitespace-pre">
              name,type,amount,unit,lot_number,best_before_date,supplier,notes{'\n'}
              Maris Otter,fermentable,25,kg,LOT-001,2027-12-31,Thomas Fawcett,{'\n'}
              Citra Hops,hop,500,g,LOT-002,2026-06-30,,Whole leaf
            </code>
            <p className="text-xs text-[var(--color-muted)] mt-2">
              Required: <strong>name, type, amount, unit, lot_number</strong>.
              Types: {TYPES.join(', ')}. Units: {UNITS.join(', ')}.
            </p>
          </>
        )}
        {format === 'beerxml' && (
          <p className="text-sm text-[var(--color-muted)]">
            Extracts fermentables (kg), hops (kg), yeasts (L), and miscs (g). Edit cells directly to fill in lot numbers and adjust any values before importing.
          </p>
        )}
        {format === 'brewfather' && (
          <p className="text-sm text-[var(--color-muted)]">
            Imports the full ingredient catalogue from a Brewfather "Export All" file — fermentables, hops, yeasts, and miscs — including items with no stock on hand (imported at 0). A unique lot number is generated where Brewfather has none; real lot numbers are kept. Edit any cell before importing.
          </p>
        )}
      </div>

      <div className="mb-6">
        <input
          key={format}
          type="file"
          accept={FORMAT_CONFIG[format].accept}
          onChange={handleFile}
          disabled={importing}
          className="block w-full text-sm text-[var(--color-fg)] file:mr-4 file:py-2 file:px-4 file:rounded file:border-0 file:text-sm file:bg-[var(--color-accent)] file:text-white hover:file:opacity-90"
        />
        {fileError && <p role="alert" className="text-sm text-[var(--color-danger)]">{fileError}</p>}
      </div>

      {rows.length > 0 && (
        <>
          <div className="mb-4 flex items-center justify-between gap-4 flex-wrap">
            <div className="flex items-center gap-4 text-sm">
              <span className="text-[var(--color-fg)]">{rows.length} rows</span>
              {invalidRows > 0 && <span className="text-[var(--color-danger)]">{invalidRows} need fixing</span>}
              {done && <span className="text-green-600">{imported} imported</span>}
              {done && failed > 0 && <span className="text-[var(--color-danger)]">{failed} failed</span>}
            </div>
            {!done ? (
              <button
                onClick={handleImport}
                disabled={importing || validRows === 0}
                title={validRows === 0 ? 'Fill in required fields (name, type, amount, unit, lot number) for at least one row' : undefined}
                className="px-6 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
              >
                {importing ? 'Importing…' : validRows === 0 ? 'Fix required fields to import' : `Import ${validRows} lot${validRows !== 1 ? 's' : ''}`}
              </button>
            ) : (
              <button
                onClick={() => navigate('/inventory')}
                className="px-6 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90"
              >
                Done — go to inventory
              </button>
            )}
          </div>

          <div className="overflow-x-auto border rounded-lg mb-6" style={{ borderColor: 'var(--color-border)', background: 'var(--color-surface)' }}>
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left text-xs text-[var(--color-muted)] uppercase tracking-wide border-b" style={{ borderColor: 'var(--color-border)', background: 'var(--color-bg)' }}>
                  <th className="p-2 w-6"></th>
                  <th className="p-2">Name</th>
                  <th className="p-2">Type</th>
                  <th className="p-2">Amount</th>
                  <th className="p-2">Unit</th>
                  <th className="p-2">Lot #</th>
                  <th className="p-2">Best Before</th>
                  <th className="p-2">Supplier</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((row, i) => {
                  const status = statuses[i]
                  const apiError = status === 'error' ? rowErrors[i] : ''
                  const displayError = row.error || apiError
                  const isOk = status === 'ok'
                  const hasError = !!displayError
                  const disabled = isOk || importing

                  return (
                    <tr
                      key={i}
                      className="border-t"
                      style={{
                        borderColor: 'var(--color-border)',
                        background: hasError
                          ? 'var(--color-danger-bg, #fff5f5)'
                          : isOk
                          ? 'var(--color-success-bg, #f0fff4)'
                          : undefined,
                      }}
                    >
                      <td className="p-2 text-center w-6 shrink-0">
                        {isOk && <span className="text-green-600">&#10003;</span>}
                        {hasError && <span className="text-[var(--color-danger)]">&#10007;</span>}
                      </td>

                      <td className="p-1">
                        <input
                          className={cellInput}
                          value={row.name}
                          disabled={disabled}
                          onChange={(e) => updateRow(i, 'name', e.target.value)}
                          placeholder="Name"
                        />
                        {displayError && (
                          <div className="text-xs text-[var(--color-danger)] px-1">{displayError}</div>
                        )}
                      </td>

                      <td className="p-1">
                        <select
                          className={cellInput}
                          value={row.type}
                          disabled={disabled}
                          onChange={(e) => updateRow(i, 'type', e.target.value)}
                        >
                          <option value="">—</option>
                          {TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
                        </select>
                      </td>

                      <td className="p-1 w-24">
                        <input
                          className={cellInput}
                          type="number"
                          min="0"
                          step="any"
                          value={row.amount}
                          disabled={disabled}
                          onChange={(e) => updateRow(i, 'amount', e.target.value)}
                          placeholder="0"
                        />
                      </td>

                      <td className="p-1 w-24">
                        <select
                          className={cellInput}
                          value={row.unit}
                          disabled={disabled}
                          onChange={(e) => updateRow(i, 'unit', e.target.value)}
                        >
                          <option value="">—</option>
                          {UNITS.map((u) => <option key={u} value={u}>{u}</option>)}
                        </select>
                      </td>

                      <td className="p-1">
                        <input
                          className={cellInput}
                          value={row.lot_number}
                          disabled={disabled}
                          onChange={(e) => updateRow(i, 'lot_number', e.target.value)}
                          placeholder="LOT-001"
                        />
                      </td>

                      <td className="p-1 w-32">
                        <input
                          className={cellInput}
                          type="date"
                          value={row.best_before_date}
                          disabled={disabled}
                          onChange={(e) => updateRow(i, 'best_before_date', e.target.value)}
                        />
                      </td>

                      <td className="p-1">
                        <input
                          className={cellInput}
                          value={row.supplier}
                          disabled={disabled}
                          onChange={(e) => updateRow(i, 'supplier', e.target.value)}
                          placeholder="Supplier"
                        />
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>

        </>
      )}
    </div>
  )
}
