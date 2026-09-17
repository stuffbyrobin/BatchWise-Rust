import { fireEvent, render, screen, within } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { calcHopIBU } from '../../../utils/ibu'
import RecipeEditorPage from '../RecipeEditorPage'

const mutate = vi.fn()
const recipeQuery = vi.fn()

vi.mock('../hooks/useRecipes', () => ({
  useRecipe: () => recipeQuery(),
  useCreateRecipe: () => ({ mutate: vi.fn(), isPending: false }),
  useUpdateRecipe: () => ({ mutate, isPending: false }),
}))
vi.mock('../hooks/useRecipeAllergens', () => ({ useRecipeAllergens: () => ({ data: undefined }) }))
vi.mock('../../account/hooks/useTenant', () => ({ useTenant: () => ({ data: { ibu_method: 'tinseth' } }) }))
let authRole = 'owner'
vi.mock('../../../auth/useAuth', () => ({ useAuth: () => ({ user: { feature_flags: {}, role: authRole } }) }))
vi.mock('../../../lib/physics/useBrewingPhysics', () => ({
  useBrewingPhysics: () => ({ ready: false, computeRecipeCalcs: vi.fn() }),
}))
vi.mock('../RecipeWaterChemistry', () => ({ RecipeWaterChemistry: () => null }))
vi.mock('../../../utils/ibu', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../../utils/ibu')>()
  return { ...actual, calcHopIBU: vi.fn(actual.calcHopIBU) }
})

function optionSet<O extends { key: string; name: string }>(groupLabel: string, options: O[]) {
  return {
    options,
    byKey: new Map(options.map((o) => [o.key, o])),
    byName: new Map(options.map((o) => [o.name, o])),
    groups: [{ label: groupLabel, options }],
    loading: false,
  }
}

const maltOptions = optionSet('Base Malt', [
  { key: 'generic:pale', name: 'Pale Malt', label: 'Pale Malt', type: 'Base Malt', color_ebc: 6, potential_ppg: 36, source: 'generic' as const },
  { key: 'generic:c60', name: 'Crystal 60', label: 'Crystal 60', type: 'Crystal', color_ebc: 120, potential_ppg: 34, source: 'generic' as const },
])
const hopOptions = optionSet('In stock', [
  { key: 'stock:Citra', name: 'Citra', label: 'Citra — 100 g in stock', alpha_acid_pct: 12.5, source: 'stock' as const },
  { key: 'stock:Mosaic', name: 'Mosaic', label: 'Mosaic — 50 g in stock', alpha_acid_pct: 11, source: 'stock' as const },
  { key: 'stock:Simcoe', name: 'Simcoe', label: 'Simcoe — 80 g in stock', alpha_acid_pct: 13, source: 'stock' as const },
])
const yeastOptions = optionSet('Ale', [
  { key: 'generic:us05', id: 'y-us05', name: 'US-05', label: 'US-05', attenuation_pct: 81, type: 'ale', source: 'generic' as const },
  { key: 'generic:s04', id: 'y-s04', name: 'S-04', label: 'S-04', attenuation_pct: 75, type: 'ale', source: 'generic' as const },
])

vi.mock('../useMaltOptions', () => ({ useMaltOptions: () => maltOptions }))
vi.mock('../useHopOptions', () => ({ useHopOptions: () => hopOptions }))
vi.mock('../useYeastOptions', () => ({ useYeastOptions: () => yeastOptions }))

function makeRecipe(overrides: Record<string, unknown> = {}) {
  return {
    id: 'r1',
    name: 'Test IPA',
    type: 'all_grain',
    batch_size_liters: 20,
    boil_size_liters: 25,
    boil_time_minutes: 60,
    efficiency_pct: 75,
    notes: '',
    calc_og: 1.05,
    calc_fg: 1.01,
    calc_abv_pct: 5.2,
    calc_ibu: 40,
    calc_color_ebc: 10,
    fermentables: [
      { step_order: 1, name: 'Pale Malt', amount: 4.5, unit: 'kg', color_ebc: 5, potential_ppg: 37, type: 'Base Malt', addition: null },
      { step_order: 2, name: 'Mystery Grain', amount: 0.5, unit: 'kg', color_ebc: 100, potential_ppg: 30, type: null, addition: null },
    ],
    hops: [
      { step_order: 1, name: 'Citra', amount: 30, unit: 'g', alpha_acid_pct: 12, boil_time_minutes: 60, form: 'pellet', use: 'boil' },
      { step_order: 2, name: 'Mosaic', amount: 20, unit: 'g', alpha_acid_pct: 11, boil_time_minutes: 10, form: null, use: null },
    ],
    yeasts: [{ yeast_id: null, name: 'US-05', amount: 11, unit: 'g', attenuation_pct: 78 }],
    mash_steps: [{ step_order: 1, step_type: 'infusion', target_temp_c: 67, hold_minutes: 60, infusion_volume_liters: 15 }],
    ...overrides,
  }
}

