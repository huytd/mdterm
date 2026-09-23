import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './visual',
  timeout: 30000,
  expect: {
    toHaveScreenshot: {
      maxDiffPixelRatio: 0.01,
      animations: 'disabled',
    },
  },
  use: {
    baseURL: 'http://127.0.0.1:3123',
    trace: 'on-first-retry',
  },
  webServer: {
    command: 'node visual/server.mjs',
    port: 3123,
    reuseExistingServer: !process.env.CI,
    env: {
      PORT: '3123',
    },
  },
  projects: [
    {
      name: 'webkit',
      use: {
        ...devices['Desktop Safari'],
        launchOptions: {
          env: {
            ...process.env,
            PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS: 'true',
          },
        },
      },
    },
  ],
});
