// Interface tests (§18.5). Playwright ships its own Chromium and WebKit
// builds for macOS, Windows, and Linux, so this suite runs identically on
// every platform — no desktop-automation driver, nothing to be blocked by a
// runner's environment. WebKit is the engine family behind the macOS webview,
// so it is the closest available check on how the app renders there.

import { defineConfig, devices } from '@playwright/test';

const PORT = 4173;

export default defineConfig({
  testDir: './ui-tests',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [['github'], ['list']] : [['list']],
  use: {
    baseURL: `http://localhost:${String(PORT)}`,
    trace: 'on-first-retry',
  },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
  ],
  // Serves the real production bundle — the same asset the app ships.
  webServer: {
    command: `pnpm vite preview --port ${String(PORT)} --strictPort`,
    port: PORT,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