function renderEditor() {
  return render(
    <MemoryRouter initialEntries={['/recipes/r1']}>
      <Routes>
        <Route path="/recipes/:id" element={<RecipeEditorPage />} />
      </Routes>
    </MemoryRouter>,
  )
}

// Tables in page order: fermentables, hops, yeasts, mash steps. Header row skipped.
type Section = 0 | 1 | 2 | 3
function rows(section: Section) {
  return within(screen.getAllByRole('table')[section]).getAllByRole('row').slice(1)
}
function numbers(row: HTMLElement) {
  return within(row).getAllByRole('spinbutton') as HTMLInputElement[]
}
function selects(row: HTMLElement) {
  return within(row).getAllByRole('combobox') as HTMLSelectElement[]
}
function texts(row: HTMLElement) {
  return within(row).queryAllByRole('textbox') as HTMLInputElement[]
}
type Payload = Record<string, Record<string, unknown>[]>
function save(): Payload {
  fireEvent.click(screen.getByRole('button', { name: 'Save' }))
  expect(mutate).toHaveBeenCalledTimes(1)
  return mutate.mock.calls[0][0] as Payload
}

beforeEach(() => {
  authRole = 'owner'
  mutate.mockReset()
  recipeQuery.mockReturnValue({ data: makeRecipe(), isLoading: false, isError: false, error: null })
})

