import { useCallback, useEffect, useRef } from 'react'

/**
 * Opens rendered PDF blob URLs in a new tab. The previous URL is revoked when
 * it is replaced and the last one on unmount, so repeated previews do not leak
 * blobs. The new tab's opener link is cut; the window handle is kept only so a
 * print request can call print() once the PDF has loaded.
 */
export function usePdfWindow() {
  const urlRef = useRef<string | null>(null)

  useEffect(
    () => () => {
      if (urlRef.current) URL.revokeObjectURL(urlRef.current)
    },
    [],
  )

  return useCallback((url: string, print: boolean) => {
    if (urlRef.current && urlRef.current !== url) URL.revokeObjectURL(urlRef.current)
    urlRef.current = url
    const w = window.open(url, '_blank')
    if (!w) return
    w.opener = null
    if (print) w.addEventListener('load', () => w.print())
  }, [])
}
