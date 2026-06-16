const HIGH_PRIORITY = new Set(['gluten', 'milk', 'nuts', 'peanuts', 'soya', 'sesame', 'lupin'])

interface Props {
  allergens: string[]
}

export function AllergenBadges({ allergens }: Props) {
  if (!allergens.length) return null
  return (
    <div className="flex flex-wrap gap-1">
      {allergens.map((a) => (
        <span
          key={a}
          className="inline-block px-2 py-0.5 rounded text-xs font-medium"
          style={
            HIGH_PRIORITY.has(a)
              ? { background: 'var(--color-danger)', color: '#fff' }
              : { background: 'var(--color-border)', color: 'var(--color-fg)' }
          }
        >
          {a}
        </span>
      ))}
    </div>
  )
}
