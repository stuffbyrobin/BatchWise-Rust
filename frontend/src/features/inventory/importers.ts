/**
 * Parsers that turn an uploaded inventory file (CSV, BeerXML, Brewfather JSON)
 * into editable rows with per-row validation. Pure functions: no React, no I/O.
 */

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

export { TYPES, UNITS, makeLotNumber, parseCSV, parseBestBefore, parseBeerXML, bfMsToDate, bfNormaliseUnit, parseBrewfather, validateRow }
export type { Format, Row }
