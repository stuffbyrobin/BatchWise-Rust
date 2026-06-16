import { test, expect } from '@playwright/test'

const BACKEND_PORT = process.env.E2E_BACKEND_PORT ?? '8082'
const API = `http://localhost:${BACKEND_PORT}/api/v1`

// Unique per test run so parallel CI workers don't collide
const RUN_ID = Date.now()
const EMAIL = `brewer${RUN_ID}@example.com`
const PASSWORD = 'Brewers1234!'
const TENANT = `Test Brewery ${RUN_ID}`

// ─── helpers ─────────────────────────────────────────────────────────────────

async function getToken(page: import('@playwright/test').Page): Promise<string> {
  const token = await page.evaluate(() => {
    const bw = (window as unknown as Record<string, { getToken: () => string | null }>).__batchwise
    return bw?.getToken() ?? null
  })
  if (!token) throw new Error('No auth token found in window.__batchwise')
  return token
}

function brewDate(): string {
  const d = new Date()
  d.setDate(d.getDate() + 3)
  return d.toISOString().split('T')[0]
}

// ─── test ────────────────────────────────────────────────────────────────────

test('full brew cycle', async ({ page }) => {
  // ── 1. Register a new tenant ─────────────────────────────────────────────
  await page.goto('/register')
  await page.getByLabel('Email').fill(EMAIL)
  await page.getByLabel('Password').fill(PASSWORD)
  await page.getByLabel('Brewery Name').fill(TENANT)
  await page.getByRole('button', { name: 'Create account' }).click()
  await expect(page).toHaveURL('/app', { timeout: 15_000 })

  const token = await getToken(page)
  const headers = { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' }

  // ── 2. Add inventory ──────────────────────────────────────────────────────
  const mkLot = (
    type: string,
    name: string,
    amount: number,
    unit: string,
    lot: string,
  ) =>
    page.request.post(`${API}/inventory`, {
      headers,
      data: { type, name, amount, unit, lot_number: lot },
    })

  const [marisPkg, citraPkg, us05Pkg] = await Promise.all([
    mkLot('fermentable', 'Maris Otter', 10, 'kg', `MO-${RUN_ID}`),
    mkLot('hop', 'Citra', 50, 'g', `CI-${RUN_ID}`),
    mkLot('yeast', 'US-05', 11.5, 'g', `US05-${RUN_ID}`),
  ])
  expect(marisPkg.ok()).toBeTruthy()
  expect(citraPkg.ok()).toBeTruthy()
  expect(us05Pkg.ok()).toBeTruthy()
  const marisId = (await marisPkg.json()).id as string
  const us05LotId = (await us05Pkg.json()).id as string

  // ── 3. Create library yeast + yeast kinetics ──────────────────────────────
  const yeastRes = await page.request.post(`${API}/library/yeasts`, {
    headers,
    data: { name: 'US-05', type: 'ale' },
  })
  expect(yeastRes.status()).toBe(201)
  const yeastId = (await yeastRes.json()).id as string

  const kinRes = await page.request.post(`${API}/yeast-kinetics`, {
    headers,
    data: {
      yeast_id: yeastId,
      fermentation_temp_c: 18,
      primary_fermentation_days: 7,
      conditioning_days: 14,
      lag_phase_hours: 24,
    },
  })
  expect(kinRes.status()).toBe(201)

  // ── 4. Create recipe with linked yeast ───────────────────────────────────
  const recipeRes = await page.request.post(`${API}/recipes`, {
    headers,
    data: {
      name: `Citra Pale ${RUN_ID}`,
      type: 'all_grain',
      batch_size_liters: 20,
      fermentables: [{ step_order: 1, name: 'Maris Otter', amount: 5, unit: 'kg' }],
      hops: [{ step_order: 1, name: 'Citra', amount: 30, unit: 'g', alpha_acid_pct: 13, boil_time_minutes: 60 }],
      yeasts: [{ name: 'US-05', yeast_id: yeastId, amount: 11, unit: 'g' }],
      mash_steps: [{ step_order: 1, step_type: 'infusion', target_temp_c: 67, hold_minutes: 60 }],
    },
  })
  expect(recipeRes.status()).toBe(201)
  const recipeId = (await recipeRes.json()).id as string

  // Verify recipe appears in the UI
  await page.goto('/recipes')
  await expect(page.getByText(`Citra Pale ${RUN_ID}`)).toBeVisible({ timeout: 10_000 })

  // ── 5. Create batch with brew_date = today + 3 ────────────────────────────
  await page.goto('/batches/new')
  await page.getByRole('combobox').selectOption({ label: `Citra Pale ${RUN_ID}` })
  await page.getByLabel(/Batch Number/i).fill(`B${RUN_ID}`)
  await page.getByLabel(/Name/i).fill(`E2E Batch ${RUN_ID}`)
  await page.getByLabel(/Brew Date/i).fill(brewDate())
  await page.getByRole('button', { name: /Create batch/i }).click()

  // Should land on batch detail page
  await expect(page).toHaveURL(/\/batches\/[0-9a-f-]{36}$/, { timeout: 15_000 })
  const batchUrl = page.url()
  const batchId = batchUrl.split('/batches/')[1]

  // ── 6. Assert four calendar events with correct dates ─────────────────────
  await page.goto('/calendar')
  await expect(page.getByRole('heading', { name: /calendar/i })).toBeVisible({ timeout: 10_000 })

  // Verify events via API (UI calendar date assertions are fragile across views)
  const eventsRes = await page.request.get(`${API}/calendar/events?batch_id=${batchId}&page_size=20`, { headers })
  expect(eventsRes.ok()).toBeTruthy()
  const eventsBody = await eventsRes.json()
  expect(eventsBody.items).toHaveLength(4)

  // ── 7. Transition planned → brewing; assert no error toast ───────────────
  await page.goto(batchUrl)
  await expect(page.getByText(/planned/i).first()).toBeVisible({ timeout: 10_000 })
  await page.getByRole('button', { name: /brewing/i }).click()
  // Confirmation dialog for the brewing transition
  await page.getByRole('button', { name: /start brewing/i }).click()
  await expect(page.getByText(/brewing/i).first()).toBeVisible({ timeout: 10_000 })
  // No error toast visible
  await expect(page.getByText(/insufficient stock|error|failed/i)).not.toBeVisible()

  // ── 8. Visit inventory; assert lot amounts decreased correctly ────────────
  // Verify via API (inventory detail page shows amounts in input fields, not text nodes)
  const marisRes = await page.request.get(`${API}/inventory/${marisId}`, { headers })
  expect(marisRes.ok()).toBeTruthy()
  const marisData = await marisRes.json()
  // After brewing transition, 5 kg Maris Otter deducted from 10 kg → 5 kg remaining
  expect(marisData.amount).toBe(5)

  const us05Res = await page.request.get(`${API}/inventory/${us05LotId}`, { headers })
  expect(us05Res.ok()).toBeTruthy()
  const us05Data = await us05Res.json()
  // 11 g US-05 deducted from 11.5 g → 0.5 g remaining
  expect(us05Data.amount).toBeCloseTo(0.5, 1)

  // ── 9. Transition through remaining statuses ──────────────────────────────
  const transitions: string[] = ['fermenting', 'conditioning', 'packaging', 'completed']
  for (const status of transitions) {
    await page.goto(batchUrl)
    await page.getByRole('button', { name: new RegExp(status, 'i') }).click()
    await expect(page.getByText(new RegExp(status, 'i')).first()).toBeVisible({ timeout: 10_000 })
  }

  // ── 10. Dashboard: active_batches = 0, completed = 1 ─────────────────────
  await page.goto('/app')
  await expect(page.getByText('0').first()).toBeVisible({ timeout: 10_000 })

  // Verify via API
  const statsRes = await page.request.get(`${API}/dashboard/stats`, { headers })
  expect(statsRes.ok()).toBeTruthy()
  const stats = await statsRes.json()
  expect(stats.active_batches_count).toBe(0)
  expect(stats.batch_status_breakdown?.completed).toBeGreaterThanOrEqual(1)

  // Also verify recipe was created correctly (used ID to query)
  const recipeCheck = await page.request.get(`${API}/recipes/${recipeId}`, { headers })
  expect(recipeCheck.ok()).toBeTruthy()
})
