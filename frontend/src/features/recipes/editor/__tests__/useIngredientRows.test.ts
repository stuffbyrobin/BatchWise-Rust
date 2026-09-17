import { act, renderHook } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import {
  CUSTOM,
  NONE,
  nextStepOrder,
  useIngredientRows,
} from '../useIngredientRows'

type Row = {
  step_order: number
  name: string
  amount: number
  color_ebc?: number | null
  form?: string
}

const numeric = { step_order: 'zero', amount: 'zero', color_ebc: 'null' } as const

describe('useIngredientRows', () => {
  it('initial rows get distinct uids and values strips them', () => {
    const initial: Row[] = [
      { step_order: 1, name: 'A', amount: 10 },
      { step_order: 2, name: 'B', amount: 20 },
    ]
    const { result } = renderHook(() => useIngredientRows<Row>(numeric, initial))

    expect(result.current.rows.length).toBe(2)
    expect(result.current.rows[0].uid).toBeDefined()
    expect(result.current.rows[1].uid).toBeDefined()
    expect(result.current.rows[0].uid).not.toBe(result.current.rows[1].uid)
    expect(result.current.values).toEqual(initial)
    expect(result.current.values.every((v) => 'uid' in v)).toBe(false)
  })

  it('update coerces numeric strings', () => {
    const initial: Row[] = [{ step_order: 1, name: 'A', amount: 10, color_ebc: 5 }]
    const { result } = renderHook(() => useIngredientRows<Row>(numeric, initial))
    const uid = result.current.rows[0].uid

    act(() => result.current.update(uid, 'amount', '2.5'))
    expect(result.current.rows[0].amount).toBe(2.5)

    act(() => result.current.update(uid, 'amount', ''))
    expect(result.current.rows[0].amount).toBe(0)

    act(() => result.current.update(uid, 'color_ebc', ''))
    expect(result.current.rows[0].color_ebc).toBe(null)

    act(() => result.current.update(uid, 'color_ebc', '12'))
    expect(result.current.rows[0].color_ebc).toBe(12)

    act(() => result.current.update(uid, 'name', ''))
    expect(result.current.rows[0].name).toBe('')

    act(() => result.current.update(uid, 'amount', 3))
    expect(result.current.rows[0].amount).toBe(3)
  })

  it('update/patch/remove target rows by uid', () => {
    const initial: Row[] = [
      { step_order: 1, name: 'A', amount: 10 },
      { step_order: 2, name: 'B', amount: 20 },
      { step_order: 3, name: 'C', amount: 30 },
    ]
    const { result } = renderHook(() => useIngredientRows<Row>(numeric, initial))
    const uid1 = result.current.rows[0].uid
    const uid2 = result.current.rows[1].uid
    const uid3 = result.current.rows[2].uid

    act(() => result.current.remove(uid2))

    expect(result.current.rows.length).toBe(2)
    expect(result.current.rows[0].uid).toBe(uid1)
    expect(result.current.rows[1].uid).toBe(uid3)
    expect(result.current.rows[0].name).toBe('A')
    expect(result.current.rows[1].name).toBe('C')

    const lastUid = result.current.rows[1].uid
    act(() => result.current.patch(lastUid, { name: 'X', amount: 9 }))

    expect(result.current.rows[0].name).toBe('A')
    expect(result.current.rows[1].name).toBe('X')
    expect(result.current.rows[1].amount).toBe(9)
  })

  it('add appends a row with a new uid; nextStepOrder returns max+1', () => {
    const { result } = renderHook(() => useIngredientRows<Row>(numeric, []))

    expect(nextStepOrder([])).toBe(1)

    const rowsWithOrders: Row[] = [
      { step_order: 1, name: 'A', amount: 10 },
      { step_order: 4, name: 'B', amount: 20 },
    ]
    expect(nextStepOrder(rowsWithOrders)).toBe(5)

    const initialUid = result.current.rows[0]?.uid
    act(() => result.current.add({ step_order: 1, name: 'New', amount: 5 }))

    expect(result.current.rows.length).toBe(1)
    const newUid = result.current.rows[0].uid
    expect(newUid).toBeGreaterThan(initialUid ?? 0)
  })

  it('pick handles CUSTOM, NONE, resolved, and unknown keys', () => {
    const initial: Row[] = [{ step_order: 1, name: '', amount: 0, form: '' }]
    const { result } = renderHook(() => useIngredientRows<Row>(numeric, initial))
    const uid = result.current.rows[0].uid

    const handlers = {
      resolve: (key: string) => {
        if (key === 'hops') return { form: 'pellet', color_ebc: 10 }
        return undefined
      },
      none: { form: '', color_ebc: null },
      custom: { form: 'custom' },
    }

    // CUSTOM
    act(() => result.current.pick(uid, CUSTOM, handlers))
    expect(result.current.customRows).toContain(uid)
    expect(result.current.rows[0].form).toBe('custom')

    // Known key
    act(() => result.current.pick(uid, 'hops', handlers))
    expect(result.current.customRows).not.toContain(uid)
    expect(result.current.rows[0].form).toBe('pellet')
    expect(result.current.rows[0].color_ebc).toBe(10)

    // NONE
    act(() => result.current.pick(uid, NONE, handlers))
    expect(result.current.customRows).not.toContain(uid)
    expect(result.current.rows[0].form).toBe('')
    expect(result.current.rows[0].color_ebc).toBe(null)

    // Unknown key - should not change anything
    const before = result.current.rows
    act(() => result.current.pick(uid, 'unknown', handlers))
    expect(result.current.rows).toBe(before)
    expect(result.current.customRows).not.toContain(uid)
  })

  it('remove clears the custom flag of the removed row', () => {
    const initial: Row[] = [{ step_order: 1, name: 'A', amount: 10 }]
    const { result } = renderHook(() => useIngredientRows<Row>(numeric, initial))
    const uid = result.current.rows[0].uid

    act(() => result.current.setCustom(uid, true))
    expect(result.current.customRows).toContain(uid)

    act(() => result.current.remove(uid))
    expect(result.current.customRows).not.toContain(uid)
  })

  it('reset replaces rows with fresh uids and clears customRows', () => {
    const initial: Row[] = [{ step_order: 1, name: 'A', amount: 10 }]
    const { result } = renderHook(() => useIngredientRows<Row>(numeric, initial))
    const uid = result.current.rows[0].uid

    act(() => result.current.setCustom(uid, true))
    expect(result.current.customRows).toContain(uid)

    const newItems: Row[] = [{ step_order: 2, name: 'B', amount: 20 }]
    act(() => result.current.reset(newItems))

    expect(result.current.rows.length).toBe(1)
    expect(result.current.rows[0].uid).not.toBe(uid)
    expect(result.current.customRows.size).toBe(0)
  })

  it('callbacks are stable', () => {
    const { result } = renderHook(() => useIngredientRows<Row>(numeric, []))

    const updateFn = result.current.update
    const addFn = result.current.add
    const removeFn = result.current.remove
    const patchFn = result.current.patch
    const pickFn = result.current.pick
    const resetFn = result.current.reset
    const setCustomFn = result.current.setCustom

    act(() => result.current.add({ step_order: 1, name: 'A', amount: 10 }))

    expect(result.current.update).toBe(updateFn)
    expect(result.current.add).toBe(addFn)
    expect(result.current.remove).toBe(removeFn)
    expect(result.current.patch).toBe(patchFn)
    expect(result.current.pick).toBe(pickFn)
    expect(result.current.reset).toBe(resetFn)
    expect(result.current.setCustom).toBe(setCustomFn)
  })
})
