import { describe, expect, it } from 'vitest'
import { fileTooLarge, formatBytes, MAX_LOGO_BYTES } from '../files'

describe('file size helpers', () => {
  it('formats sizes', () => {
    expect(formatBytes(512)).toBe('512 B')
    expect(formatBytes(2048)).toBe('2 KiB')
    expect(formatBytes(MAX_LOGO_BYTES)).toBe('2 MiB')
  })

  it('reports only files over the limit', () => {
    const small = new File(['x'.repeat(10)], 'logo.png')
    const big = new File([new Uint8Array(MAX_LOGO_BYTES + 1)], 'huge.png')
    expect(fileTooLarge(small, MAX_LOGO_BYTES)).toBeNull()
    expect(fileTooLarge(big, MAX_LOGO_BYTES)).toBe('huge.png is 2 MiB; the limit is 2 MiB.')
  })
})
