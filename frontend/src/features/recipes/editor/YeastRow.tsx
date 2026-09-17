import { memo } from 'react'
import type { IngredientRows, WithUid } from './useIngredientRows'
import { editorInputCls as inputCls, YEAST_UNITS, type Yeast } from './model'
import { NumberCell } from './NumberCell'
import { OptionPicker, type PickerOptions } from './OptionPicker'

interface Props {
  row: WithUid<Yeast>
  custom: boolean
  options: PickerOptions
  onUpdate: IngredientRows<Yeast>['update']
  onPick: (uid: number, value: string) => void
  onRemove: (uid: number) => void
}

/** One editable yeast row. Re-renders only when its own props change. */
export const YeastRow = memo(function YeastRow({ row, custom, options, onUpdate, onPick, onRemove }: Props) {
  const { uid } = row
  return (
    <tr className="border-t border-[var(--color-border)]">
      <td className="px-3 py-2">
        <OptionPicker
          label="Yeast"
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
          label="Yeast amount"
          value={row.amount}
          onValueChange={(v) => onUpdate(uid, 'amount', v)}
          className={`${inputCls} w-20`}
        />
      </td>
      <td className="px-3 py-2">
        <select
          aria-label="Yeast unit"
          value={row.unit}
          onChange={(e) => onUpdate(uid, 'unit', e.target.value)}
          className={`${inputCls} w-24`}
        >
          {YEAST_UNITS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </td>
      <td className="px-3 py-2">
        <NumberCell
          label="Attenuation %"
          value={row.attenuation_pct}
          onValueChange={(v) => onUpdate(uid, 'attenuation_pct', v)}
          placeholder="Atten %"
          className={`${inputCls} w-20`}
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
