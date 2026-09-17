import { memo } from 'react'
import type { IngredientRows, WithUid } from './useIngredientRows'
import { editorInputCls as inputCls, FERMENTABLE_UNITS, type Fermentable } from './model'
import { NumberCell } from './NumberCell'
import { OptionPicker, type PickerOptions } from './OptionPicker'

interface Props {
  row: WithUid<Fermentable>
  custom: boolean
  options: PickerOptions
  onUpdate: IngredientRows<Fermentable>['update']
  onPick: (uid: number, value: string) => void
  onRemove: (uid: number) => void
}

/** One editable fermentable row. Re-renders only when its own props change. */
export const FermentableRow = memo(function FermentableRow({ row, custom, options, onUpdate, onPick, onRemove }: Props) {
  const { uid } = row
  return (
    <tr className="border-t border-[var(--color-border)]">
      <td className="px-3 py-2">
        <NumberCell
          label="Fermentable order"
          value={row.step_order}
          onValueChange={(v) => onUpdate(uid, 'step_order', v)}
          className={`${inputCls} w-16`}
        />
      </td>
      <td className="px-3 py-2">
        <OptionPicker
          label="Malt"
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
          label="Fermentable amount"
          value={row.amount}
          onValueChange={(v) => onUpdate(uid, 'amount', v)}
          className={`${inputCls} w-20`}
        />
      </td>
      <td className="px-3 py-2">
        <select
          aria-label="Fermentable unit"
          value={row.unit}
          onChange={(e) => onUpdate(uid, 'unit', e.target.value)}
          className={`${inputCls} w-20`}
        >
          {FERMENTABLE_UNITS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </td>
      <td className="px-3 py-2">
        <NumberCell
          label="Colour (EBC)"
          value={row.color_ebc}
          onValueChange={(v) => onUpdate(uid, 'color_ebc', v)}
          placeholder="EBC"
          className={`${inputCls} w-20`}
        />
      </td>
      <td className="px-3 py-2">
        <NumberCell
          label="Potential (PPG)"
          value={row.potential_ppg}
          onValueChange={(v) => onUpdate(uid, 'potential_ppg', v)}
          placeholder="PPG"
          className={`${inputCls} w-20`}
        />
      </td>
      <td className="px-3 py-2">
        <input
          type="text"
          aria-label="Fermentable type"
          value={row.type ?? ''}
          onChange={(e) => onUpdate(uid, 'type', e.target.value)}
          placeholder="Type"
          className={`${inputCls} w-24`}
        />
      </td>
      <td className="px-3 py-2">
        <input
          type="text"
          aria-label="Addition"
          value={row.addition ?? ''}
          onChange={(e) => onUpdate(uid, 'addition', e.target.value)}
          placeholder="Addition"
          className={`${inputCls} w-24`}
        />
      </td>
      <td className="px-3 py-2">
        <button type="button" onClick={() => onRemove(uid)} className="text-red-600 hover:underline">
          Remove
        </button>
      </td>
    </tr>
  )
})
