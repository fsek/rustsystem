import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright cross-browser e2e configuration.
 *
 * Prerequisites:
 *   1. Rust backend (must be started manually):
 *        API_ENDPOINT=http://localhost:3000 cargo run --bin rustsystem-server
 *   2. Vite dev server — started automatically by the webServer option below.
 *   3. Browser system dependencies are handled automatically inside the Docker
 *      container (Ubuntu Noble). To run via Docker (recommended on non-Ubuntu):
 *        pnpm test:e2e:docker
 *      On Ubuntu/Debian natively: sudo pnpm exec playwright install-deps
 *
 * Run all browsers:          pnpm test:e2e
 * Run a single browser:      pnpm test:e2e --project=chromium
 * Run with UI (debug):       pnpm test:e2e --ui
 * Run a single file:         pnpm test:e2e e2e/voting.spec.ts
 */
export default defineConfig({
  testDir: "./e2e",

  // Give each test a generous timeout — the BBS+ crypto and sequential API
  // calls add up, especially in Firefox and WebKit.
  timeout: 30_000,
  expect: { timeout: 10_000 },

  // Keep tests independent: each creates its own meeting via the UI.
  // The Rust server stores meetings in a HashMap, so concurrent meetings are
  // isolated by their UUIDs and can run in parallel safely.
  fullyParallel: true,
  workers: process.env["CI"] ? 2 : undefined,

  reporter: [["list"], ["html", { open: "never" }]],

  use: {
    baseURL: "http://localhost:5173",

    // Capture traces and screenshots on failure for debugging.
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    video: "on-first-retry",
  },

  projects: [
    // ── Desktop browsers ──────────────────────────────────────────────────
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
    {
      name: "firefox",
      use: { ...devices["Desktop Firefox"] },
    },
    {
      name: "webkit",
      use: { ...devices["Desktop Safari"] },
    },

    // ── Mobile viewports (still using desktop browser engines under the hood,
    //    but with mobile UA strings, touch events, and smaller viewports) ──
    {
      name: "mobile-chrome",
      use: { ...devices["Pixel 7"] },
    },
    {
      name: "mobile-safari",
      use: { ...devices["iPhone 14"] },
    },
  ],

  // Start the Vite dev server automatically.
  // The Rust backend must be started separately (see Prerequisites above).
  webServer: {
    command: "pnpm dev",
    url: "http://localhost:5173",
    reuseExistingServer: !process.env["CI"],
    timeout: 30_000,
  },
});
