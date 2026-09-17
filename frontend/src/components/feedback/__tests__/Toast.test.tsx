import { render } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { ToastProvider, useToast } from '../Toast'

describe('Toast', () => {
  it('keeps the context value stable across provider re-renders', () => {
    const seen: unknown[] = []
    function Probe() {
      seen.push(useToast())
      return null
    }
    const { rerender } = render(
      <ToastProvider>
        <Probe />
      </ToastProvider>,
    )
    rerender(
      <ToastProvider>
        <Probe />
      </ToastProvider>,
    )
    expect(seen.length).toBeGreaterThanOrEqual(2)
    expect(seen[seen.length - 1]).toBe(seen[0])
  })
})
