import { fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { BatchIngredientsEditor } from '../BatchIngredientsEditor'
import type { Batch } from '../hooks/useBatches'
import { calcHopIBU } from '../../../utils/ibu'

const mutate = vi.fn()
const inventoryList = vi.fn()

vi.mock('../hooks/useBatches', () => ({
  usePatchBatchIngredients: () => ({ mutate, isPending: false }),
}))
vi.mock('../../inventory/hooks/useInventory', () => ({
  useInventoryList: (params: { name?: string; type?: string }) => inventoryList(params),
}))
vi.mock('../../account/hooks/useTenant', () => ({ useTenant: () => ({ data: { ibu_method: 'tinseth' } }) }))

function makeBatch(): Batch {
  return {
    id: 'b1',
    status: 'planned',
    target_og: 1.05,
    actual_volume_liters: null,
    batch_recipe_snapshot: {
      schema_version: 1,
      recipe_id: 'r1',
      name: 'IPA',
      type: 'all_grain',
      batch_size_liters: 20,
      fermentables: [
        { id: 'f1', recipe_id: 'r1', step_order: 1, name: 'Pale Malt', amount: 4.5, unit: 'kg', color_ebc: 5, potential_ppg: 37, type: 'Grain', addition: null },
        { id: 'f2', recipe_id: 'r1', step_order: 2, name: 'Crystal', amount: 0.3, unit: 'kg', color_ebc: 120, potential_ppg: 34, type: 'Grain', addition: null },
      ],
      hops: [{ id: 'h1', recipe_id: 'r1', step_order: 1, name: 'Citra', amount: 0.03, unit: 'kg', alpha_acid_pct: 12, boil_time_minutes: 60, form: 'pellet', use: 'boil' }],
      yeasts: [{ id: 'y1', recipe_id: 'r1', yeast_id: null, name: 'US-05', amount: 1, unit: 'count', attenuation_pct: 78 }],
      mash_steps: [],
    },
  } as unknown as Batch
}

type Payload = Record<'fermentables' | 'hops' | 'yeasts', Record<string, unknown>[]>
function save(): Payload {
  fireEvent.click(screen.getByRole('button', { name: 'Save ingredients' }))
  expect(mutate).toHaveBeenCalledTimes(1)
  return mutate.mock.calls[0][0] as Payload
}
const inputs = (role: string, name: string) => screen.getAllByRole(role, { name }) as HTMLInputElement[]

beforeEach(() => {
  mutate.mockReset()
  inventoryList.mockReset()
  inventoryList.mockReturnValue({ data: { items: [] } })
})

describe('BatchIngredientsEditor', () => {
  it('shows snapshot rows in labelled cells', () => {
    render(<BatchIngredientsEditor batch={makeBatch()} canEdit />)
    const fermentableNames = inputs('textbox', 'Fermentable name')
    expect(fermentableNames).toHaveLength(2)
    expect(fermentableNames[0].value).toBe('Pale Malt')
    expect(fermentableNames[1].value).toBe('Crystal')

    const hopForms = inputs('combobox', 'Hop form')
    expect(hopForms[0].value).toBe('pellet')

    const hopUses = inputs('combobox', 'Hop use')
    expect(hopUses[0].value).toBe('boil')

    const hopUnits = inputs('combobox', 'Hop unit')
    expect(hopUnits[0].value).toBe('kg')

    const yeastAmounts = inputs('spinbutton', 'Yeast amount')
    expect(yeastAmounts[0].value).toBe('1')
  })

  it('estimates hop IBU from a kilogram amount', () => {
    render(<BatchIngredientsEditor batch={makeBatch()} canEdit />)
    const expectedIBU = calcHopIBU('tinseth', 30, 12, 60, 20, 1.05).toFixed(1)
    expect(screen.getByText(expectedIBU)).toBeInTheDocument()
  })

  it('saves typed rows without client uids, including added rows', () => {
    render(<BatchIngredientsEditor batch={makeBatch()} canEdit />)

    fireEvent.click(screen.getByRole('button', { name: '+ Add fermentable' }))
    fireEvent.click(screen.getByRole('button', { name: '+ Add hop' }))
    fireEvent.click(screen.getByRole('button', { name: '+ Add yeast' }))

    fireEvent.change(inputs('textbox', 'Hop name')[1], { target: { value: 'Mosaic' } })
    fireEvent.change(inputs('spinbutton', 'Fermentable amount')[0], { target: { value: '' } })
    fireEvent.change(inputs('spinbutton', 'Colour (EBC)')[0], { target: { value: '' } })

    const payload = save()

    for (const row of [...payload.fermentables, ...payload.hops, ...payload.yeasts]) {
      expect(row).not.toHaveProperty('uid')
    }

    expect(payload.fermentables[0]).toMatchObject({ id: 'f1', amount: 0, color_ebc: null })
    expect(payload.fermentables[2]).toEqual({ step_order: 3, name: '', amount: 0, unit: 'kg', type: 'Grain', color_ebc: null, potential_ppg: null, addition: null })
    expect(payload.hops[1]).toEqual({ step_order: 2, name: 'Mosaic', amount: 0, unit: 'g', alpha_acid_pct: 0, boil_time_minutes: 60, use: 'boil', form: null })
    expect(payload.yeasts[1]).toEqual({ name: '', amount: 1, unit: 'count', attenuation_pct: null, yeast_id: null })
  })

  it('removing a row keeps the other rows intact', () => {
    render(<BatchIngredientsEditor batch={makeBatch()} canEdit />)

    fireEvent.click(screen.getAllByRole('button', { name: 'Remove fermentable' })[0])

    const fermentableNames = inputs('textbox', 'Fermentable name')
    expect(fermentableNames).toHaveLength(1)
    expect(fermentableNames[0].value).toBe('Crystal')

    const colorInputs = inputs('spinbutton', 'Colour (EBC)')
    expect(colorInputs[0].value).toBe('120')

    const payload = save()
    expect(payload.fermentables).toHaveLength(1)
    expect(payload.fermentables[0].id).toBe('f2')
  })

  it('picking a stock lot links it and copies the lot colour', () => {
    inventoryList.mockImplementation((p: { name?: string; type?: string }) => ({
      data: { items: p.type === 'fermentable' && p.name === 'Pale Malt' ? [{ id: 'lot-1', lot_number: 'PM-1', amount: 25, unit: 'kg', color_ebc: 6.5 }] : [] }
    }))
    render(<BatchIngredientsEditor batch={makeBatch()} canEdit />)

    fireEvent.change(screen.getByRole('combobox', { name: 'Fermentable lot' }), { target: { value: 'lot-1' } })

    const payload = save()
    expect(payload.fermentables[0]).toMatchObject({ inventory_lot_id: 'lot-1', color_ebc: 6.5 })
  })

  it('locked batches are read-only', () => {
    render(<BatchIngredientsEditor batch={makeBatch()} canEdit={false} />)

    expect(screen.queryByRole('button', { name: 'Save ingredients' })).toBeNull()
    expect(screen.queryAllByRole('button', { name: /Remove/ })).toHaveLength(0)

    const fermentableNameInputs = inputs('textbox', 'Fermentable name')
    fermentableNameInputs.forEach((el) => expect(el).toBeDisabled())

    expect(screen.getByText('Locked — terminal status')).toBeInTheDocument()
  })
})
