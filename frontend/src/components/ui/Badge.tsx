import type { ReactNode } from 'react'

/**
 * The app's small, closed set of badge tones. `info` and `neutral` have no
 * dedicated brand colour of their own: `info` reuses the accent (the landing
 * page has no separate "blue"), `neutral` reuses the muted grey already used
 * for borders and secondary text.
 */
export type BadgeTone = 'neutral' | 'info' | 'positive' | 'warning' | 'negative'

const TONE_CLASSES: Record<BadgeTone, string> = {
  neutral: 'bg-[var(--color-neutral-bg)] text-[var(--color-muted)]',
  info: 'bg-[var(--color-info-bg)] text-[var(--color-accent)]',
  positive: 'bg-[var(--color-success-bg)] text-[var(--color-success)]',
  warning: 'bg-[var(--color-warning-bg)] text-[var(--color-warning)]',
  negative: 'bg-[var(--color-danger-bg)] text-[var(--color-danger)]',
}

/**
 * A small status/category pill in one of the app's five tones. Replaces the
 * five separate status-badge implementations that used to exist across
 * Purchase Orders, Label Records, Compliance Audit, Duty Returns and
 * Container Assets, each with its own colours and shape.
 */
export function Badge({ tone, children, className = '' }: { tone: BadgeTone; children: ReactNode; className?: string }) {
  return (
    <span className={`inline-block px-2 py-0.5 rounded-full text-xs font-medium whitespace-nowrap ${TONE_CLASSES[tone]} ${className}`}>
      {children}
    </span>
  )
}
