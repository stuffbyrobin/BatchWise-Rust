import React from 'react'
import { useNavigate } from 'react-router-dom'
import { useInventoryCreate } from './hooks/useInventory'
import { APIError } from '../../api/error'

const TYPES = ['fermentable', 'hop', 'yeast', 'adjunct', 'chemical', 'other'] as const
const UNITS = ['kg', 'g', 'L', 'mL', 'count'] as const

type Format = 'csv' | 'beerxml' | 'brewfather'

type Row = {
  name: string
  type: string
  amount: string
  unit: string
  lot_number: string
  best_before_date: string
  supplier: string
  notes: string
  error?: string
}

type RowStatus = 'pending' | 'ok' | 'error'

function validateRow(row: Omit<Row, 'error'>): string | undefined {
  if (!row.name) return 'name required'
  if (!TYPES.includes(row.type as typeof TYPES[number])) return `type must be one of: ${TYPES.join(', ')}`
  // Zero is allowed: an ingredient may be imported on-record with no stock held.
  if (row.amount === '' || isNaN(Number(row.amount)) || Number(row.amount) < 0) return 'amount must be 0 or a positive number'
  if (!UNITS.includes(row.unit as typeof UNITS[number])) return `unit must be one of: ${UNITS.join(', ')}`
  if (!row.lot_number) return 'lot_number required'
  return undefined
}

// Inventory is lot-based: lot numbers must be 1–100 chars of [A-Za-z0-9-] and
// unique per tenant. A Brewfather catalogue often has missing or invalid lot
// numbers, so synthesise a unique, valid one — preserve a real lot (sanitised),
// otherwise derive `BF-<TYPE>-<NAME-SLUG>` — de-duplicating within the import.
function makeLotNumber(raw: unknown, type: string, name: string, seen: Set<string>): string {
  const sanitize = (s: string) =>
    s.replace(/[^A-Za-z0-9-]/g, '-').replace(/-+/g, '-').replace(/^-+|-+$/g, '').slice(0, 100)
  let base = sanitize(String(raw ?? ''))
  if (!base) {
    const slug = sanitize(name).toUpperCase()
    base = sanitize(`BF-${type.slice(0, 4).toUpperCase()}-${slug || 'ITEM'}`).slice(0, 90)
  }
  if (!base) base = 'BF-ITEM'
  let lot = base
  let n = 2
  while (seen.has(lot)) lot = `${base}-${n++}`.slice(0, 100)
  seen.add(lot)
  return lot
}

function parseCSV(text: string): Row[] {
  const lines = text.split('\n').map((l) => l.trim()).filter(Boolean)
  if (lines.length < 2) return []
  const headers = lines[0].split(',').map((h) => h.trim().toLowerCase())
  return lines.slice(1).map((line) => {
    const cells = line.split(',').map((c) => c.trim().replace(/^"|"$/g, ''))
    const get = (key: string) => cells[headers.indexOf(key)] ?? ''
    const base: Omit<Row, 'error'> = {
      name: get('name'),
      type: get('type'),
      amount: get('amount'),
      unit: get('unit'),
      lot_number: get('lot_number'),
      best_before_date: get('best_before_date'),
      supplier: get('supplier'),
      notes: get('notes'),
    }
    return { ...base, error: validateRow(base) }
  })
}

function parseBestBefore(notes: string): string {
  const match = notes.match(/Best Before[:\s]+([^\n]+)/i)
  if (!match) return ''
  const raw = match[1].trim()
  const dmy = raw.match(/^(\d{1,2})\/(\d{1,2})\/(\d{4})$/)
  if (dmy) return `${dmy[3]}-${dmy[2].padStart(2, '0')}-${dmy[1].padStart(2, '0')}`
  const MONTHS: Record<string, string> = {
    Jan: '01', Feb: '02', Mar: '03', Apr: '04', May: '05', Jun: '06',
    Jul: '07', Aug: '08', Sep: '09', Oct: '10', Nov: '11', Dec: '12',
  }
  const dMonY = raw.match(/^(\d{1,2})-([A-Za-z]{3})-(\d{4})$/)
  if (dMonY) {
    const m = MONTHS[dMonY[2]]
    if (m) return `${dMonY[3]}-${m}-${dMonY[1].padStart(2, '0')}`
  }
  return ''
}

