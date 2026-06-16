import { test, expect } from '@playwright/test'

const RUN_ID = Date.now()
const EMAIL = `refresh${RUN_ID}@example.com`
const PASSWORD = 'Brewers1234!'
const TENANT = `Refresh Brewery ${RUN_ID}`

test('silent token refresh on 401', async ({ page }) => {
  // Register and land on dashboard
  await page.goto('/register')
  await page.getByLabel('Email').fill(EMAIL)
  await page.getByLabel('Password').fill(PASSWORD)
  await page.getByLabel('Brewery Name').fill(TENANT)
  await page.getByRole('button', { name: 'Create account' }).click()
  await expect(page).toHaveURL('/app', { timeout: 15_000 })

  // Confirm dashboard loaded
  await expect(page.getByRole('heading', { name: /dashboard/i })).toBeVisible({ timeout: 10_000 })

  // Verify VITE_TEST_MODE is active so window.__batchwise is available
  const bwDefined = await page.evaluate(
    () => typeof (window as unknown as Record<string, unknown>).__batchwise !== 'undefined',
  )
  expect(bwDefined).toBe(true)

  // Corrupt the access token — keep the refresh token intact in Zustand
  await page.evaluate(() => {
    const bw = (window as unknown as Record<string, { setToken: (t: string) => void }>).__batchwise
    bw.setToken('deliberately-bad-token-to-trigger-refresh')
  })

  // Navigate within the SPA (no full-page reload) so Zustand token state is preserved
  await page.getByRole('link', { name: 'Inventory' }).first().click()

  // Should land on inventory — NOT redirected to /login (silent refresh succeeded)
  await expect(page).toHaveURL('/inventory', { timeout: 15_000 })

  // The page content should be visible
  await expect(page.getByRole('heading', { name: /inventory/i })).toBeVisible({ timeout: 10_000 })
})