describe('RecipeEditorPage (characterization)', () => {
  it('hydrates every section from the loaded recipe', () => {
    renderEditor()

    // Check row counts
    expect(rows(0)).toHaveLength(2) // fermentables
    expect(rows(1)).toHaveLength(2) // hops
    expect(rows(2)).toHaveLength(1) // yeasts
    expect(rows(3)).toHaveLength(1) // mash steps

    // Fermentable row 0: picker should be 'generic:pale', amount 4.5
    const ferm0 = rows(0)[0]
    expect(selects(ferm0)[0].value).toBe('generic:pale')
    expect(numbers(ferm0)[1].value).toBe('4.5') // amount is the 2nd spinbutton (after step_order)

    // Fermentable row 1: custom mode with name 'Mystery Grain'
    const ferm1 = rows(0)[1]
    expect(selects(ferm1)[0].value).toBe('__custom__')
    expect(texts(ferm1)[0].value).toBe('Mystery Grain')

    // Hop row 0: picker should be 'stock:Citra'
    const hop0 = rows(1)[0]
    expect(selects(hop0)[0].value).toBe('stock:Citra')

    // Hop row 1: form select should be empty string (not undefined, but the DOM shows '')
    const hop1 = rows(1)[1]
    expect(selects(hop1)[2].value).toBe('')

    // Yeast row 0: picker should be 'generic:us05'
    const yeast0 = rows(2)[0]
    expect(selects(yeast0)[0].value).toBe('generic:us05')

    // Mash row 0: temp should be 67
    const mash0 = rows(3)[0]
    expect(numbers(mash0)[1].value).toBe('67')
  })

  it('saves edits as a typed payload without client uids', () => {
    renderEditor()

    // Change fermentable row 0 amount to 5.25
    fireEvent.change(numbers(rows(0)[0])[1], { target: { value: '5.25' } })

    // Change hop row 1 boil time to empty string (should become 0)
    fireEvent.change(numbers(rows(1)[1])[3], { target: { value: '' } })

    // Change yeast row 0 attenuation to 80
    fireEvent.change(numbers(rows(2)[0])[1], { target: { value: '80' } })

    const payload = save()

    // Check fermentables[0]
    expect(payload.fermentables[0]).toEqual({
      step_order: 1,
      name: 'Pale Malt',
      amount: 5.25,
      unit: 'kg',
      color_ebc: 5,
      potential_ppg: 37,
      type: 'Base Malt',
      addition: undefined,
    })

    // Check hops[1] boil_time_minutes is 0
    expect(payload.hops[1].boil_time_minutes).toBe(0)

    // Check yeasts[0] attenuation
    expect(payload.yeasts[0].attenuation_pct).toBe(80)

    // Check mash_steps[0]
    expect(payload.mash_steps[0]).toEqual({
      step_order: 1,
      step_type: 'infusion',
      target_temp_c: 67,
      hold_minutes: 60,
      infusion_volume_liters: 15,
    })

    // Check no uid in any row
    for (const row of payload.fermentables) expect(row).not.toHaveProperty('uid')
    for (const row of payload.hops) expect(row).not.toHaveProperty('uid')
    for (const row of payload.yeasts) expect(row).not.toHaveProperty('uid')
    for (const row of payload.mash_steps) expect(row).not.toHaveProperty('uid')
  })

  it('picking a malt fills its analytical fields and leaves custom mode', () => {
    renderEditor()

    const ferm1 = rows(0)[1]

    // Change picker to 'generic:c60'
    fireEvent.change(selects(ferm1)[0], { target: { value: 'generic:c60' } })

    // Should no longer be in custom mode - only type and addition textboxes remain
    expect(texts(ferm1)).toHaveLength(2)

    const payload = save()

    expect(payload.fermentables[1]).toMatchObject({
      name: 'Crystal 60',
      type: 'Crystal',
      color_ebc: 120,
      potential_ppg: 34,
      amount: 0.5,
    })
  })

  it('Custom mode keeps the typed name and saves it', () => {
    renderEditor()

    const hop0 = rows(1)[0]

    // Change picker to '__custom__'
    fireEvent.change(selects(hop0)[0], { target: { value: '__custom__' } })

    // The row now has a textbox with value 'Citra'
    expect(texts(hop0)[0].value).toBe('Citra')

    // Change it to 'Citra Cryo'
    fireEvent.change(texts(hop0)[0], { target: { value: 'Citra Cryo' } })

    const payload = save()

    expect(payload.hops[0].name).toBe('Citra Cryo')
  })

  it('custom mode follows the row when an earlier row is removed', () => {
    renderEditor()

    const hop1 = rows(1)[1]

    // Change hop row 1 picker to '__custom__'
    fireEvent.change(selects(hop1)[0], { target: { value: '__custom__' } })

    // Textbox with 'Mosaic' appears
    expect(texts(hop1)[0].value).toBe('Mosaic')

    // Click Remove button in hop row 0
    fireEvent.click(within(rows(1)[0]).getByRole('button', { name: 'Remove' }))

    // Now hops has 1 row
    expect(rows(1)).toHaveLength(1)

    const remainingHop = rows(1)[0]
    // Its picker should be '__custom__' and textbox should be 'Mosaic'
    expect(selects(remainingHop)[0].value).toBe('__custom__')
    expect(texts(remainingHop)[0].value).toBe('Mosaic')

    const payload = save()
    expect(payload.hops).toHaveLength(1)
    expect(payload.hops[0].name).toBe('Mosaic')
  })

  it('picking a library yeast keeps its strain id and attenuation', () => {
    renderEditor()

    const yeast0 = rows(2)[0]

    // Change yeast picker to 'generic:s04'
    fireEvent.change(selects(yeast0)[0], { target: { value: 'generic:s04' } })

    const payload = save()

    expect(payload.yeasts[0]).toMatchObject({
      name: 'S-04',
      yeast_id: 'y-s04',
      attenuation_pct: 75,
    })
  })

  it('add buttons append rows with the next step order', () => {
    renderEditor()

    fireEvent.click(screen.getByRole('button', { name: 'Add Hop' }))
    fireEvent.click(screen.getByRole('button', { name: 'Add Mash Step' }))
    fireEvent.click(screen.getByRole('button', { name: 'Add Yeast' }))
    fireEvent.click(screen.getByRole('button', { name: 'Add Fermentable' }))

    // Check row counts after adding
    expect(rows(0)).toHaveLength(3) // fermentables
    expect(rows(1)).toHaveLength(3) // hops
    expect(rows(2)).toHaveLength(2) // yeasts
    expect(rows(3)).toHaveLength(2) // mash steps

    const payload = save()

    expect(payload.hops[2]).toEqual({ step_order: 3, name: '', amount: 0, unit: 'g', alpha_acid_pct: 0, boil_time_minutes: 0 })
    expect(payload.mash_steps[1]).toEqual({ step_order: 2, step_type: 'infusion', target_temp_c: 0, hold_minutes: 0 })
    expect(payload.yeasts[1]).toEqual({ name: '', amount: 0, unit: 'g' })
    expect(payload.fermentables[2]).toEqual({ step_order: 3, name: '', amount: 0, unit: 'kg' })
  })

  it('shows a per-hop IBU estimate', () => {
    renderEditor()

    const hop0 = rows(1)[0]

    // Hop row 0 should have an IBU value matching /\d+\.\d/
    expect(within(hop0).getByText(/^\d+\.\d$/)).toBeInTheDocument()

    // Change hop row 0 amount to '0'
    fireEvent.change(numbers(hop0)[1], { target: { value: '0' } })

    // Now the row should contain '—'
    expect(within(hop0).getByText('—')).toBeInTheDocument()
  })

  it('a refetch of the same recipe does not overwrite edits', () => {
    const view = renderEditor()

    // Change fermentable row 0 amount to '6'
    fireEvent.change(numbers(rows(0)[0])[1], { target: { value: '6' } })

    // Simulate a refetch with a renamed recipe on server
    recipeQuery.mockReturnValue({ data: makeRecipe({ name: 'Renamed on server' }), isLoading: false, isError: false, error: null })

    view.rerender(
      <MemoryRouter initialEntries={['/recipes/r1']}>
        <Routes>
          <Route path="/recipes/:id" element={<RecipeEditorPage />} />
        </Routes>
      </MemoryRouter>
    )

    // Fermentable row 0 amount should still be '6'
    expect(numbers(rows(0)[0])[1].value).toBe('6')
    // Recipe name input should still be 'Test IPA'
    expect((screen.getByPlaceholderText('Recipe name') as HTMLInputElement).value).toBe('Test IPA')
  })

  it('gives every basic field and table cell an accessible name', () => {
    renderEditor()
    expect(screen.getByLabelText('Name *')).toHaveValue('Test IPA')
    expect(screen.getByLabelText('Batch Size (L) *')).toHaveValue(20)
    expect(screen.getByLabelText('Notes')).toHaveValue('')
    expect(screen.getAllByRole('combobox', { name: 'Malt' })).toHaveLength(2)
    expect(screen.getByRole('textbox', { name: 'Malt name' })).toHaveValue('Mystery Grain')
    expect(screen.getAllByRole('spinbutton', { name: 'Hop amount' })).toHaveLength(2)
    expect(screen.getByRole('combobox', { name: 'Yeast' })).toHaveValue('generic:us05')
    expect(screen.getByRole('spinbutton', { name: 'Infusion volume (L)' })).toHaveValue(15)
  })

  it('saves an emptied optional measurement as null', () => {
    renderEditor()
    fireEvent.change(screen.getByRole('spinbutton', { name: 'Infusion volume (L)' }), { target: { value: '' } })
    fireEvent.change(screen.getAllByRole('spinbutton', { name: 'Colour (EBC)' })[0], { target: { value: '' } })
    const payload = save()
    expect(payload.mash_steps[0].infusion_volume_liters).toBeNull()
    expect(payload.fermentables[0].color_ebc).toBeNull()
  })

  it('re-renders only the edited row', () => {
    renderEditor()
    const ibu = vi.mocked(calcHopIBU)
    ibu.mockClear()
    fireEvent.change(numbers(rows(1)[0])[1], { target: { value: '35' } })
    // HopRow computes its IBU on every render; the untouched hop row is memoized.
    expect(ibu).toHaveBeenCalledTimes(1)
  })

  it('is read-only for a role that cannot change production data', () => {
    authRole = 'viewer'
    renderEditor()
    for (const name of ['Add Fermentable', 'Add Hop', 'Add Yeast', 'Add Mash Step', 'Save']) {
      expect(screen.queryByRole('button', { name })).toBeNull()
    }
    expect(screen.getByLabelText('Name *')).toBeDisabled()
    expect(screen.getAllByRole('combobox', { name: 'Malt' })[0]).toBeDisabled()
    expect(screen.getAllByRole('spinbutton', { name: 'Hop amount' })[0]).toBeDisabled()
    expect(screen.getByRole('spinbutton', { name: 'Infusion volume (L)' })).toBeDisabled()
  })
})
