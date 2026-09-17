import { CUSTOM, NONE } from './useIngredientRows'

export interface PickerOption {
  key: string
  name: string
  label: string
}

export interface PickerOptions {
  byName: Map<string, PickerOption>
  groups: { label: string; options: PickerOption[] }[]
  loading: boolean
}

interface Props {
  /** Accessible name of the select, e.g. "Malt"; the custom-name input is "<label> name". */
  label: string
  options: PickerOptions
  name: string
  /** The row was explicitly switched to custom mode. */
  custom: boolean
  className: string
  onPick: (value: string) => void
  onNameChange: (name: string) => void
}

/**
 * Stock/library picker with a "Custom / Other…" free-text fallback. A saved
 * name that matches no option shows as custom once the options have loaded.
 */
export function OptionPicker({ label, options, name, custom, className, onPick, onNameChange }: Props) {
  const matched = options.byName.get(name)
  // While options load byName is empty; don't infer custom mode from that or
  // every saved row flashes as "Custom / Other" on first paint.
  const isCustom = custom || (!options.loading && name !== '' && !matched)
  const value = isCustom ? CUSTOM : matched ? matched.key : NONE
  return (
    <div className="flex flex-col gap-1">
      <select aria-label={label} value={value} onChange={(e) => onPick(e.target.value)} className={className}>
        <option value={NONE}>Select {label.toLowerCase()}…</option>
        <option value={CUSTOM}>Custom / Other…</option>
        {options.groups.map((g) => (
          <optgroup key={g.label} label={g.label}>
            {g.options.map((o) => (
              <option key={o.key} value={o.key}>
                {o.label}
              </option>
            ))}
          </optgroup>
        ))}
      </select>
      {isCustom && (
        <input
          type="text"
          aria-label={`${label} name`}
          value={name}
          onChange={(e) => onNameChange(e.target.value)}
          placeholder={`${label} name`}
          className={className}
        />
      )}
    </div>
  )
}
