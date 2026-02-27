/**
 * Cross-browser e2e tests for the anonymous voting workflow.
 *
 * These tests exercise the full stack in real browser engines (Chromium,
 * Firefox, WebKit) plus mobile viewports, covering behaviour that the
 * Node.js Vitest e2e tests cannot reach:
 *
 *   • Actual browser cookie jars (HttpOnly / SameSite enforcement)
 *   • Crypto in the browser's own JS engine (V8, SpiderMonkey, JavaScriptCore)
 *   • Mobile viewport rendering and touch interaction
 *   • CORS preflight handling (browsers enforce this; Node.js fetch does not)
 *
 * Prerequisites — both must be running before `pnpm test:e2e`:
 *   Rust backend:  cargo run --bin rustsystem-server && cargo run --bin rustsystem-trustauth
 *   Vite server:   started automatically by playwright.config.ts webServer option
 *
 * Each test creates its own meeting through the UI so tests are fully
 * independent and can run in parallel.
 */

import { test, expect, type Page } from "@playwright/test";

// ─── Shared helpers ───────────────────────────────────────────────────────────

/**
 * Navigate to the dev page and wait until it is interactive.
 * All four step-cards are rendered unconditionally, so we just wait for the
 * Create Meeting button to be visible.
 */
async function goto(page: Page) {
  await page.goto("/dev/signature-dev");
  await page.getByTestId("btn-create-meeting").waitFor({ state: "visible" });
}

/**
 * Click "Create Meeting" and wait for the server to respond.
 * Returns when the success alert ("Meeting ready") becomes visible.
 */
async function createMeeting(page: Page) {
  await page.getByTestId("btn-create-meeting").click();
  await expect(
    page.getByRole("alert").filter({ hasText: "Meeting ready" }),
  ).toBeVisible();
}

/**
 * Click "Start Vote" and wait for the "Vote round active" confirmation.
 */
async function startVote(page: Page) {
  await page.getByTestId("btn-start-vote").click();
  await expect(
    page.getByRole("alert").filter({ hasText: "Vote round active" }),
  ).toBeVisible();
}

/**
 * Click "Register" and wait for the "Blind signature received" confirmation.
 * The BBS+ commitment generation and the round-trip to the server take a
 * moment, especially in Firefox and WebKit — the 10 s expect timeout in the
 * config covers this.
 */
async function register(page: Page) {
  await page.getByTestId("btn-register").click();
  await expect(
    page.getByRole("alert").filter({ hasText: "Blind signature received" }),
  ).toBeVisible();
}

/**
 * Fill the choice input, click "Submit Vote", and wait for the acceptance
 * confirmation.
 *
 * @param choice Comma-separated indices, e.g. "0" or "0,1". Pass "" for blank.
 */
async function submitVote(page: Page, choice: string) {
  await page.getByTestId("input-choice").fill(choice);
  await page.getByTestId("btn-submit").click();
  await expect(
    page.getByRole("alert").filter({ hasText: "Vote accepted" }),
  ).toBeVisible();
}

// ─── Full vote cycle ──────────────────────────────────────────────────────────

test.describe("full vote cycle", () => {
  test("candidate vote (choice index 0)", async ({ page }) => {
    // The canonical happy path: host creates a meeting, starts a round, the
    // voter registers and submits a candidate vote.
    await goto(page);
    await createMeeting(page);
    await startVote(page);
    await register(page);
    await submitVote(page, "0");
  });

  test("blank vote (empty choice)", async ({ page }) => {
    // Voters may abstain by leaving the choice field empty. The server records
    // this separately in the blank counter.
    await goto(page);
    await createMeeting(page);
    await startVote(page);
    await register(page);
    await submitVote(page, ""); // empty → blank vote
  });
});

// ─── Cookie handling ──────────────────────────────────────────────────────────

