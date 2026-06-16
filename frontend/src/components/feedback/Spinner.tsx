export function Spinner({ size = 24 }: { size?: number }) {
  return (
    <div
      className="inline-block rounded-full border-2 border-[var(--color-border)] border-t-[var(--color-accent)] animate-spin"
      style={{ width: size, height: size }}
      aria-label="Loading"
    />
  )
}
