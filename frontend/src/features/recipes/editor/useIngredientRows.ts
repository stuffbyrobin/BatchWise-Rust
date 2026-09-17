import { useState, useCallback, useMemo, useRef, useEffect } from 'react'

/** Select value that switches a picker row to free-text entry. */
export const CUSTOM = '__custom__'
/** Select value for "nothing picked". */
export const NONE = '__none__'

/** How an emptied numeric input is stored. */
export type EmptyAs = 'zero' | 'null'

/** A row plus its client-only identity (React key and per-row UI state). Never sent to the API. */
export type WithUid<T> = T & { uid: number }

let uidCounter = 1

/** Returns a new, never-reused row uid (module-level counter starting at 1). */
export function newRowUid(): number {
  return uidCounter++
}

/** Next step order after the highest in `rows`, or 1 when empty. */
export function nextStepOrder(rows: readonly { step_order?: number | null }[]): number {
  if (rows.length === 0) return 1
  let max = 0
  for (const row of rows) {
    const order = row.step_order ?? 0
    if (order > max) max = order
  }
  return max + 1
}

export interface PickHandlers<T> {
  /** Fields to apply for an option key, or undefined if the key is unknown. */
  resolve: (key: string) => Partial<T> | undefined
  /** Fields to apply when NONE is picked. */
  none: Partial<T>
  /** Fields to apply when CUSTOM is picked (optional). */
  custom?: Partial<T>
}

export interface IngredientRows<T> {
  rows: WithUid<T>[]
  /** Rows without the uid, for the API payload. */
  values: T[]
  /** uids of rows in free-text (custom) mode. */
  customRows: ReadonlySet<number>
  /** Replaces every row (hydration): fresh uids, custom flags cleared. */
  reset: (items: T[]) => void
  add: (item: T) => void
  remove: (uid: number) => void
  /** Sets one field. A string given for a numeric field is coerced: '' becomes 0 or null per `numericFields`, anything else Number(value). Other values are stored as given. */
  update: <K extends keyof T>(uid: number, field: K, value: T[K] | string) => void
  patch: (uid: number, fields: Partial<T>) => void
  setCustom: (uid: number, custom: boolean) => void
  /** Applies a picker <select> value: CUSTOM marks the row custom (and applies handlers.custom), NONE clears the flag and applies handlers.none, any other key applies handlers.resolve(key) and clears the flag; an unknown key changes nothing. */
  pick: (uid: number, value: string, handlers: PickHandlers<T>) => void
}

/**
 * Editable table rows keyed by a stable client-side uid, so memoized row
 * components and per-row UI state (such as custom-name mode) survive inserts
 * and removals. All returned callbacks are referentially stable.
 */
export function useIngredientRows<T extends object>(
  numericFields: Partial<Record<keyof T, EmptyAs>>,
  initial: T[] = [],
): IngredientRows<T> {
  const [rows, setRows] = useState<WithUid<T>[]>(() =>
    initial.map((r) => ({ ...r, uid: newRowUid() }))
  )
  const [customRows, setCustomRows] = useState<Set<number>>(() => new Set())

  const numericFieldsRef = useRef(numericFields)
  useEffect(() => {
    numericFieldsRef.current = numericFields
  }, [numericFields])

  const reset = useCallback(
    (items: T[]) => {
      setRows(items.map((r) => ({ ...r, uid: newRowUid() })))
      setCustomRows(new Set())
    },
    []
  )

  const add = useCallback(
    (item: T) => {
      setRows((prev) => [...prev, { ...item, uid: newRowUid() }])
    },
    []
  )

  const remove = useCallback(
    (uid: number) => {
      setRows((prev) => prev.filter((r) => r.uid !== uid))
      setCustomRows((prev) => {
        if (!prev.has(uid)) return prev
        const next = new Set(prev)
        next.delete(uid)
        return next
      })
    },
    []
  )

  const update = useCallback(
    <K extends keyof T>(uid: number, field: K, value: T[K] | string) => {
      setRows((prev) =>
        prev.map((r) => {
          if (r.uid !== uid) return r
          const numericConfig = numericFieldsRef.current[field]
          if (typeof value === 'string' && numericConfig) {
            const trimmed = value.trim()
            if (trimmed === '') {
              const v = numericConfig === 'null' ? null : 0
              return { ...r, [field]: v as T[K] }
            }
            const num = Number(trimmed)
            return { ...r, [field]: num as unknown as T[K] }
          }
          return { ...r, [field]: value as T[K] }
        })
      )
    },
    []
  )

  const patch = useCallback(
    (uid: number, fields: Partial<T>) => {
      setRows((prev) =>
        prev.map((r) => (r.uid !== uid ? r : { ...r, ...fields }))
      )
    },
    []
  )

  const setCustom = useCallback(
    (uid: number, custom: boolean) => {
      setCustomRows((prev) => {
        if (prev.has(uid) === custom) return prev
        const next = new Set(prev)
        if (custom) next.add(uid)
        else next.delete(uid)
        return next
      })
    },
    []
  )

  const pick = useCallback(
    (uid: number, value: string, handlers: PickHandlers<T>) => {
      if (value === CUSTOM) {
        if (handlers.custom) patch(uid, handlers.custom)
        setCustom(uid, true)
        return
      }
      const fields = value === NONE ? handlers.none : handlers.resolve(value)
      if (fields === undefined) return
      patch(uid, fields)
      setCustom(uid, false)
    },
    [patch, setCustom]
  )

  const values = useMemo(
    () => rows.map(({ uid: _uid, ...rest }) => rest as T),
    [rows]
  )

  return useMemo(
    () => ({
      rows,
      values,
      customRows,
      reset,
      add,
      remove,
      update,
      patch,
      setCustom,
      pick,
    }),
    [rows, customRows, values, reset, add, remove, update, patch, setCustom, pick]
  )
}
