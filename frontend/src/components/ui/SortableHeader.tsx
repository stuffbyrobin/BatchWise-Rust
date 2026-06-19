// Reusable sortable table header. The backend sort spec is a column name with
// an optional leading `-` for descending (e.g. `name` asc, `-name` desc).

export type SortDir = 'asc' | 'desc'

export function parseSort(sort: string | undefined): { col: string; dir: SortDir } | null {
  if (!sort) return null
  return sort.startsWith('-') ? { col: sort.slice(1), dir: 'desc' } : { col: sort, dir: 'asc' }
}

/**
 * Given the current sort spec and a clicked column, return the next spec:
 * a new column sorts ascending; clicking the active column flips the direction.
 */
export function nextSort(current: string | undefined, col: string): string {
  const parsed = parseSort(current)
  if (parsed?.col === col) return parsed.dir === 'asc' ? `-${col}` : col
  return col
}

export function SortableHeader({
  column,
  label,
  sort,
  onSort,
  align = 'left',
  className = '',
}: {
  /** Backend sort key for this column (must be in the endpoint's allow-list). */
  column: string
  label: string
  /** Current sort spec (e.g. `-calc_og`). */
  sort: string | undefined
  /** Called with the next sort spec when the header is clicked. */
  onSort: (next: string) => void
  align?: 'left' | 'right'
  /** Padding/layout classes for the `<th>`; defaults to `px-4 py-3` to match
   *  most tables. Pass e.g. `p-3` to match a table with different spacing. */
  className?: string
}) {
  const parsed = parseSort(sort)
  const active = parsed?.col === column
  const arrow = active ? (parsed!.dir === 'asc' ? '↑' : '↓') : '↕'

  return (
    <th
      className={`${className || 'px-4 py-3'} text-${align} text-xs font-medium text-[var(--color-muted)] uppercase`}
      aria-sort={active ? (parsed!.dir === 'asc' ? 'ascending' : 'descending') : 'none'}
    >
      <button
        type="button"
        onClick={() => onSort(nextSort(sort, column))}
        className={`inline-flex items-center gap-1 uppercase tracking-inherit hover:text-[var(--color-fg)] transition-colors ${
          align === 'right' ? 'flex-row-reverse' : ''
        }`}
      >
        {label}
        <span className={`text-[10px] ${active ? 'text-[var(--color-fg)]' : 'opacity-40'}`}>{arrow}</span>
      </button>
    </th>
  )
}
