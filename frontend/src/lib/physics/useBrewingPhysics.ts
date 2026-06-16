// React hook for the brewing-physics WASM module.
//
// Usage:
//   const physics = useBrewingPhysics()
//   if (physics.ready) physics.calculateAbv(1.050, 1.010) // 5.25
//
// The module loads once (shared across all hook consumers). Before it resolves,
// `ready` is false and the calculation methods are no-ops returning NaN, so
// callers can render a loading state without null checks on every call.

import { useEffect, useState } from 'react'
import { loadPhysics, type Physics } from './index'

export type UseBrewingPhysics =
  | ({ ready: true; error: null } & Physics)
  | ({ ready: false; error: Error | null } & Physics)

const NOT_READY = new Proxy({} as Physics, {
  get: () => () => NaN,
})

export function useBrewingPhysics(): UseBrewingPhysics {
  const [physics, setPhysics] = useState<Physics | null>(null)
  const [error, setError] = useState<Error | null>(null)

  useEffect(() => {
    let active = true
    loadPhysics()
      .then((p) => active && setPhysics(p))
      .catch((e) => active && setError(e instanceof Error ? e : new Error(String(e))))
    return () => {
      active = false
    }
  }, [])

  if (physics) {
    return { ready: true, error: null, ...physics }
  }
  return { ready: false, error, ...NOT_READY }
}
