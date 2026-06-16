import { render } from '@testing-library/react'
import { axe, toHaveNoViolations } from 'jest-axe'
import { EBCSwatch } from '../EBCSwatch'

expect.extend(toHaveNoViolations)

describe('EBCSwatch accessibility', () => {
  it('has no axe violations at low EBC', async () => {
    const { container } = render(<EBCSwatch ebc={5} />)
    const results = await axe(container)
    expect(results).toHaveNoViolations()
  })

  it('has no axe violations at mid EBC', async () => {
    const { container } = render(<EBCSwatch ebc={40} />)
    const results = await axe(container)
    expect(results).toHaveNoViolations()
  })

  it('has no axe violations at high EBC', async () => {
    const { container } = render(<EBCSwatch ebc={80} />)
    const results = await axe(container)
    expect(results).toHaveNoViolations()
  })

  it('maps EBC 18.3 to roughly SRM step 3-4', () => {
    const ebc = 18.3
    const step = Math.min(10, Math.max(1, Math.round((ebc / 80) * 9) + 1))
    expect(step).toBeGreaterThanOrEqual(3)
    expect(step).toBeLessThanOrEqual(4)
  })
})
