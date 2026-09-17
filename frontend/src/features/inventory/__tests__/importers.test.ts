import { describe, expect, it } from 'vitest'
import {
  makeLotNumber,
  parseBeerXML,
  parseBestBefore,
  parseBrewfather,
  parseCSV,
  validateRow,
} from '../importers'

const validRow = {
  name: 'Maris Otter',
  type: 'fermentable',
  amount: '25',
  unit: 'kg',
  lot_number: 'MO-1',
  best_before_date: '',
  supplier: '',
  notes: '',
}

describe('validateRow', () => {
  it('accepts a valid row, including zero stock', () => {
    expect(validateRow(validRow)).toBeUndefined()
    expect(validateRow({ ...validRow, amount: '0' })).toBeUndefined()
  })

  it('reports the first problem', () => {
    expect(validateRow({ ...validRow, name: '' })).toBe('name required')
    expect(validateRow({ ...validRow, type: 'grain' })).toMatch(/^type must be one of/)
    expect(validateRow({ ...validRow, amount: '-1' })).toBe('amount must be 0 or a positive number')
    expect(validateRow({ ...validRow, unit: 'lb' })).toMatch(/^unit must be one of/)
    expect(validateRow({ ...validRow, lot_number: '' })).toBe('lot_number required')
  })
})

describe('makeLotNumber', () => {
  it('sanitises a real lot and synthesises unique ones', () => {
    const seen = new Set<string>()
    expect(makeLotNumber('AB 12/3', 'hop', 'Citra', seen)).toBe('AB-12-3')
    expect(makeLotNumber('', 'hop', 'Citra', seen)).toBe('BF-HOP-CITRA')
    expect(makeLotNumber(undefined, 'hop', 'Citra', seen)).toBe('BF-HOP-CITRA-2')
  })
})

describe('parseBestBefore', () => {
  it('reads d/m/y and d-Mon-y dates from notes', () => {
    expect(parseBestBefore('Best Before: 5/3/2027')).toBe('2027-03-05')
    expect(parseBestBefore('best before 07-Nov-2026')).toBe('2026-11-07')
    expect(parseBestBefore('no date here')).toBe('')
  })
})

describe('parseCSV', () => {
  it('maps columns by header and validates each row', () => {
    const rows = parseCSV(
      'name,type,amount,unit,lot_number\nMaris Otter,fermentable,25,kg,MO-1\nBad,grain,1,kg,B-1',
    )
    expect(rows).toHaveLength(2)
    expect(rows[0]).toMatchObject({ name: 'Maris Otter', amount: '25', lot_number: 'MO-1', error: undefined })
    expect(rows[1].error).toMatch(/^type must be one of/)
  })

  it('needs a header and at least one row', () => {
    expect(parseCSV('name,type')).toEqual([])
  })
})

describe('parseBeerXML', () => {
  it('reads fermentables and converts misc amounts from kg to g', () => {
    const xml = `<RECIPES><RECIPE>
      <FERMENTABLES><FERMENTABLE><NAME>Pale Malt</NAME><AMOUNT>4.5</AMOUNT><BATCH_ID>PM-1</BATCH_ID></FERMENTABLE></FERMENTABLES>
      <MISCS><MISC><NAME>Irish Moss</NAME><AMOUNT>0.005</AMOUNT><BATCH_ID>IM-1</BATCH_ID></MISC></MISCS>
    </RECIPE></RECIPES>`
    const rows = parseBeerXML(xml)
    expect(rows).toHaveLength(2)
    expect(rows[0]).toMatchObject({ name: 'Pale Malt', type: 'fermentable', amount: '4.5', unit: 'kg', lot_number: 'PM-1' })
    expect(rows[1]).toMatchObject({ name: 'Irish Moss', type: 'adjunct', amount: '5', unit: 'g' })
  })

  it('returns nothing for malformed XML', () => {
    expect(parseBeerXML('<RECIPES><unclosed>')).toEqual([])
  })
})

describe('parseBrewfather', () => {
  it('imports the whole inventory export with unique synthesised lots', () => {
    const json = JSON.stringify({
      _type: 'Brewfather_Export_User_1',
      data: {
        inventory: {
          hops: [
            { name: 'Citra', inventory: 100 },
            { name: 'Citra', inventory: -5 },
          ],
          miscs: [{ name: 'Gypsum', type: 'Water Agent', inventory: 50, unit: 'g' }],
        },
      },
    })
    const rows = parseBrewfather(json)
    expect(rows.map((r) => [r.type, r.amount, r.lot_number])).toEqual([
      ['hop', '100', 'BF-HOP-CITRA'],
      ['hop', '0', 'BF-HOP-CITRA-2'],
      ['chemical', '50', 'BF-CHEM-GYPSUM'],
    ])
  })

  it('returns nothing for invalid JSON', () => {
    expect(parseBrewfather('{not json')).toEqual([])
  })
})