function parseBeerXML(text: string): Row[] {
  const parser = new DOMParser()
  const doc = parser.parseFromString(text, 'application/xml')
  if (doc.getElementsByTagName('parsererror').length > 0) return []

  const rows: Row[] = []
  // getElementsByTagName is reliable for XML documents; querySelector is not.
  const getText = (parent: Element, tag: string) =>
    parent.getElementsByTagName(tag)[0]?.textContent?.trim() ?? ''

  const sections: Array<{ tag: string; type: string; unit: string }> = [
    { tag: 'FERMENTABLE', type: 'fermentable', unit: 'kg' },
    { tag: 'HOP', type: 'hop', unit: 'kg' },
    { tag: 'YEAST', type: 'yeast', unit: 'L' },
    { tag: 'MISC', type: 'adjunct', unit: 'g' },
  ]

  for (const { tag, type, unit } of sections) {
    Array.from(doc.getElementsByTagName(tag)).forEach((el) => {
      const name = getText(el, 'NAME')
      if (!name) return
      const rawAmount = getText(el, 'AMOUNT')
      const amount = type === 'adjunct' && rawAmount
        ? String(Math.round(Number(rawAmount) * 1000 * 10000) / 10000)
        : rawAmount
      const notes = getText(el, 'NOTES')
      const base: Omit<Row, 'error'> = {
        name, type, amount, unit,
        lot_number: getText(el, 'BATCH_ID'),
        best_before_date: parseBestBefore(notes),
        supplier: '',
        notes: notes || 'Imported from BeerXML',
      }
      rows.push({ ...base, error: validateRow(base) })
    })
  }
  return rows
}

function bfMsToDate(ts: unknown): string {
  if (ts == null || ts === 0 || ts === false) return ''
  const n = Number(ts)
  if (!n || n <= 0) return ''
  const d = new Date(n > 1e10 ? n : n * 1000)
  return d.toISOString().split('T')[0]
}

function bfNormaliseUnit(raw: unknown, fallback: string): string {
  const map: Record<string, string> = { kg: 'kg', g: 'g', l: 'L', ml: 'mL', count: 'count', pkg: 'count' }
  return map[String(raw ?? '').toLowerCase()] ?? fallback
}

