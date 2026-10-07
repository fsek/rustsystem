/**
 * End-to-end tests in real browser engines, against running services:
 *
 *   rustsystem-trustauth, and rustsystem-server serving the built frontend (`pnpm build`).
 *   See TESTING.md for the exact commands.
 *
 * WebKit can't run natively on every Linux distribution. Set PW_WEBKIT_WS to the endpoint of a
 * Playwright server running in the official Docker image and WebKit runs there instead.
 */
import { defineConfig, devices } from "@playwright/test";

const webkitWs = process.env.PW_WEBKIT_WS;

export default defineConfig({
  testDir: "e2e",
  timeout: 90_000,
  fullyParallel: true,
  reporter: "list",
  use: {
    baseURL: process.env.E2E_BASE_URL ?? "http://localhost:1443",
    trace: "retain-on-failure",
  },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
    { name: "firefox", use: { ...devices["Desktop Firefox"] } },
    {
      name: "webkit",
      use: {
        ...devices["Desktop Safari"],
        ...(webkitWs ? { connectOptions: { wsEndpoint: webkitWs } } : {}),
      },
    },
  ],
});
