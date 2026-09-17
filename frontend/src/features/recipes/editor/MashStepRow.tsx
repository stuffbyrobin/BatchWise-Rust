import { memo } from 'react'
import type { IngredientRows, WithUid } from './useIngredientRows'
import { editorInputCls as inputCls, MASH_STEP_TYPES, type MashStep } from './model'
import { NumberCell } from './NumberCell'

interface Props {
  row: WithUid<MashStep>
  onUpdate: IngredientRows<MashStep>['update']
  onRemove: (uid: number) => void
}

/** One editable mash step row. Re-renders only when its own props change. */
export const MashStepRow = memo(function MashStepRow({ row, onUpdate, onRemove }: Props) {
  const { uid } = row
  return (
    <tr className="border-t border-[var(--color-border)]">
      <td className="px-3 py-2">
        <NumberCell
          label="Mash step order"
          value={row.step_order}
          onValueChange={(v) => onUpdate(uid, 'step_order', v)}
          className={`${inputCls} w-16`}
        />
      </td>
      <td className="px-3 py-2">
        <select
          aria-label="Mash step type"
          value={row.step_type}
          onChange={(e) => onUpdate(uid, 'step_type', e.target.value)}
          className={`${inputCls} w-24`}
        >
          {MASH_STEP_TYPES.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </td>
      <td className="px-3 py-2">
        <NumberCell
          label="Target temp (C)"
          value={row.target_temp_c}
          onValueChange={(v) => onUpdate(uid, 'target_temp_c', v)}
          placeholder="Temp C"
          className={`${inputCls} w-20`}
        />
      </td>
      <td className="px-3 py-2">
        <NumberCell
          label="Hold (min)"
          value={row.hold_minutes}
          onValueChange={(v) => onUpdate(uid, 'hold_minutes', v)}
          placeholder="Minutes"
          className={`${inputCls} w-20`}
        />
      </td>
      <td className="px-3 py-2">
        <NumberCell
          label="Infusion volume (L)"
          value={row.infusion_volume_liters}
          onValueChange={(v) => onUpdate(uid, 'infusion_volume_liters', v)}
          placeholder="Volume L"
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