test.describe("cookie handling", () => {
  test("session cookie is set after createMeeting and used in subsequent requests", async ({
    page,
    context,
  }) => {
    // The server sets an HttpOnly JWT cookie on the create-meeting response.
    // The browser must attach it automatically on every subsequent API call.
    // This test verifies the cookie jar is populated by checking that Start Vote
    // (which requires an authenticated host session) succeeds after creating a
    // meeting — if the cookie were missing, Start Vote would return 401.
    await goto(page);
    await createMeeting(page);

    // Confirm the cookie is present in the browser's cookie jar.
    const cookies = await context.cookies();
    const sessionCookie = cookies.find((c) => c.name === "access_token");
    expect(sessionCookie).toBeDefined();

    // HttpOnly flag ensures JS cannot read the cookie, but the browser still
    // sends it. Verify the end-to-end flow works (Start Vote succeeds).
    await startVote(page);
  });

  test("session persists across a same-origin navigation", async ({
    page,
    context,
  }) => {
    // Navigating to a different route and back must not lose the session cookie
    // (SameSite=Lax / Strict should allow same-site navigations).
    await goto(page);
    await createMeeting(page);

    // Navigate away (to the root) and come back.
    // waitUntil:"networkidle" ensures TanStack Router fully settles before we
    // fire the next goto — without it, WebKit can raise "navigation interrupted
    // by another navigation" because the SPA's history manipulation is still
    // in flight when the second goto begins.
    await page.goto("/", { waitUntil: "networkidle" });
    await page.goto("/dev/signature-dev");
    await page.getByTestId("btn-create-meeting").waitFor({ state: "visible" });

    // Cookie must still be present after the round-trip navigation.
    const cookies = await context.cookies();
    expect(cookies.find((c) => c.name === "access_token")).toBeDefined();
  });
});

// ─── UI state machine ─────────────────────────────────────────────────────────

test.describe("UI state machine", () => {
  test("Start Vote and Register buttons are disabled until their prerequisites are met", async ({
    page,
  }) => {
    // The UI gates each step on the previous one completing successfully.
    // This prevents confusing API errors from out-of-order actions.
    await goto(page);

    // Before creating a meeting, Start Vote must be disabled.
    await expect(page.getByTestId("btn-start-vote")).toBeDisabled();

    // After creating a meeting, Start Vote becomes enabled.
    await createMeeting(page);
    await expect(page.getByTestId("btn-start-vote")).toBeEnabled();

    // Register must be disabled until a vote round is active.
    await expect(page.getByTestId("btn-register")).toBeDisabled();

    await startVote(page);
    await expect(page.getByTestId("btn-register")).toBeEnabled();
  });

  test("Submit Vote button is disabled until registration is complete", async ({
    page,
  }) => {
    await goto(page);
    await expect(page.getByTestId("btn-submit")).toBeDisabled();

    await createMeeting(page);
    await startVote(page);
    await expect(page.getByTestId("btn-submit")).toBeDisabled();

    await register(page);
    await expect(page.getByTestId("btn-submit")).toBeEnabled();
  });

  test("End Vote Round resets the vote-round state", async ({ page }) => {
    // After ending a round the UI should no longer show "Vote round active"
    // and Register should be disabled again.
    await goto(page);
    await createMeeting(page);
    await startVote(page);

    await page.getByTestId("btn-end-round").click();
    await expect(
      page.getByRole("alert").filter({ hasText: "Round ended" }),
    ).toBeVisible();

    // Register must be disabled again — no active round.
    await expect(page.getByTestId("btn-register")).toBeDisabled();
  });
});

// ─── Mobile viewport smoke tests ─────────────────────────────────────────────

test.describe("mobile viewport", () => {
  test("full vote cycle completes on a mobile viewport", async ({ page }) => {
    // Ensures buttons are tappable and the layout does not break on small
    // screens. The Pixel 7 / iPhone 14 device profiles set a narrow viewport
    // (~390–412 px) and a mobile user-agent string.
    //
    // This is a smoke test; the behaviour is identical to the desktop cycle.
    await goto(page);
    await createMeeting(page);
    await startVote(page);
    await register(page);
    await submitVote(page, "0");
  });
});
