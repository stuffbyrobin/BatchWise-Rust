import { test, expect } from '@playwright/test'

const RUN_ID = Date.now()
const OWNER_EMAIL = `owner${RUN_ID}@example.com`
const INVITEE_EMAIL = `invitee${RUN_ID}@example.com`
const PASSWORD = 'Brewers1234!'
const TENANT = `Invite Brewery ${RUN_ID}`

test('an owner invites a viewer who joins through the link', async ({ page, browser }) => {
  // ── 1. The owner registers and creates an invite link ──────────────────
  await page.goto('/register')
  await page.getByLabel('Email').fill(OWNER_EMAIL)
  await page.getByLabel('Password').fill(PASSWORD)
  await page.getByLabel('Your Name').fill('E2E Owner')
  await page.getByLabel('Brewery Name').fill(TENANT)
  await page.getByRole('button', { name: 'Create account' }).click()
  await expect(page).toHaveURL('/app', { timeout: 15_000 })

  await page.getByRole('link', { name: 'Members' }).first().click()
  await expect(page).toHaveURL('/members', { timeout: 15_000 })
  await page.getByLabel('Email', { exact: true }).fill(INVITEE_EMAIL)
  // The wrapping label's text includes the options, so match the accessible name.
  await page.getByRole('combobox', { name: 'Role', exact: true }).selectOption('viewer')
  await page.getByRole('button', { name: 'Create invite link' }).click()

  const linkField = page.getByLabel('Invite link')
  await expect(linkField).toHaveValue(/\/invite#.+/, { timeout: 10_000 })
  const link = await linkField.inputValue()
  const openInvitations = page.getByRole('region', { name: 'Open invitations' })
  await expect(openInvitations.getByRole('cell', { name: INVITEE_EMAIL })).toBeVisible()

  // ── 2. The invitee opens the link in a fresh session and joins ─────────
  const inviteeContext = await browser.newContext()
  const invitee = await inviteeContext.newPage()
  await invitee.goto(link)
  await expect(invitee.getByText(TENANT)).toBeVisible({ timeout: 10_000 })
  await expect(invitee.getByText(INVITEE_EMAIL)).toBeVisible()

  await invitee.getByLabel('Your Name').fill('E2E Viewer')
  await invitee.getByLabel('Password', { exact: true }).fill(PASSWORD)
  await invitee.getByLabel('Confirm password').fill(PASSWORD)
  await invitee.getByRole('button', { name: 'Create account and join' }).click()
  await expect(invitee).toHaveURL('/app', { timeout: 15_000 })
  await expect(invitee.getByRole('heading', { name: /dashboard/i })).toBeVisible({ timeout: 10_000 })

  // A Viewer does not manage members.
  await expect(invitee.getByRole('link', { name: 'Members' })).toHaveCount(0)

  // ── 3. The link works only once ────────────────────────────────────────
  const again = await browser.newContext()
  const reuse = await again.newPage()
  await reuse.goto(link)
  await expect(reuse.getByRole('alert')).toContainText('This invitation is not valid.', { timeout: 10_000 })

  // ── 4. The owner sees the new member and no open invitation ────────────
  await page.reload()
  await expect(page.getByRole('cell', { name: 'E2E Viewer' })).toBeVisible({ timeout: 10_000 })
  await expect(page.getByRole('region', { name: 'Open invitations' })).toHaveCount(0)

  await inviteeContext.close()
  await again.close()
})
