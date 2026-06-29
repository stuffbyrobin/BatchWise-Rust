import { SALT_FORMS, normalizeForm } from './mineralForms'

// Renders the anhydrous/hydrate(/liquid) selector for a salt that has multiple
// supplied forms, plus a %w/w strength input when the liquid form is chosen.
// Returns null for salts with no form choice.
export function MineralFormControls({
  type,
  form,
  strength,
  onForm,
  onStrength,
}: {
  type: string
  form?: string
  strength?: string
  onForm: (value: string) => void
  onStrength: (value: string) => void
}) {
  const cfg = SALT_FORMS[type]
  if (!cfg) return null
  const current = normalizeForm(form, type)
  const selectCls =
    'p-2 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)]'
  return (
    <>
      <select value={current} onChange={(e) => onForm(e.target.value)} title="Salt form" className={selectCls}>
        <option value="anhydrous">Anhydrous</option>
        <option value="hydrate">{cfg.hydrateLabel}</option>
        {cfg.liquid && <option value="liquid">Liquid</option>}
      </select>
      {current === 'liquid' && (
        <div className="flex items-center gap-1">
          <input
            type="number"
            min="0"
            max="100"
            step="1"
            placeholder="%w/w"
            value={strength ?? ''}
            onChange={(e) => onStrength(e.target.value)}
            className="w-16 p-2 rounded border text-sm bg-[var(--color-bg)] border-[var(--color-border)]"
          />
          <span className="text-xs text-[var(--color-muted)]">%w/w</span>
        </div>
      )}
    </>
  )
}
