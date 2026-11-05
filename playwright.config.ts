import { defineConfig, devices } from '@playwright/test'

// Local runs use the system Chromium (REMMODS_E2E_CHROMIUM=/usr/bin/chromium); CI leaves it unset
// and uses the Chromium that `npx playwright install` put in place.
const executablePath = process.env.REMMODS_E2E_CHROMIUM
const port = 1431

export default defineConfig({
  testDir: 'e2e/specs',
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : [['list']],
  use: {
    ...devices['Desktop Chrome'],
    baseURL: `http://127.0.0.1:${port}`,
    viewport: { width: 1200, height: 760 },
    headless: true,
    trace: 'retain-on-failure',
    launchOptions: executablePath ? { executablePath } : {},
  },
  projects: [{ name: 'chromium' }],
  webServer: {
    command: `npx vite --mode e2e --host 127.0.0.1 --port ${port} --strictPort`,
    url: `http://127.0.0.1:${port}`,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
})