function parseBrewfather(text: string): Row[] {
  let json: Record<string, unknown>
  try { json = JSON.parse(text) } catch { return [] }

  // ── Export All format: { _type: "Brewfather_Export_User_1", data: { inventory: { fermentables, hops, yeasts, miscs } } }
  if (json._type === 'Brewfather_Export_User_1') {
    const dataSection = json.data as Record<string, unknown> | undefined
    const inv = dataSection?.inventory as Record<string, unknown[]> | undefined
    if (!inv) return []
    const rows: Row[] = []
    // Whole catalogue is imported (including zero-stock items); `seen` keeps the
    // synthesised lot numbers unique to satisfy UNIQUE(tenant_id, lot_number).
    const seen = new Set<string>()

    for (const f of (inv.fermentables ?? []) as Record<string, unknown>[]) {
      // Clamp at 0: Brewfather can carry a negative tracking balance.
      const stock = Math.max(0, Number(f.inventory ?? 0), Number(f.amount ?? 0))
      const name = String(f.name ?? '')
      const base: Omit<Row, 'error'> = {
        name,
        type: 'fermentable',
        amount: String(stock),
        unit: 'kg',
        lot_number: makeLotNumber(f.lotNumber, 'fermentable', name, seen),
        best_before_date: bfMsToDate(f.bestBeforeDate),
        supplier: String(f.supplier ?? ''),
        notes: '',
      }
      rows.push({ ...base, error: validateRow(base) })
    }

    for (const h of (inv.hops ?? []) as Record<string, unknown>[]) {
      const stock = Math.max(0, Number(h.inventory ?? 0), Number(h.amount ?? 0))
      const name = String(h.name ?? '')
      const base: Omit<Row, 'error'> = {
        name,
        type: 'hop',
        amount: String(stock),
        unit: 'g',
        lot_number: makeLotNumber(h.lotNumber, 'hop', name, seen),
        best_before_date: bfMsToDate(h.bestBeforeDate),
        supplier: String(h.supplier ?? ''),
        notes: '',
      }
      rows.push({ ...base, error: validateRow(base) })
    }

    for (const y of (inv.yeasts ?? []) as Record<string, unknown>[]) {
      const stock = Math.max(0, Number(y.inventory ?? 0))
      const name = String(y.name ?? '')
      const base: Omit<Row, 'error'> = {
        name,
        type: 'yeast',
        amount: String(stock),
        unit: bfNormaliseUnit(y.unit, 'g'),
        lot_number: makeLotNumber(y.lotNumber, 'yeast', name, seen),
        best_before_date: bfMsToDate(y.bestBeforeDate),
        supplier: String(y.laboratory ?? ''),
        notes: '',
      }
      rows.push({ ...base, error: validateRow(base) })
    }

    const miscTypeMap: Record<string, string> = {
      'water agent': 'chemical', 'fining': 'chemical', 'nutrient': 'chemical',
      'spice': 'adjunct', 'herb': 'adjunct', 'flavor': 'adjunct', 'other': 'other',
    }
    for (const m of (inv.miscs ?? []) as Record<string, unknown>[]) {
      const stock = Math.max(0, Number(m.inventory ?? 0))
      const name = String(m.name ?? '')
      const type = miscTypeMap[String(m.type ?? '').toLowerCase()] ?? 'adjunct'
      const base: Omit<Row, 'error'> = {
        name,
        type,
        amount: String(stock),
        unit: bfNormaliseUnit(m.unit, 'g'),
        lot_number: makeLotNumber(m.lotNumber, type, name, seen),
        best_before_date: bfMsToDate(m.bestBeforeDate),
        supplier: '',
        notes: '',
      }
      rows.push({ ...base, error: validateRow(base) })
    }

    return rows
  }

  // ── Single batch export: recipe nested under 'recipe' key
  // ── Single recipe export: ingredients at top level
  const source = (json.recipe && typeof json.recipe === 'object' && !Array.isArray(json.recipe))
    ? json.recipe as Record<string, unknown>
    : json

  const rows: Row[] = []
  const sections: Array<{ key: string; type: string; defaultUnit: string }> = [
    { key: 'fermentables', type: 'fermentable', defaultUnit: 'kg' },
    { key: 'hops', type: 'hop', defaultUnit: 'g' },
    { key: 'yeasts', type: 'yeast', defaultUnit: 'g' },
    { key: 'miscs', type: 'adjunct', defaultUnit: 'g' },
  ]

  for (const { key, type, defaultUnit } of sections) {
    const items = source[key]
    if (!Array.isArray(items)) continue
    for (const item of items) {
      const name = String(item.name ?? '')
      const rawAmount = item.amount != null ? Number(item.amount) : 0
      const base: Omit<Row, 'error'> = {
        name, type,
        amount: rawAmount > 0 ? String(rawAmount) : '',
        unit: bfNormaliseUnit(item.unit, defaultUnit),
        lot_number: String(item.lotNumber ?? ''),
        best_before_date: bfMsToDate(item.bestBeforeDate),
        supplier: String(item.supplier ?? ''),
        notes: 'Imported from Brewfather',
      }
      rows.push({ ...base, error: validateRow(base) })
    }
  }
  return rows
}

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
