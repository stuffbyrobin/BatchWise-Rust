import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { Badge, type BadgeTone } from '../Badge'

const TONE_TOKENS: Record<BadgeTone, { bg: string; text: string }> = {
  neutral: { bg: '--color-neutral-bg', text: '--color-muted' },
  info: { bg: '--color-info-bg', text: '--color-accent' },
  positive: { bg: '--color-success-bg', text: '--color-success' },
  warning: { bg: '--color-warning-bg', text: '--color-warning' },
  negative: { bg: '--color-danger-bg', text: '--color-danger' },
}

describe('Badge', () => {
  it.each(Object.entries(TONE_TOKENS))('renders the %s tone with its own tokens', (tone, { bg, text }) => {
    render(<Badge tone={tone as BadgeTone}>Label</Badge>)
    const badge = screen.getByText('Label')
    expect(badge.className).toContain(`var(${bg})`)
    expect(badge.className).toContain(`var(${text})`)
  })

  it('is a rounded pill, not the app-wide default radius', () => {
    render(<Badge tone="neutral">Label</Badge>)
    expect(screen.getByText('Label').className).toContain('rounded-full')
  })

  it('accepts extra classes without losing the tone', () => {
    render(
      <Badge tone="positive" className="ml-2">
        Label
      </Badge>,
    )
    const badge = screen.getByText('Label')
    expect(badge.className).toContain('ml-2')
    expect(badge.className).toContain('var(--color-success)')
  })
})
