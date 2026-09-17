import { renderHook } from '@testing-library/react'
import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest'
import { usePdfWindow } from '../usePdfWindow'

describe('usePdfWindow', () => {
  beforeEach(() => {
    URL.revokeObjectURL = vi.fn()
    const win = { opener: {} as unknown, addEventListener: vi.fn(), print: vi.fn() }
    vi.spyOn(window, 'open').mockReturnValue(win as unknown as Window)
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('opens the PDF in a new tab and cuts the opener link', () => {
    const { result } = renderHook(() => usePdfWindow())
    result.current('blob:a', false)
    expect(window.open).toHaveBeenCalledWith('blob:a', '_blank')
    const win = vi.mocked(window.open).mock.results[0].value
    expect(win.opener).toBeNull()
    expect(win.addEventListener).not.toHaveBeenCalled()
  })

  it('prints once the PDF has loaded', () => {
    const { result } = renderHook(() => usePdfWindow())
    result.current('blob:a', true)
    const win = vi.mocked(window.open).mock.results[0].value
    expect(win.addEventListener).toHaveBeenCalledWith('load', expect.any(Function))
    win.addEventListener.mock.calls[0][1]()
    expect(win.print).toHaveBeenCalledTimes(1)
  })

  it('revokes the previous URL when a new PDF replaces it', () => {
    const { result } = renderHook(() => usePdfWindow())
    result.current('blob:a', false)
    expect(URL.revokeObjectURL).not.toHaveBeenCalled()
    result.current('blob:b', false)
    expect(URL.revokeObjectURL).toHaveBeenCalledTimes(1)
    expect(URL.revokeObjectURL).toHaveBeenCalledWith('blob:a')
  })

  it('revokes the last URL on unmount', () => {
    const { result, unmount } = renderHook(() => usePdfWindow())
    result.current('blob:a', false)
    unmount()
    expect(URL.revokeObjectURL).toHaveBeenCalledWith('blob:a')
  })

  it('does nothing more when the popup is blocked', () => {
    vi.mocked(window.open).mockReturnValue(null)
    const { result } = renderHook(() => usePdfWindow())
    expect(() => result.current('blob:a', true)).not.toThrow()
  })
})
