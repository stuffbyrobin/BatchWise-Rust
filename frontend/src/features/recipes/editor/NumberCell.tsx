import type { InputHTMLAttributes } from 'react'

type Props = Omit<InputHTMLAttributes<HTMLInputElement>, 'type' | 'value' | 'onChange'> & {
  /** Accessible name; table cells have no visible label of their own. */
  label: string
  value: number | string | null | undefined
  /** Receives the raw input string; the row hook coerces it. */
  onValueChange: (value: string) => void
}

/** Numeric table-cell input with an accessible name. */
export function NumberCell({ label, value, onValueChange, ...rest }: Props) {
  return (
    <input
      {...rest}
      type="number"
      aria-label={label}
      value={value ?? ''}
      onChange={(e) => onValueChange(e.target.value)}
    />
  )
}
