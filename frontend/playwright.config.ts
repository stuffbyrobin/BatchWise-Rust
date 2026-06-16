import { defineConfig, devices } from '@playwright/test'

const E2E_BACKEND_PORT = '8082'

export default defineConfig({
  testDir: './tests/e2e',
  timeout: 120_000,
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI
    ? [['github'], ['html', { open: 'never' }]]
    : [['list'], ['html', { open: 'never' }]],
  use: {
    baseURL: 'http://localhost:5174',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  globalSetup: './tests/e2e/helpers/setup.ts',
  globalTeardown: './tests/e2e/helpers/teardown.ts',
  webServer: {
    command: 'pnpm run dev -- --port 5174',
    url: 'http://localhost:5174',
    reuseExistingServer: true,
    timeout: 120_000,
    env: {
      VITE_TEST_MODE: 'true',
      E2E_BACKEND_PORT,
    },
  },
})
