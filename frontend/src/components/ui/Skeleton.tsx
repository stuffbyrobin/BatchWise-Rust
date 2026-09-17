/**
 * Placeholder rows shown while a list or panel loads.
 *
 * @param rows number of placeholder bars
 * @param rowClassName height (and any extra classes) for each bar
 */
export function Skeleton({
  rows = 3,
  rowClassName = 'h-10',
  className = '',
}: {
  rows?: number
  rowClassName?: string
  className?: string
}) {
  return (
    <div className={`space-y-2 animate-pulse ${className}`} role="status" aria-label="Loading">
      {Array.from({ length: rows }, (_, i) => (
        <div
          key={i}
          className={`rounded ${rowClassName}`}
          style={{ background: 'var(--color-border)' }}
        />
      ))}
    </div>
  )
}
