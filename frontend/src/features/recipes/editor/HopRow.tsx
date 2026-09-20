import { memo } from 'react'
import { calcHopIBU, type IBUMethod } from '../../../utils/ibu'
import type { IngredientRows, WithUid } from './useIngredientRows'
import { editorInputCls as inputCls, HOP_FORMS, HOP_UNITS, HOP_USES, type Hop } from './model'
import { NumberCell } from './NumberCell'
import { OptionPicker, type PickerOptions } from './OptionPicker'

interface Props {
  row: WithUid<Hop>
  custom: boolean
  options: PickerOptions
  /** Volume used for the IBU estimate. */
  batchSizeLiters: number
  ibuMethod: IBUMethod
  og: number
  onUpdate: IngredientRows<Hop>['update']
  onPick: (uid: number, value: string) => void
  onRemove: (uid: number) => void
}

/** One editable hop row with its IBU estimate. Re-renders only when its own props change. */
export const HopRow = memo(function HopRow({ row, custom, options, ibuMethod, batchSizeLiters, og, onUpdate, onPick, onRemove }: Props) {
  const { uid } = row
  const ibu = calcHopIBU(ibuMethod, row.unit === 'kg' ? row.amount * 1000 : row.amount, row.alpha_acid_pct, row.boil_time_minutes, batchSizeLiters, og)
  return (
    <tr className="border-t border-[var(--color-border)]">
      <td className="px-3 py-2">
        <NumberCell
          label="Hop order"
          value={row.step_order}
          onValueChange={(v) => onUpdate(uid, 'step_order', v)}
          className={`${inputCls} w-16`}
        />
      </td>
      <td className="px-3 py-2">
        <OptionPicker
          label="Hop"
          options={options}
          name={row.name}
          custom={custom}
          className={`${inputCls} w-44`}
          onPick={(v) => onPick(uid, v)}
          onNameChange={(v) => onUpdate(uid, 'name', v)}
        />
      </td>
      <td className="px-3 py-2">
        <NumberCell
          label="Hop amount"
          value={row.amount}
          onValueChange={(v) => onUpdate(uid, 'amount', v)}
          className={`${inputCls} w-20`}
        />
      </td>
      <td className="px-3 py-2">
        <select
          aria-label="Hop unit"
          value={row.unit}
          onChange={(e) => onUpdate(uid, 'unit', e.target.value)}
          className={`${inputCls} w-20`}
        >
          {HOP_UNITS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </td>
      <td className="px-3 py-2">
        <NumberCell
          label="Alpha acid %"
          value={row.alpha_acid_pct}
          onValueChange={(v) => onUpdate(uid, 'alpha_acid_pct', v)}
          placeholder="AA%"
          className={`${inputCls} w-20`}
        />
      </td>
      <td className="px-3 py-2">
        <NumberCell
          label="Boil time (min)"
          value={row.boil_time_minutes}
          onValueChange={(v) => onUpdate(uid, 'boil_time_minutes', v)}
          placeholder="Minutes"
          className={`${inputCls} w-20`}
        />
      </td>
      <td className="px-3 py-2">
        <select
          aria-label="Hop form"
          value={row.form ?? ''}
          onChange={(e) => onUpdate(uid, 'form', e.target.value || undefined)}
          className={`${inputCls} w-24`}
        >
          <option value="">- Form -</option>
          {HOP_FORMS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </td>
      <td className="px-3 py-2">
        <select
          aria-label="Hop use"
          value={row.use ?? ''}
          onChange={(e) => onUpdate(uid, 'use', e.target.value || undefined)}
          className={`${inputCls} w-24`}
        >
          <option value="">- Use -</option>
          {HOP_USES.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </td>
      <td className="px-3 py-2 text-sm text-[var(--color-muted)] tabular-nums text-right">
        {ibu > 0 ? ibu.toFixed(1) : '—'}
      </td>
      <td className="px-3 py-2">
        <button type="button" onClick={() => onRemove(uid)} className="text-[var(--color-danger)] hover:underline">
          Remove
        </button>
      </td>
    </tr>
  )
})
