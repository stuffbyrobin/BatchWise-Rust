// Demo of the brewing-physics WASM hook: enter OG/FG and see ABV, attenuation,
// calories and per-100 ml energy computed client-side by the same Rust code the
// server runs. Drop <PhysicsDemo /> into any route to try it.

import { useState } from 'react'
import { useBrewingPhysics } from './useBrewingPhysics'

export function PhysicsDemo() {
  const physics = useBrewingPhysics()
  const [og, setOg] = useState(1.05)
  const [fg, setFg] = useState(1.01)

  if (!physics.ready) {
    return <p>{physics.error ? `Physics failed to load: ${physics.error.message}` : 'Loading physics…'}</p>
  }

  let abv: number | string = '—'
  let attenuation: number | string = '—'
  let calories: number | string = '—'
  try {
    abv = physics.calculateAbv(og, fg).toFixed(2)
    attenuation = physics.calculateAttenuation(og, fg).toFixed(1)
    calories = physics.calculateCalories(og, fg).toFixed(0)
  } catch (e) {
    // calculateAbv throws (via the Rust Result) on invalid gravities.
    abv = attenuation = calories = (e as Error).message
  }

  const energyKcal = physics.energyKcalPer100ml(Number(abv) || 0).toFixed(0)

  return (
    <div style={{ display: 'grid', gap: 8, maxWidth: 320 }}>
      <h3>Brewing physics (WASM)</h3>
      <label>
        OG{' '}
        <input
          type="number"
          step="0.001"
          value={og}
          onChange={(e) => setOg(parseFloat(e.target.value))}
        />
      </label>
      <label>
        FG{' '}
        <input
          type="number"
          step="0.001"
          value={fg}
          onChange={(e) => setFg(parseFloat(e.target.value))}
        />
      </label>
      <ul>
        <li>ABV: {abv}%</li>
        <li>Attenuation: {attenuation}%</li>
        <li>Calories (12 oz): {calories}</li>
        <li>Energy: {energyKcal} kcal / 100 ml</li>
      </ul>
    </div>
  )
}
